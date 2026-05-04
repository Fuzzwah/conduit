## Context

When conduit runs as a TUI (`conduit` with no subcommand), it spawns agent processes (claude, codex, etc.) directly as child processes of itself and tracks their PIDs entirely in in-memory state (`session.agent_pid`, `session.agent_pid_start_time` on `TabState`).

The web server (`conduit serve`) maintains its own in-memory sessions map (`SessionManager.sessions: HashMap<Uuid, ActiveSession>`) which only contains sessions started via the web API or WebSocket path. These two PID registries are completely disjoint.

When a workspace is archived (via the web UI), `archive_workspace` calls `session_manager.stop_workspace_sessions(id)`, which iterates DB session tabs for that workspace and calls `stop_session(session_id)` for each. `stop_session` removes the entry from the in-memory map and kills the stored PID — but TUI-launched sessions are never in that map, so their processes are left running.

## Goals / Non-Goals

**Goals:**
- Archive always terminates all agent processes for a workspace regardless of launch path (TUI or web)
- PID data is durable: a crash or restart doesn't leave a stale PID that could accidentally kill a future unrelated process
- No behavioral change to existing web-launched session cleanup

**Non-Goals:**
- Unifying the TUI and web session registries at runtime (they remain separate; the DB is the shared source of truth only at cleanup time)
- Cleaning up processes from crashed/abnormal exits in scenarios other than explicit archive

## Decisions

### Store PIDs in `session_tabs` rather than a separate table

The `session_tabs` table already has `workspace_id` and is the join point used by `stop_workspace_sessions`. Adding two nullable columns (`agent_pid INTEGER`, `agent_pid_start_time INTEGER`) is the smallest change and follows the existing pattern for optional fields (see the many `ALTER TABLE session_tabs ADD COLUMN` migrations already present).

Alternative: a separate `active_pids` table. Rejected — more schema surface, no benefit for this use case.

### TUI writes PID to DB on agent start; clears it on agent exit

The TUI already has the `session_tab_store` via `ConduitCore`. The write happens at the same site where `session.agent_pid` is set today (`AppEvent::AgentStarted` handler, app.rs ~line 7778). The clear happens where `session.agent_pid = None` is set today (agent exit event, ~line 7855) and also in `interrupt_agent` / `stop_agent_for_tab`.

This is a fire-and-forget DB write — errors are logged and swallowed so a DB failure never disrupts agent startup.

### `stop_session` falls back to DB PID when in-memory session has no PID

Rather than changing `stop_workspace_sessions`, the fix lives in `stop_session`: after removing from the in-memory map, if `pid` is `None` (session not in map, or in map but PID never set), read the PID from the DB and kill it. This keeps the fallback centralized and handles both "session not in map at all" and "session in map with pid: None" (the idle-subscription case from `subscribe()`).

`stop_session` gains a reference to the `session_tab_store` (already available on `SessionManager` via `self.core`).

Alternative: read PIDs in `stop_workspace_sessions` before calling `stop_session`. Rejected — duplicates kill logic and bypasses `terminate_process_tree`.

### Stale PID safety

`terminate_process_tree` already validates PID identity via `pid_start_time` (reads `/proc/<pid>/stat` on Linux). A stale PID left in the DB after an agent crash will either belong to an unrelated process (start time won't match → kill skipped) or the PID slot will be recycled but won't match. This gives the same safety guarantee as the existing in-memory path.

The TUI also clears the PID in the DB on clean agent exit, so stale entries are rare in practice.

## Risks / Trade-offs

**Race: agent exits between DB write and archive kill** → `terminate_process_tree` is already tolerant of "process already gone" (ESRCH); it logs and returns. No action needed.

**Race: agent starts on a new session after archive begins** → archive marks the workspace archived in DB before stop; new sessions should not be started against an archived workspace (this is a pre-existing guard). Out of scope for this fix.

**DB write on agent start adds latency** → negligible; a single `UPDATE` on a local SQLite file. Agent startup already does async I/O.

**`agent_pid_start_time` is `u64` in Rust but SQLite stores as `INTEGER` (i64)** → values from `/proc/<pid>/stat` fit in i64 (they are jiffies since boot). Cast via `as i64` on write and `as u64` on read.

## Migration Plan

1. Add `ALTER TABLE session_tabs ADD COLUMN agent_pid INTEGER` and `ALTER TABLE session_tabs ADD COLUMN agent_pid_start_time INTEGER` to `apply_migrations()` in `database.rs`, guarded by column-existence checks (same pattern used for all other migrations).
2. Existing rows get `NULL` for both columns, which is correct (no running process to kill for historical sessions).
3. No rollback concern — additive schema change; old binary ignores unknown columns on read.
