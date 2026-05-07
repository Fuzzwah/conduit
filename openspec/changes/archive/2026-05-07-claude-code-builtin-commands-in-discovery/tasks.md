## 1. Static Builtin Registry

- [x] 1.1 Add `inject_claude_builtins()` method to `DiscoveryRegistry` in `crates/conduit-resolver/src/lib.rs` that inserts a static `ProviderInvocation::PromptCommand` entry for each Claude built-in command (`/compact`, `/context`, `/cost`, `/clear`, `/doctor`, `/help`, `/init`, `/memory`, `/review`) using sentinel paths (`PathBuf::from("<builtin>/<name>")`) and `source: Claude`
- [x] 1.2 Call `inject_claude_builtins()` at the start of `DiscoveryRegistry::discover()`, before filesystem scanning, so user-defined commands with the same name take precedence via the sort key (filesystem entries sort ahead of builtins)
- [x] 1.3 Add descriptions for each builtin command (e.g., "Compact conversation context", "Show context window usage", etc.)

## 2. Deduplication and Precedence

- [x] 2.1 Ensure builtin entries use a sort key that ranks lower priority than real filesystem entries — verify the existing `sort_key()` function handles this, or adjust it so `<builtin>` paths sort after real paths

## 3. Verification

- [x] 3.1 Run `cargo fmt --check && cargo clippy --workspace --all-targets -- -D warnings && cargo test --workspace` and confirm all pass
- [x] 3.2 Manually test: switch to Claude agent, type `/comp`, and confirm `/compact` appears in the slash menu with a "Claude command" source badge
- [x] 3.3 Manually test: switch to a non-Claude agent and confirm Claude builtin entries do not appear in the slash menu
- [x] 3.4 Manually test: select `/compact` from the menu, confirm it is inserted into the input, submit it, and confirm Claude Code compacts the context
