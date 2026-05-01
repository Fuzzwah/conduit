## 1. Core Implementation

- [x] 1.1 Add a helper function (or inline logic) in the archive preflight closure in `crates/conduit-ui/src/app.rs` that, given a repo base path and workspace name, reads `openspec/changes/{name}/tasks.md` and returns the count of `- [ ]` lines (returns 0 if the file does not exist or cannot be read)
- [x] 1.2 Add equivalent logic for `.specify/specs/{name}/tasks.md` in the same preflight closure
- [x] 1.3 Push a warning onto the `warnings` vec when the OpenSpec incomplete count is > 0, formatted as `"OpenSpec change has {n} incomplete task(s)"`
- [x] 1.4 Push a warning onto the `warnings` vec when the Specify incomplete count is > 0, formatted as `"Specify spec has {n} incomplete task(s)"`

## 2. Verification

- [x] 2.1 Run `cargo fmt --check && cargo clippy -- -D warnings && cargo test` and confirm all pass
