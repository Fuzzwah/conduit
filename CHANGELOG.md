# Changelog

All notable changes to Conduit will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.3.1] - 2026-03-15

### Added
- `gpt-5.4` as the default Codex model in the model picker, docs, and example config

### Fixed
- Codex app-server compatibility by switching Conduit to the current `thread/*` and `turn/*` RPCs
- OpenAI context-window resolution for `gpt-5.4` and `openai/gpt-5.4`
- Codex JSON-RPC error surfacing so server failures are shown directly instead of deserialization errors

## [0.3.0] - 2026-02-22

### Added
- **OpenCode Agent** - Full integration with OpenCode as a new agent backend, including history restore, tool rendering, reasoning messages, and live event streaming
- **Checkout Workspaces** - Clone and manage repositories as workspaces directly from the TUI
- **/fork Command** - Fork sessions into new tabs with full context preservation
- **Provider-First Architecture** - Provider selection flow with model filtering and onboarding improvements
- **Reasoning Effort Selector** - Configure reasoning effort per session in the TUI
- **TUI Visual Polish** - Margins, role labels, placeholder text, file tabs with syntax highlighting, redesigned stop button
- **Web UI Enhancements** - Progress and unseen indicators on session tabs, auto-drain queued messages, improved error display
- **Homebrew Tap** - Automated formula updates on release via GitHub Actions
- **E2E Testing** - CI pipeline with Termwright for end-to-end TUI testing
- Model catalog refresh for Claude and Codex

### Fixed
- Adaptive frame rate for lower CPU usage
- Session persistence and tab identity across workspace switches
- Modal freeze during long-running preflights
- Chat scroll pinning during streaming
- Cross-agent model switching blocked in active sessions
- OpenCode idle reconnection and error recovery
- Workspace session tab stability
- Plan mode alignment across all agents
- Fork context window resolution

## [0.2.0] - 2025-01-20

### Added
- **Web Interface** - Full web UI with theme support, session management, file viewer
- **Open Source Release** - Conduit is now free and publicly available
- **Gemini CLI Support** - New agent integration for Google's Gemini CLI
- **Install Script** - Curl-based binary installation for easy setup
- Project management in web UI (add/remove projects with Ctrl+N)
- Image support for queued messages
- Workspace archive actions and session persistence
- Web onboarding flow for first-time users
- Tab shortcuts and clickable file paths in web viewer
- Plan/Build mode toggle with Ctrl+Shift+P hotkey
- Theme switcher with auto-scroll and better matching
- Model selector dialog with full agent type switching
- Paged session history
- Bootstrap loading and chat parity updates
- Task tool card rendering
- Shared core services between TUI and web
- E2E testing infrastructure with Termwright
- GitHub issue and PR templates
- Contributing documentation

### Fixed
- Harden workspace/web handlers
- Claude image error handling improvements
- Config borrow conflicts resolved
- Web action edge cases
- Model names updated to latest versions
- Gemini CLI npm package name correction
- Web tabs and models stability
- Tool card display (line spacing, grep highlighting)
- Chat input controls alignment

## [0.1.6] - 2025-01-18

### Added
- File viewer with tabs and clickable file paths in chat
- Hover highlighting for clickable file paths
- Slash command menu for quick actions
- Plan/build prompts for all supported models
- Human-readable elapsed time display
- Configurable chat scrollbar

### Fixed
- Chat selection copy alignment
- Relative path resolution in `:open` command
- Footer click context alignment with hint selection
- UTF-8 safe path truncation in file viewer header
- Error display when home directory is unavailable for tilde expansion

## [0.1.5] - 2025-01-10

### Added
- Session import from Claude Code and Codex CLI
- Model selector with `Ctrl+O`
- View mode toggle between Chat and Raw Events

### Fixed
- Codex command output coalescing
- Dialog instructions rendering on bottom border
- NPX fallback detection
- Raw event logging hardening
- Gemini input signature alignment

## [0.1.4] - 2025-01-05

### Added
- Git integration with PR status and branch tracking
- Worktree management support
- Session persistence and resumption

### Fixed
- Tab switching stability improvements
- Token counting accuracy

## [0.1.3] - 2025-01-01

### Added
- Real-time token tracking with cost estimation
- Status bar with session information
- Sidebar toggle with `Ctrl+T`

### Fixed
- Streaming response handling for large outputs
- Input box multi-line support

## [0.1.2] - 2024-12-28

### Added
- Tab-based session management (up to 10 concurrent sessions)
- Keyboard shortcuts for tab navigation
- Session forking with `Alt+Shift+F`

### Fixed
- Agent process cleanup on tab close
- Memory usage optimization

## [0.1.1] - 2024-12-25

### Added
- Codex CLI agent support
- Agent switching within sessions
- Markdown rendering in chat view

### Fixed
- Claude Code event parsing
- Syntax highlighting for code blocks

## [0.1.0] - 2024-12-20

### Added
- Initial release
- Claude Code agent integration
- Basic TUI with chat interface
- Real-time streaming responses
- Terminal-native UI with Ratatui

[Unreleased]: https://github.com/conduit-cli/conduit/compare/v0.3.1...HEAD
[0.3.1]: https://github.com/conduit-cli/conduit/compare/v0.3.0...v0.3.1
[0.3.0]: https://github.com/conduit-cli/conduit/compare/v0.2.0...v0.3.0
[0.2.0]: https://github.com/conduit-cli/conduit/compare/v0.1.6...v0.2.0
[0.1.6]: https://github.com/conduit-cli/conduit/compare/v0.1.5...v0.1.6
[0.1.5]: https://github.com/conduit-cli/conduit/compare/v0.1.4...v0.1.5
[0.1.4]: https://github.com/conduit-cli/conduit/compare/v0.1.3...v0.1.4
[0.1.3]: https://github.com/conduit-cli/conduit/compare/v0.1.2...v0.1.3
[0.1.2]: https://github.com/conduit-cli/conduit/compare/v0.1.1...v0.1.2
[0.1.1]: https://github.com/conduit-cli/conduit/compare/v0.1.0...v0.1.1
[0.1.0]: https://github.com/conduit-cli/conduit/releases/tag/v0.1.0
