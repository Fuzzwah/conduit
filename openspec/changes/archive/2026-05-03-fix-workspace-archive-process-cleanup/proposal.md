## Why

When a workspace is archived, agent processes launched by the TUI are not terminated because the TUI manages PIDs in-memory only and the web server's archive handler can only kill processes it started itself. This leaves orphaned `claude`, `codex`, and similar agent processes running after archival.

## What Changes

- Add `agent_pid` and `agent_pid_start_time` columns to the `session_tabs` database table
- TUI persists the agent PID to the database when a session starts, and clears it when the agent exits
- Archive handler reads PIDs from the database for any session not found in the in-memory map, and terminates those processes

## Capabilities

### New Capabilities

- `session-pid-persistence`: Database-backed storage of active agent PIDs so they can be killed by any code path (archive, web handler, etc.)

### Modified Capabilities

- `archive-workspace-spec-check`: Archive now guarantees termination of all agent processes associated with the workspace, regardless of whether they were launched by the TUI or web server

## Impact

- `crates/conduit-data/src/database.rs` — schema migration adding two new columns
- `crates/conduit-data/src/session_tab.rs` — new `set_agent_pid` / `clear_agent_pid` store methods; updated `SessionTab` struct
- `crates/conduit-ui/src/app.rs` — persist PID on agent start; clear PID on agent exit
- `crates/conduit-web/src/ws/handler.rs` — `stop_session` falls back to DB-stored PID when in-memory session has no PID
