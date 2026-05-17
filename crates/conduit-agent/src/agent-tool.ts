/**
 * Agent Tool — Pi Extension for Orchestration
 *
 * Registers an "Agent" tool that Pi can call to delegate sub-tasks
 * (exploration, review, adversarial review) to sub-sessions running
 * configured models. Spawns a separate `pi --mode json` process for
 * each sub-agent invocation, keeping context windows isolated.
 *
 * The tool reads the model from each conduit-* skill file's frontmatter
 * (written by Conduit at startup) and passes it via `--model`.
 *
 * Tool signature matches Claude's "Agent" tool so Conduit's delegation
 * badge detection works without modification.
 */

import { spawn } from "node:child_process";
import * as fs from "node:fs";
import * as os from "node:os";
import * as path from "node:path";
import type { ExtensionAPI } from "@mariozechner/pi-coding-agent";

import { Type } from "typebox";

// ---------------------------------------------------------------------------
// Skill file reading
// ---------------------------------------------------------------------------

/** Read the model field from a conduit-* skill's frontmatter. */
function readSkillModel(skillDir: string, skillName: string): string | null {
	const skillPath = path.join(skillDir, skillName, "SKILL.md");
	try {
		const raw = fs.readFileSync(skillPath, "utf-8");
	// Remove frontmatter delimiters and parse YAML key-value pairs
	const fmMatch = raw.match(/^---\n([\s\S]*?)\n---\n/);
	if (!fmMatch) return null;
	const yaml = fmMatch[1];
	for (const line of yaml.split("\n")) {
		const match = line.match(/^\s*model\s*:\s*(.+)\s*$/);
		if (match) return match[1].trim();
	}
		return null;
	} catch {
		return null;
	}
}

// ---------------------------------------------------------------------------
// Sub-agent process spawning
// ---------------------------------------------------------------------------

interface SubagentResult {
	exitCode: number;
	output: string;
	usage: { input: number; output: number; cost: number };
}

async function runSubagent(
	cwd: string,
	model: string | null,
	task: string | undefined,
	signal: AbortSignal | undefined,
): Promise<SubagentResult> {
	const args: string[] = ["--mode", "json", "-p", "--no-session"];
	if (model) {
		args.push("--model", model);
	}
	if (task && task.trim().length > 0) {
		args.push(task);
	}

	const result: SubagentResult = {
		exitCode: 0,
		output: "",
		usage: { input: 0, output: 0, cost: 0 },
	};

	const invocation = resolvePiInvocation(args);
	const proc = spawn(invocation.command, invocation.args, {
		cwd,
		shell: false,
		stdio: ["ignore", "pipe", "pipe"],
	});

	let buffer = "";

	const processLine = (line: string) => {
		if (!line.trim()) return;
		let event: any;
		try {
			event = JSON.parse(line);
		} catch {
			return;
		}

		if (event.type === "message_end" && event.message) {
			const msg = event.message;
			if (msg && msg.role === "assistant") {
				const parts = msg.content ?? [];
				for (const part of parts) {
					if (part.type === "text") {
						result.output += part.text;
					}
				}
				if (msg.usage) {
					result.usage.input += msg.usage.input || 0;
					result.usage.output += msg.usage.output || 0;
					result.usage.cost += msg.usage.cost?.total || 0;
				}
			}
		}
	};

	proc.stdout.on("data", (data: Buffer) => {
		buffer += data.toString();
		const lines = buffer.split("\n");
		buffer = lines.pop() || "";
		for (const line of lines) processLine(line);
	});

	const exitCode = await new Promise<number>((resolve) => {
		proc.on("close", (code) => {
			if (buffer.trim()) processLine(buffer);
			resolve(code ?? 0);
		});
		proc.on("error", () => resolve(1));

		if (signal) {
			if (signal.aborted) {
				proc.kill("SIGTERM");
			} else {
				signal.addEventListener(
					"abort",
					() => {
						proc.kill("SIGTERM");
						setTimeout(() => {
							if (!proc.killed) proc.kill("SIGKILL");
						}, 5000);
					},
					{ once: true },
				);
			}
		}
	});

	result.exitCode = exitCode;
	return result;
}

/** Resolve the pi binary invocation — mirrors Pi's subagent example. */
function resolvePiInvocation(args: string[]): { command: string; args: string[] } {
	const currentScript = process.argv[1];
	const isBunVirtualScript = currentScript?.startsWith("/$bunfs/root/");
	if (currentScript && !isBunVirtualScript && fs.existsSync(currentScript)) {
		return { command: process.execPath, args: [currentScript, ...args] };
	}

	const execName = path.basename(process.execPath).toLowerCase();
	const isGenericRuntime = /^(node|bun)(\.exe)?$/.test(execName);
	if (!isGenericRuntime) {
		return { command: process.execPath, args };
	}

	return { command: "pi", args };
}

// ---------------------------------------------------------------------------
// Tool registration
// ---------------------------------------------------------------------------

const ALLOWED_SUBAGENT_TYPES = [
	"conduit-explore",
	"conduit-review",
	"conduit-adversarial-review",
] as const;

/** Agent tool parameter schema */
const AgentToolParams = Type.Object({
	subagent_type: Type.Union(
		ALLOWED_SUBAGENT_TYPES.map((value) => Type.Literal(value)),
		{
			description:
				"Which sub-agent skill to invoke: conduit-explore, conduit-review, or conduit-adversarial-review",
		},
	),
	task: Type.Optional(
		Type.String({ description: "The task to delegate to the sub-agent" }),
	),
});

function isAllowedSubagentType(
	value: string,
): value is (typeof ALLOWED_SUBAGENT_TYPES)[number] {
	return (ALLOWED_SUBAGENT_TYPES as readonly string[]).includes(value);
}

export default function (pi: ExtensionAPI) {
	pi.registerTool({
		name: "Agent",
		label: "Agent",
		description: [
			"Delegate exploration, review, or adversarial review tasks to a sub-agent.",
			"Sub-agents run in isolated sessions with their own context window and model.",
			"Use conduit-explore for reading/summarizing files,",
			"conduit-review for reviewing diffs, and",
			"conduit-adversarial-review for rigorous security/correctness review.",
		].join(" "),
		parameters: AgentToolParams,

		async execute(_toolCallId, params, signal, _onUpdate, ctx) {
			const subagentType = params.subagent_type;
			if (!isAllowedSubagentType(subagentType)) {
				return {
					content: [
						{
							type: "text" as const,
							text: `Unknown sub-agent type: ${subagentType}`,
						},
					],
					isError: true,
				};
			}

			const task = typeof params.task === "string" ? params.task : undefined;
			const skillDir = path.join(os.homedir(), ".pi", "agent", "skills");

			// Read the model from the skill file's frontmatter (templated at startup
			// by Conduit with the user-configured model).
			const model = readSkillModel(skillDir, subagentType);

			try {
				const result = await runSubagent(ctx.cwd, model, task, signal);

				if (result.exitCode !== 0 && !result.output.trim()) {
					return {
						content: [
							{
								type: "text" as const,
								text: `Sub-agent "${subagentType}" failed (exit code ${result.exitCode}). No output produced.`,
							},
						],
						isError: true,
					};
				}

				return {
					content: [
						{
							type: "text" as const,
							text: result.output || "(no output)",
						},
					],
				};
			} catch (err) {
				const message = err instanceof Error ? err.message : String(err);
				return {
					content: [{ type: "text" as const, text: `Sub-agent error: ${message}` }],
					isError: true,
				};
			}
		},
	});
}
