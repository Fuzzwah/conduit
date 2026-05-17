use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::{Path, PathBuf};

use serde::Deserialize;

use conduit_types::AgentType;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ConduitCommand {
    Model,
    Reasoning,
    Providers,
    NewSession,
    Fork,
    Handoff,
    Btw,
    Status,
    Rewind,
    AdversarialReview,
}

impl ConduitCommand {
    pub const ALL: [ConduitCommand; 10] = [
        ConduitCommand::Model,
        ConduitCommand::Reasoning,
        ConduitCommand::Providers,
        ConduitCommand::NewSession,
        ConduitCommand::Fork,
        ConduitCommand::Handoff,
        ConduitCommand::Btw,
        ConduitCommand::Status,
        ConduitCommand::Rewind,
        ConduitCommand::AdversarialReview,
    ];

    pub fn label(self) -> &'static str {
        match self {
            ConduitCommand::Model => "/model",
            ConduitCommand::Reasoning => "/reasoning",
            ConduitCommand::Providers => "/providers",
            ConduitCommand::NewSession => "/new",
            ConduitCommand::Fork => "/fork",
            ConduitCommand::Handoff => "/handoff",
            ConduitCommand::Btw => "/btw",
            ConduitCommand::Status => "/status",
            ConduitCommand::Rewind => "/rewind",
            ConduitCommand::AdversarialReview => "/adversarial-review",
        }
    }

    pub fn name(self) -> &'static str {
        self.label().trim_start_matches('/')
    }

    pub fn description(self) -> &'static str {
        match self {
            ConduitCommand::Model => "Select model",
            ConduitCommand::Reasoning => "Set reasoning effort",
            ConduitCommand::Providers => "Select enabled Agent CLIs",
            ConduitCommand::NewSession => "Start a new session",
            ConduitCommand::Fork => "Fork current session",
            ConduitCommand::Handoff => "Handoff current session",
            ConduitCommand::Btw => "Queue a note without interrupting",
            ConduitCommand::Status => "Show current session status",
            ConduitCommand::Rewind => "Remove the last turn from the conversation (Claude only)",
            ConduitCommand::AdversarialReview => "Run adversarial code review on workspace changes",
        }
    }

    pub fn parse(token: &str) -> Option<Self> {
        Self::ALL
            .into_iter()
            .find(|command| command.label() == token)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ProviderArtifactKind {
    Skill,
    Command,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ProviderArtifactSource {
    Conduit,
    Codex,
    Claude,
    Gemini,
    Opencode,
    Pi,
}

impl ProviderArtifactSource {
    pub fn display_name(self) -> &'static str {
        match self {
            ProviderArtifactSource::Conduit => "Conduit",
            ProviderArtifactSource::Codex => "Codex",
            ProviderArtifactSource::Claude => "Claude",
            ProviderArtifactSource::Gemini => "Gemini",
            ProviderArtifactSource::Opencode => "OpenCode",
            ProviderArtifactSource::Pi => "Pi",
        }
    }

    pub fn provider_priority(self) -> usize {
        match self {
            ProviderArtifactSource::Conduit => 0,
            ProviderArtifactSource::Codex => 1,
            ProviderArtifactSource::Claude => 2,
            ProviderArtifactSource::Opencode => 3,
            ProviderArtifactSource::Gemini => 4,
            ProviderArtifactSource::Pi => 1,
        }
    }
}

pub use conduit_types::SkillReference;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProviderInvocation {
    PromptCommand {
        source: ProviderArtifactSource,
        name: String,
        description: String,
        content: String,
        path: PathBuf,
    },
    Skill {
        source: ProviderArtifactSource,
        name: String,
        description: String,
        path: PathBuf,
    },
}

impl ProviderInvocation {
    pub fn source(&self) -> ProviderArtifactSource {
        match self {
            ProviderInvocation::PromptCommand { source, .. }
            | ProviderInvocation::Skill { source, .. } => *source,
        }
    }

    pub fn name(&self) -> &str {
        match self {
            ProviderInvocation::PromptCommand { name, .. }
            | ProviderInvocation::Skill { name, .. } => name,
        }
    }

    pub fn description(&self) -> &str {
        match self {
            ProviderInvocation::PromptCommand { description, .. }
            | ProviderInvocation::Skill { description, .. } => description,
        }
    }

    pub fn path(&self) -> &Path {
        match self {
            ProviderInvocation::PromptCommand { path, .. }
            | ProviderInvocation::Skill { path, .. } => path,
        }
    }

    pub fn invocation_kind(&self) -> ProviderArtifactKind {
        match self {
            ProviderInvocation::PromptCommand { .. } => ProviderArtifactKind::Command,
            ProviderInvocation::Skill { .. } => ProviderArtifactKind::Skill,
        }
    }

    pub fn trigger_char(&self) -> char {
        match self {
            ProviderInvocation::PromptCommand { .. } => '/',
            ProviderInvocation::Skill { source, .. } => match source {
                ProviderArtifactSource::Codex => '$',
                _ => '/',
            },
        }
    }

    pub fn trigger_label(&self) -> String {
        format!("{}{}", self.trigger_char(), self.name())
    }

    pub fn source_badge(&self) -> String {
        let kind = match self.invocation_kind() {
            ProviderArtifactKind::Skill => "skill",
            ProviderArtifactKind::Command => "command",
        };
        format!("{} {}", self.source().display_name(), kind)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResolvedPrompt {
    pub agent_text: String,
    pub history_text: String,
    pub codex_skill: Option<SkillReference>,
    pub source_badge: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ResolveResult {
    ConduitCommand {
        command: ConduitCommand,
        args: String,
    },
    ProviderPrompt(ResolvedPrompt),
    ListRequest {
        trigger: char,
    },
    Passthrough {
        text: String,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MenuEntryKind {
    ConduitCommand(ConduitCommand),
    ProviderInvocation(ProviderInvocation),
    FilePath(String),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MenuEntry {
    pub label: String,
    pub description: String,
    pub source_badge: String,
    pub trigger: char,
    pub kind: MenuEntryKind,
}

pub struct CommandResolver;

impl CommandResolver {
    pub fn resolve(text: &str, working_dir: &Path, active_provider: AgentType) -> ResolveResult {
        let trimmed = text.trim();
        if trimmed.is_empty() {
            return ResolveResult::Passthrough {
                text: text.to_string(),
            };
        }

        let Some(parsed) = ParsedInvocation::parse(trimmed) else {
            return ResolveResult::Passthrough {
                text: text.to_string(),
            };
        };

        match parsed.trigger {
            '/' => {
                let conduit_token = format!("/{}", parsed.token);
                if let Some(command) = ConduitCommand::parse(conduit_token.as_str()) {
                    return ResolveResult::ConduitCommand {
                        command,
                        args: parsed.args,
                    };
                }
            }
            '$' if parsed.token.is_empty() && parsed.args.is_empty() => {
                return ResolveResult::ListRequest { trigger: '$' };
            }
            _ => {}
        }

        if parsed.token.is_empty() {
            return ResolveResult::ListRequest {
                trigger: parsed.trigger,
            };
        }

        let registry = DiscoveryRegistry::discover(working_dir);
        if let Some(invocation) = registry.resolve(parsed.trigger, parsed.token.as_str()) {
            return ResolveResult::ProviderPrompt(Self::render_invocation(
                invocation,
                parsed.args.trim(),
                active_provider,
                trimmed,
            ));
        }

        ResolveResult::Passthrough {
            text: text.to_string(),
        }
    }

    pub fn menu_entries(working_dir: &Path, active_provider: AgentType) -> Vec<MenuEntry> {
        let registry = DiscoveryRegistry::discover(working_dir);
        let mut entries = Vec::new();

        for command in ConduitCommand::ALL {
            entries.push(MenuEntry {
                label: command.label().to_string(),
                description: command.description().to_string(),
                source_badge: ProviderArtifactSource::Conduit.display_name().to_string(),
                trigger: '/',
                kind: MenuEntryKind::ConduitCommand(command),
            });
        }

        for invocation in registry.menu_entries(active_provider) {
            entries.push(MenuEntry {
                label: invocation.trigger_label(),
                description: invocation.description().to_string(),
                source_badge: invocation.source_badge(),
                trigger: invocation.trigger_char(),
                kind: MenuEntryKind::ProviderInvocation(invocation),
            });
        }

        entries.sort_by(|left, right| left.label.cmp(&right.label));
        entries
    }

    fn render_invocation(
        invocation: ProviderInvocation,
        args: &str,
        active_provider: AgentType,
        original_text: &str,
    ) -> ResolvedPrompt {
        match invocation.clone() {
            ProviderInvocation::PromptCommand {
                source, content, ..
            } => {
                let agent_text = if provider_matches_source(active_provider, source) {
                    original_text.to_string()
                } else {
                    render_prompt_command(&content, args)
                };
                ResolvedPrompt {
                    agent_text,
                    history_text: original_text.to_string(),
                    codex_skill: None,
                    source_badge: Some(invocation.source_badge()),
                }
            }
            ProviderInvocation::Skill {
                source, name, path, ..
            } => {
                render_skill_invocation(source, &name, &path, args, active_provider, original_text)
            }
        }
    }
}

fn provider_matches_source(active_provider: AgentType, source: ProviderArtifactSource) -> bool {
    matches!(
        (active_provider, source),
        (AgentType::Codex, ProviderArtifactSource::Codex)
            | (AgentType::Claude, ProviderArtifactSource::Claude)
            | (AgentType::Gemini, ProviderArtifactSource::Gemini)
            | (AgentType::Opencode, ProviderArtifactSource::Opencode)
            | (AgentType::Pi, ProviderArtifactSource::Pi)
    )
}

fn render_skill_invocation(
    source: ProviderArtifactSource,
    name: &str,
    path: &Path,
    args: &str,
    active_provider: AgentType,
    original_text: &str,
) -> ResolvedPrompt {
    let args_suffix = if args.is_empty() {
        String::new()
    } else {
        format!(" {}", args)
    };

    match active_provider {
        AgentType::Codex => ResolvedPrompt {
            agent_text: format!("${name}{args_suffix}"),
            history_text: original_text.to_string(),
            codex_skill: Some(SkillReference {
                name: name.to_string(),
                path: path.to_path_buf(),
            }),
            source_badge: Some(format!("{} skill", source.display_name())),
        },
        AgentType::Claude if source == ProviderArtifactSource::Claude => ResolvedPrompt {
            agent_text: format!("/{name}{args_suffix}"),
            history_text: original_text.to_string(),
            codex_skill: None,
            source_badge: Some(format!("{} skill", source.display_name())),
        },
        AgentType::Gemini
        | AgentType::DeepseekTui
        | AgentType::Dirac
        | AgentType::Opencode
        | AgentType::Copilot
        | AgentType::Pi
        | AgentType::Claude => {
            let task = if args.is_empty() {
                "Use it for the user's current request.".to_string()
            } else {
                format!("Apply it to this task: {args}")
            };
            ResolvedPrompt {
                agent_text: format!(
                    "Use the skill \"{name}\" from {} for this task.\n\n{}",
                    source.display_name(),
                    task
                ),
                history_text: original_text.to_string(),
                codex_skill: None,
                source_badge: Some(format!("{} skill", source.display_name())),
            }
        }
    }
}

fn render_prompt_command(content: &str, args: &str) -> String {
    let replacements = [
        "{{args}}",
        "{{ arguments }}",
        "{{arguments}}",
        "$ARGUMENTS",
        "$ARGS",
        "${args}",
    ];

    let mut rendered = content.to_string();
    let mut replaced = false;
    for placeholder in replacements {
        if rendered.contains(placeholder) {
            rendered = rendered.replace(placeholder, args);
            replaced = true;
        }
    }

    if !replaced && !args.is_empty() {
        rendered.push_str("\n\nArguments:\n");
        rendered.push_str(args);
    }

    rendered
}

#[derive(Debug, Clone)]
struct ParsedInvocation {
    trigger: char,
    token: String,
    args: String,
}

impl ParsedInvocation {
    fn parse(input: &str) -> Option<Self> {
        let mut chars = input.chars();
        let trigger = chars.next()?;
        if trigger != '/' && trigger != '$' {
            return None;
        }

        let rest = chars.as_str();
        let (token, args) = match rest.split_once(char::is_whitespace) {
            Some((token, args)) => (token.trim().to_string(), args.trim().to_string()),
            None => (rest.trim().to_string(), String::new()),
        };

        Some(Self {
            trigger,
            token,
            args,
        })
    }
}

#[derive(Debug, Default)]
struct DiscoveryRegistry {
    slash_entries: HashMap<String, Vec<ProviderInvocation>>,
    dollar_entries: HashMap<String, Vec<ProviderInvocation>>,
}

impl DiscoveryRegistry {
    fn discover(working_dir: &Path) -> Self {
        let mut registry = Self::default();
        let mut seen = HashSet::new();

        for base in candidate_roots(working_dir) {
            registry.scan_codex_skills(&base, &mut seen);
            registry.scan_claude(&base, &mut seen);
            registry.scan_gemini(&base, &mut seen);
            registry.scan_opencode(&base, &mut seen);
            registry.scan_pi(&base, &mut seen);
        }

        registry.inject_claude_builtins();
        registry
    }

    fn inject_claude_builtins(&mut self) {
        const BUILTINS: &[(&str, &str)] = &[
            ("compact", "Compact conversation context"),
            ("context", "Show context window usage"),
            ("cost", "Show session cost and token usage"),
            ("clear", "Clear conversation history"),
            ("doctor", "Check Claude Code installation health"),
            ("help", "Show Claude Code help"),
            ("init", "Initialize CLAUDE.md for this project"),
            ("memory", "Edit CLAUDE.md memory files"),
            ("review", "Review pending code changes"),
        ];

        for (name, description) in BUILTINS {
            if self.slash_entries.contains_key(*name) {
                continue;
            }
            self.insert(ProviderInvocation::PromptCommand {
                source: ProviderArtifactSource::Claude,
                name: name.to_string(),
                description: description.to_string(),
                content: format!("/{name}"),
                path: PathBuf::from(format!("<builtin>/{name}")),
            });
        }
    }

    fn resolve(&self, trigger: char, name: &str) -> Option<ProviderInvocation> {
        let map = match trigger {
            '$' => &self.dollar_entries,
            '/' => &self.slash_entries,
            _ => return None,
        };

        map.get(name)
            .and_then(|entries| entries.iter().min_by_key(|entry| sort_key(entry)).cloned())
    }

    fn menu_entries(&self, active_provider: AgentType) -> Vec<ProviderInvocation> {
        let mut entries = Vec::new();
        let mut seen = HashSet::new();

        for entry in self
            .slash_entries
            .values()
            .chain(self.dollar_entries.values())
            .filter_map(|entries| {
                entries
                    .iter()
                    .filter(|entry| provider_matches_source(active_provider, entry.source()))
                    .min_by_key(|entry| sort_key(entry))
            })
        {
            let key = format!("{}:{}", entry.trigger_char(), entry.name());
            if seen.insert(key) {
                entries.push(entry.clone());
            }
        }

        entries
    }

    fn insert(&mut self, invocation: ProviderInvocation) {
        let name = invocation.name().to_string();
        let target = match invocation.trigger_char() {
            '/' => &mut self.slash_entries,
            '$' => &mut self.dollar_entries,
            _ => return,
        };
        target.entry(name).or_default().push(invocation);
    }

    fn scan_codex_skills(
        &mut self,
        base: &Path,
        seen: &mut HashSet<(ProviderArtifactSource, PathBuf)>,
    ) {
        for path in skill_files(&base.join(".codex/skills")) {
            if !seen.insert((ProviderArtifactSource::Codex, path.clone())) {
                continue;
            }
            let name = skill_name_from_path(&path);
            self.insert(ProviderInvocation::Skill {
                source: ProviderArtifactSource::Codex,
                description: skill_description(&path),
                name,
                path,
            });
        }
    }

    fn scan_claude(&mut self, base: &Path, seen: &mut HashSet<(ProviderArtifactSource, PathBuf)>) {
        for path in skill_files(&base.join(".claude/skills")) {
            if !seen.insert((ProviderArtifactSource::Claude, path.clone())) {
                continue;
            }
            let name = skill_name_from_path(&path);
            self.insert(ProviderInvocation::Skill {
                source: ProviderArtifactSource::Claude,
                description: skill_description(&path),
                name,
                path,
            });
        }

        for path in markdown_command_files(&base.join(".claude/commands")) {
            if !seen.insert((ProviderArtifactSource::Claude, path.clone())) {
                continue;
            }
            if let Some((name, description, prompt)) =
                load_markdown_command(&path, &base.join(".claude/commands"))
            {
                self.insert(ProviderInvocation::PromptCommand {
                    source: ProviderArtifactSource::Claude,
                    name,
                    description,
                    content: prompt,
                    path,
                });
            }
        }
    }

    fn scan_gemini(&mut self, base: &Path, seen: &mut HashSet<(ProviderArtifactSource, PathBuf)>) {
        for path in skill_files(&base.join(".gemini/skills"))
            .into_iter()
            .chain(skill_files(&base.join(".agents/skills")))
        {
            if !seen.insert((ProviderArtifactSource::Gemini, path.clone())) {
                continue;
            }
            let name = skill_name_from_path(&path);
            self.insert(ProviderInvocation::Skill {
                source: ProviderArtifactSource::Gemini,
                description: skill_description(&path),
                name,
                path,
            });
        }

        for path in toml_command_files(&base.join(".gemini/commands")) {
            if !seen.insert((ProviderArtifactSource::Gemini, path.clone())) {
                continue;
            }
            if let Some((name, description, prompt)) =
                load_gemini_command(&path, &base.join(".gemini/commands"))
            {
                self.insert(ProviderInvocation::PromptCommand {
                    source: ProviderArtifactSource::Gemini,
                    name,
                    description,
                    content: prompt,
                    path,
                });
            }
        }
    }

    fn scan_pi(&mut self, base: &Path, seen: &mut HashSet<(ProviderArtifactSource, PathBuf)>) {
        for path in skill_files(&base.join(".pi/skills")) {
            if !seen.insert((ProviderArtifactSource::Pi, path.clone())) {
                continue;
            }
            let name = skill_name_from_path(&path);
            self.insert(ProviderInvocation::Skill {
                source: ProviderArtifactSource::Pi,
                description: skill_description(&path),
                name,
                path,
            });
        }

        for path in markdown_command_files(&base.join(".pi/prompts")) {
            if !seen.insert((ProviderArtifactSource::Pi, path.clone())) {
                continue;
            }
            if let Some((name, description, prompt)) =
                load_markdown_command(&path, &base.join(".pi/prompts"))
            {
                self.insert(ProviderInvocation::PromptCommand {
                    source: ProviderArtifactSource::Pi,
                    name,
                    description,
                    content: prompt,
                    path,
                });
            }
        }
    }

    fn scan_opencode(
        &mut self,
        base: &Path,
        seen: &mut HashSet<(ProviderArtifactSource, PathBuf)>,
    ) {
        for path in skill_files(&base.join(".opencode/skills"))
            .into_iter()
            .chain(skill_files(&base.join(".agents/skills")))
            .chain(skill_files(&base.join(".claude/skills")))
        {
            if !seen.insert((ProviderArtifactSource::Opencode, path.clone())) {
                continue;
            }
            let name = skill_name_from_path(&path);
            self.insert(ProviderInvocation::Skill {
                source: ProviderArtifactSource::Opencode,
                description: skill_description(&path),
                name,
                path,
            });
        }

        for root in [
            base.join(".opencode/commands"),
            base.join(".opencode/command"),
        ] {
            for path in markdown_command_files(&root) {
                if !seen.insert((ProviderArtifactSource::Opencode, path.clone())) {
                    continue;
                }
                if let Some((name, description, prompt)) = load_markdown_command(&path, &root) {
                    self.insert(ProviderInvocation::PromptCommand {
                        source: ProviderArtifactSource::Opencode,
                        name,
                        description,
                        content: prompt,
                        path,
                    });
                }
            }
        }
    }
}

fn sort_key(invocation: &ProviderInvocation) -> (usize, usize, &str) {
    (
        invocation.source().provider_priority(),
        match invocation.invocation_kind() {
            ProviderArtifactKind::Skill => 0,
            ProviderArtifactKind::Command => 1,
        },
        invocation.name(),
    )
}

fn candidate_roots(working_dir: &Path) -> Vec<PathBuf> {
    let mut roots = Vec::new();
    roots.push(working_dir.to_path_buf());
    if let Some(home) = dirs::home_dir() {
        roots.push(home);
    }
    roots
}

fn skill_files(root: &Path) -> Vec<PathBuf> {
    let mut results = Vec::new();
    visit_files(root, &mut |path| {
        if path.file_name().is_some_and(|name| name == "SKILL.md") {
            results.push(path.to_path_buf());
        }
    });
    results
}

fn markdown_command_files(root: &Path) -> Vec<PathBuf> {
    let mut results = Vec::new();
    visit_files(root, &mut |path| {
        let is_markdown = path
            .extension()
            .and_then(|ext| ext.to_str())
            .is_some_and(|ext| ext.eq_ignore_ascii_case("md"));
        if is_markdown {
            results.push(path.to_path_buf());
        }
    });
    results
}

fn toml_command_files(root: &Path) -> Vec<PathBuf> {
    let mut results = Vec::new();
    visit_files(root, &mut |path| {
        let is_toml = path
            .extension()
            .and_then(|ext| ext.to_str())
            .is_some_and(|ext| ext.eq_ignore_ascii_case("toml"));
        if is_toml {
            results.push(path.to_path_buf());
        }
    });
    results
}

fn visit_files(root: &Path, visit: &mut impl FnMut(&Path)) {
    let entries = match fs::read_dir(root) {
        Ok(entries) => entries,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return,
        Err(error) => {
            tracing::warn!(path = %root.display(), error = %error, "Failed to read directory");
            return;
        }
    };

    for entry in entries {
        let entry = match entry {
            Ok(entry) => entry,
            Err(error) => {
                tracing::warn!(path = %root.display(), error = %error, "Failed to read directory entry");
                continue;
            }
        };
        let path = entry.path();
        if path.is_dir() {
            visit_files(&path, visit);
        } else if path.is_file() {
            visit(&path);
        }
    }
}

fn skill_name_from_path(path: &Path) -> String {
    path.parent()
        .and_then(|parent| parent.file_name())
        .and_then(|name| name.to_str())
        .unwrap_or("skill")
        .to_string()
}

fn skill_description(path: &Path) -> String {
    match fs::read_to_string(path) {
        Ok(content) => content
            .lines()
            .map(str::trim)
            .find(|line| !line.is_empty() && !line.starts_with('#'))
            .map(|line| line.to_string())
            .unwrap_or_else(|| "Reusable skill".to_string()),
        Err(error) => {
            tracing::warn!(path = %path.display(), error = %error, "Failed to read file");
            "Reusable skill".to_string()
        }
    }
}

fn load_markdown_command(path: &Path, root: &Path) -> Option<(String, String, String)> {
    let content = match fs::read_to_string(path) {
        Ok(content) => content,
        Err(error) => {
            tracing::warn!(path = %path.display(), error = %error, "Failed to read file");
            return None;
        }
    };
    let (description, body) = split_frontmatter(path, &content);
    let name = command_name_from_path(path, root);
    let description = description.unwrap_or_else(|| "Prompt command".to_string());
    Some((name, description, body.trim().to_string()))
}

fn split_frontmatter(path: &Path, content: &str) -> (Option<String>, String) {
    let normalized = content.replace("\r\n", "\n");
    let trimmed = normalized.trim_start();
    if !trimmed.starts_with("---\n") {
        return (None, normalized);
    }

    let rest = &trimmed[4..];
    let Some((frontmatter, body)) = rest.split_once("\n---\n") else {
        tracing::warn!(path = %path.display(), "Malformed frontmatter");
        return (None, normalized);
    };

    #[derive(Deserialize)]
    struct Frontmatter {
        description: Option<String>,
    }

    // Try TOML frontmatter first (used by Claude commands, etc.)
    let description = match toml::from_str::<Frontmatter>(frontmatter) {
        Ok(frontmatter) if frontmatter.description.is_some() => {
            return (frontmatter.description, body.to_string())
        }
        Ok(_) => None,
        Err(_) => {
            // Fallback: try YAML frontmatter (used by Pi prompt templates)
            extract_yaml_description(frontmatter)
        }
    };
    (description, body.to_string())
}

/// Extract `description` from simple YAML frontmatter.
/// Supports `key: value` pairs (no nested structures, no quotes required).
fn extract_yaml_description(frontmatter: &str) -> Option<String> {
    for line in frontmatter.lines() {
        let line = line.trim();
        if let Some(value) = line.strip_prefix("description:") {
            let value = value.trim().trim_matches('"').to_string();
            if !value.is_empty() {
                return Some(value);
            }
        }
    }
    None
}

#[derive(Debug, Deserialize)]
struct GeminiCommandFile {
    description: Option<String>,
    prompt: String,
}

fn load_gemini_command(path: &Path, root: &Path) -> Option<(String, String, String)> {
    let content = match fs::read_to_string(path) {
        Ok(content) => content,
        Err(error) => {
            tracing::warn!(path = %path.display(), error = %error, "Failed to read file");
            return None;
        }
    };
    let parsed = match toml::from_str::<GeminiCommandFile>(&content) {
        Ok(parsed) => parsed,
        Err(error) => {
            tracing::warn!(path = %path.display(), error = %error, "Invalid TOML syntax");
            return None;
        }
    };
    Some((
        command_name_from_path(path, root),
        parsed
            .description
            .unwrap_or_else(|| "Prompt command".to_string()),
        parsed.prompt,
    ))
}

fn command_name_from_path(path: &Path, root: &Path) -> String {
    let relative = path.strip_prefix(root).unwrap_or(path);
    let mut segments = relative
        .iter()
        .map(|segment| segment.to_string_lossy().to_string())
        .collect::<Vec<_>>();
    if let Some(last) = segments.last_mut() {
        if let Some((stem, _)) = last.rsplit_once('.') {
            *last = stem.to_string();
        }
    }
    segments.join(":")
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn resolves_conduit_command() {
        let root = TempDir::new().unwrap();
        let result = CommandResolver::resolve("/model", root.path(), AgentType::Codex);
        assert!(matches!(
            result,
            ResolveResult::ConduitCommand {
                command: ConduitCommand::Model,
                ..
            }
        ));
    }

    #[test]
    fn parses_bare_dollar_as_list_request() {
        let root = TempDir::new().unwrap();
        let result = CommandResolver::resolve("$", root.path(), AgentType::Codex);
        assert_eq!(result, ResolveResult::ListRequest { trigger: '$' });
    }

    #[test]
    fn translates_foreign_command_to_prompt() {
        let root = TempDir::new().unwrap();
        let command_dir = root.path().join(".gemini/commands/review");
        fs::create_dir_all(&command_dir).unwrap();
        fs::write(
            command_dir.join("check.toml"),
            "description = \"Review code\"\nprompt = \"Inspect this carefully: {{args}}\"\n",
        )
        .unwrap();

        let result =
            CommandResolver::resolve("/review:check auth flow", root.path(), AgentType::Codex);
        let ResolveResult::ProviderPrompt(prompt) = result else {
            panic!("expected provider prompt");
        };
        assert_eq!(prompt.agent_text, "Inspect this carefully: auth flow");
    }

    #[test]
    fn prefers_codex_skill_over_other_provider_conflict() {
        let root = TempDir::new().unwrap();
        fs::create_dir_all(root.path().join(".codex/skills/ship")).unwrap();
        fs::create_dir_all(root.path().join(".claude/skills/ship")).unwrap();
        fs::write(
            root.path().join(".codex/skills/ship/SKILL.md"),
            "# ship\nCodex ship skill\n",
        )
        .unwrap();
        fs::write(
            root.path().join(".claude/skills/ship/SKILL.md"),
            "# ship\nClaude ship skill\n",
        )
        .unwrap();

        let result = CommandResolver::resolve("$ship", root.path(), AgentType::Codex);
        let ResolveResult::ProviderPrompt(prompt) = result else {
            panic!("expected provider prompt");
        };
        assert_eq!(prompt.agent_text, "$ship");
        assert_eq!(prompt.codex_skill.unwrap().name, "ship");
    }

    #[test]
    fn command_name_uses_namespaces() {
        let root = TempDir::new().unwrap();
        let command_dir = root.path().join(".opencode/commands/release");
        fs::create_dir_all(&command_dir).unwrap();
        fs::write(command_dir.join("notes.md"), "Write release notes").unwrap();

        let entries = CommandResolver::menu_entries(root.path(), AgentType::Opencode);
        assert!(entries.iter().any(|entry| entry.label == "/release:notes"));
    }

    #[test]
    fn preserves_opencode_aliases_for_shared_skill_paths() {
        let root = TempDir::new().unwrap();
        let skill_dir = root.path().join(".agents/skills/ship");
        fs::create_dir_all(&skill_dir).unwrap();
        fs::write(skill_dir.join("SKILL.md"), "# ship\nShared ship skill\n").unwrap();

        let result = CommandResolver::resolve("/ship", root.path(), AgentType::Opencode);
        let ResolveResult::ProviderPrompt(prompt) = result else {
            panic!("expected provider prompt");
        };
        assert!(prompt
            .agent_text
            .contains("Use the skill \"ship\" from OpenCode"));
    }

    #[test]
    fn split_frontmatter_accepts_crlf_delimiters() {
        let root = TempDir::new().unwrap();
        let command_dir = root.path().join(".claude/commands");
        fs::create_dir_all(&command_dir).unwrap();
        fs::write(
            command_dir.join("ship.md"),
            "---\r\ndescription = \"Ship it\"\r\n---\r\nDo the release\r\n",
        )
        .unwrap();

        let entries = CommandResolver::menu_entries(root.path(), AgentType::Claude);
        let entry = entries
            .into_iter()
            .find(|entry| entry.label == "/ship")
            .expect("expected command entry");
        assert_eq!(entry.description, "Ship it");
    }

    #[test]
    fn menu_entries_only_include_active_provider_invocations() {
        let root = TempDir::new().unwrap();
        fs::create_dir_all(root.path().join(".claude/skills/ship")).unwrap();
        fs::create_dir_all(root.path().join(".gemini/skills/review")).unwrap();
        fs::write(
            root.path().join(".claude/skills/ship/SKILL.md"),
            "# ship\nClaude ship skill\n",
        )
        .unwrap();
        fs::write(
            root.path().join(".gemini/skills/review/SKILL.md"),
            "# review\nGemini review skill\n",
        )
        .unwrap();

        let entries = CommandResolver::menu_entries(root.path(), AgentType::Claude);

        assert!(entries.iter().any(|entry| entry.label == "/ship"));
        assert!(!entries.iter().any(|entry| entry.label == "/review"));
    }

    #[test]
    fn claude_builtins_appear_in_menu_when_claude_is_active() {
        let root = TempDir::new().unwrap();
        let entries = CommandResolver::menu_entries(root.path(), AgentType::Claude);
        assert!(entries.iter().any(|entry| entry.label == "/compact"));
        assert!(entries.iter().any(|entry| entry.label == "/context"));
        assert!(entries.iter().any(|entry| entry.label == "/cost"));
        assert!(entries.iter().any(|entry| entry.label == "/clear"));
        assert!(entries.iter().any(|entry| entry.label == "/doctor"));
        assert!(entries.iter().any(|entry| entry.label == "/help"));
        assert!(entries.iter().any(|entry| entry.label == "/init"));
        assert!(entries.iter().any(|entry| entry.label == "/memory"));
        assert!(entries.iter().any(|entry| entry.label == "/review"));
    }

    #[test]
    fn claude_builtins_hidden_when_non_claude_is_active() {
        let root = TempDir::new().unwrap();
        for provider in [AgentType::Codex, AgentType::Gemini, AgentType::Opencode] {
            let entries = CommandResolver::menu_entries(root.path(), provider);
            assert!(
                !entries.iter().any(|entry| entry.label == "/compact"),
                "builtin /compact should not appear for {provider:?}"
            );
        }
    }

    #[test]
    fn user_defined_command_shadows_claude_builtin() {
        let root = TempDir::new().unwrap();
        let command_dir = root.path().join(".claude/commands");
        fs::create_dir_all(&command_dir).unwrap();
        fs::write(
            command_dir.join("compact.md"),
            "---\ndescription = \"Custom compact\"\n---\nDo custom compact\n",
        )
        .unwrap();

        let entries = CommandResolver::menu_entries(root.path(), AgentType::Claude);
        let compact_entries: Vec<_> = entries
            .iter()
            .filter(|entry| entry.label == "/compact")
            .collect();
        assert_eq!(
            compact_entries.len(),
            1,
            "should be exactly one /compact entry"
        );
        assert_eq!(compact_entries[0].description, "Custom compact");
    }

    #[test]
    fn discovers_pi_prompts_as_slash_commands() {
        let root = TempDir::new().unwrap();
        let prompts_dir = root.path().join(".pi/prompts");
        fs::create_dir_all(&prompts_dir).unwrap();
        fs::write(
            prompts_dir.join("opsx-explore.md"),
            "---\ndescription: \"Enter explore mode\"\n---\nExplore mode content\n",
        )
        .unwrap();
        fs::write(
            prompts_dir.join("opsx-apply.md"),
            "---\ndescription: \"Implement tasks from an OpenSpec change\"\n---\nApply content\n",
        )
        .unwrap();

        let entries = CommandResolver::menu_entries(root.path(), AgentType::Pi);
        assert!(
            entries.iter().any(|e| e.label == "/opsx-explore"),
            "expected /opsx-explore in menu entries: {:#?}",
            entries
        );
        assert!(
            entries.iter().any(|e| e.label == "/opsx-apply"),
            "expected /opsx-apply in menu entries: {:#?}",
            entries
        );
    }

    #[test]
    fn discovers_pi_skills_as_slash_commands() {
        let root = TempDir::new().unwrap();
        let skill_dir = root.path().join(".pi/skills/openspec-explore");
        fs::create_dir_all(&skill_dir).unwrap();
        fs::write(
            skill_dir.join("SKILL.md"),
            "# openspec-explore\nEnter explore mode\n",
        )
        .unwrap();

        let entries = CommandResolver::menu_entries(root.path(), AgentType::Pi);
        assert!(
            entries.iter().any(|e| e.label == "/openspec-explore"),
            "expected /openspec-explore in menu entries: {:#?}",
            entries
        );
    }

    #[test]
    fn pi_entries_do_not_appear_for_non_pi_providers() {
        let root = TempDir::new().unwrap();
        let prompts_dir = root.path().join(".pi/prompts");
        fs::create_dir_all(&prompts_dir).unwrap();
        fs::write(
            prompts_dir.join("opsx-explore.md"),
            "---\ndescription: \"Enter explore mode\"\n---\nExplore mode content\n",
        )
        .unwrap();

        for provider in [
            AgentType::Claude,
            AgentType::Codex,
            AgentType::Gemini,
            AgentType::Opencode,
        ] {
            let entries = CommandResolver::menu_entries(root.path(), provider);
            assert!(
                !entries.iter().any(|e| e.label == "/opsx-explore"),
                "/opsx-explore should NOT appear for {provider:?}"
            );
        }
    }

    #[test]
    fn provider_specific_entry_is_kept_when_lower_priority_provider_has_same_name() {
        let root = TempDir::new().unwrap();

        let conduit_commands = root.path().join(".conduit/commands");
        fs::create_dir_all(&conduit_commands).unwrap();
        fs::write(
            conduit_commands.join("opsx-explore.md"),
            "---\ndescription: \"Conduit explore\"\n---\nConduit content\n",
        )
        .unwrap();

        let pi_prompts = root.path().join(".pi/prompts");
        fs::create_dir_all(&pi_prompts).unwrap();
        fs::write(
            pi_prompts.join("opsx-explore.md"),
            "---\ndescription: \"Pi explore\"\n---\nPi content\n",
        )
        .unwrap();

        let entries = CommandResolver::menu_entries(root.path(), AgentType::Pi);
        let opsx = entries
            .iter()
            .find(|entry| entry.label == "/opsx-explore")
            .expect("expected /opsx-explore for Pi provider");
        assert_eq!(opsx.source_badge, "Pi command");
        assert_eq!(opsx.description, "Pi explore");
    }

    #[test]
    fn compact_builtin_resolves_passthrough_for_claude() {
        let root = TempDir::new().unwrap();
        let result = CommandResolver::resolve("/compact", root.path(), AgentType::Claude);
        let ResolveResult::ProviderPrompt(prompt) = result else {
            panic!("expected provider prompt, got {result:?}");
        };
        assert_eq!(prompt.agent_text, "/compact");
    }

    #[test]
    fn real_pi_prompts_are_discoverable() {
        // Smoke test: verify the actual project's .pi/prompts are discoverable
        let project_dir = Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .and_then(|p| p.parent())
            .expect("workspace root");
        let entries = CommandResolver::menu_entries(project_dir, AgentType::Pi);
        let pi_labels: Vec<_> = entries
            .iter()
            .filter(|e| e.source_badge.starts_with("Pi"))
            .map(|e| e.label.clone())
            .collect();

        assert!(
            !pi_labels.is_empty(),
            "Expected at least one Pi command from .pi/prompts/\nProject dir: {}\nAll entries: {:#?}",
            project_dir.display(),
            entries
        );
    }
}
