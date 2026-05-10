# conduit-tui

`conduit-tui` is an additive, TUI-only scaffold that treats `crates/conduit-ui` as the behavioral reference while intentionally avoiding any dependency on the existing web UI.

## Architecture

- `domain` — repositories, workspaces, persistence-facing records, provider metadata
- `runtime` — normalized session event model and a transport trait
- `tab_manager` / `session` — in-memory session and file-tab state
- `app_state` — global UI state, focus, overlays, and layout regions
- `ui` — Ratatui rendering for sidebar, tab bar, chat/raw events, composer, status bar, command palette, and modal overlays
- `app` — root event loop and wiring

## Feature matrix

| Surface / workflow | Scaffold status |
| --- | --- |
| Sidebar with repositories/workspaces | Implemented as demo data |
| Tab bar for sessions/files | Implemented |
| Chat pane + streaming updates | Implemented via mock transport |
| Raw events pane | Implemented |
| Input composer | Implemented |
| Status bar (provider, mode, tokens, context, git/PR) | Implemented |
| Command palette + help modal | Implemented |
| Transport-neutral runtime events | Implemented |
| File viewer tab | Implemented as placeholder text view |
| Persistence/provider/git integrations | Represented by domain types and traits only |
| Fork/handoff/import/queue/work-complete flows | Reserved for future phases |

## Keyboard controls

- `Ctrl+Q` — quit
- `Ctrl+P` — command palette
- `Ctrl+G` — toggle Chat / Raw Events
- `Ctrl+B` — focus sidebar / composer
- `Ctrl+N` — open a mock session tab
- `Ctrl+O` — open a demo file tab
- `Ctrl+4` — toggle Build / Plan mode
- `Tab` / `Shift+Tab` — next / previous tab
- `Enter` — submit prompt
- `Shift+Enter` — newline in composer
- `F1` — help modal
