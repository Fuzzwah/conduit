import type { Session } from '../types';

type AgentType = Session['agent_type'];

export function agentDisplayName(agentType: AgentType, options?: { short?: boolean }): string {
  const short = options?.short ?? false;

  switch (agentType) {
    case 'claude':
      return short ? 'Claude' : 'Claude Code';
    case 'codex':
      return short ? 'Codex' : 'Codex CLI';
    case 'gemini':
      return short ? 'Gemini' : 'Gemini CLI';
    case 'deepseek-tui':
      return short ? 'DeepSeek' : 'DeepSeek TUI';
    case 'dirac':
      return short ? 'Dirac' : 'Dirac CLI';
    case 'opencode':
      return 'OpenCode';
    case 'pi':
      return 'Pi';
  }
}

export function agentAccentColor(agentType: AgentType): string {
  switch (agentType) {
    case 'claude':
      return 'bg-orange-400';
    case 'codex':
      return 'bg-green-400';
    case 'dirac':
      return 'bg-fuchsia-400';
    case 'opencode':
      return 'bg-teal-400';
    case 'gemini':
      return 'bg-blue-400';
    case 'deepseek-tui':
      return 'bg-cyan-400';
    case 'pi':
      return 'bg-purple-400';
  }
}
