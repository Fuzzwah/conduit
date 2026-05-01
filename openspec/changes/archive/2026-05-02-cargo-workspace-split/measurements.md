# Cargo Workspace Split — Build-Time Measurements

Wall-clock build times before and after the workspace split. Same machine,
warm sccache disabled, `cargo build`/`cargo build --workspace` only.

Raw `/usr/bin/time -v` and `cargo --timings` logs are checked in alongside
this file (`baseline-*.log`, `split-*.log`).

## Cold build

`cargo clean && cargo build [--workspace]`

| Build       | Wall-clock | Δ vs baseline |
|-------------|-----------:|---------------|
| Baseline    |   1m 31s   | —             |
| Post-split  |   1m 15s   | **−16s (−18%)** |

Cold builds are slightly faster post-split because `[profile.dev]` was
already tuned and the per-tier crates can compile in parallel rather than
all serialised inside a single mega-crate.

## Incremental edits

`touch <file> && cargo build --workspace`

| Touched file (post-split path) | Baseline | Post-split | Δ      | What recompiled                                          |
|---|---:|---:|---|---|
| `conduit-agent/src/runner.rs`  | 10.09s   | **6.31s**  | **−37%** | agent → config → data → resolver → session → core → web → ui → conduit → bin |
| `conduit-util/src/lib.rs`      |  9.33s   | **6.77s**  | **−27%** | util (transitive: agent, theme, config, data, …) — util is the deepest leaf |
| `conduit-web/src/server.rs`    |  9.14s   | **4.35s**  | **−52%** | web → conduit → bin **(no ui rebuild — the original goal)** |
| `conduit-ui/src/app.rs`        | 10.20s   | **4.02s**  | **−60%** | ui → conduit → bin (umbrella + bin re-link is tiny)      |

The two cases the user explicitly cared about — editing inside `web/` and
inside `ui/` — both now skip recompiling the *other* of the two big crates
entirely. That's the win the split was designed to deliver.

The agent/util cases still cascade through several downstream crates because
the dependency graph is genuinely tall (`agent` is depended on by config,
data, resolver, session, core, web, ui, …). Splitting those further would
fragment behaviour and is explicitly out of scope.

## Verification of the no-ui-rebuild guarantee

```
$ touch crates/conduit-web/src/server.rs && cargo build --workspace -v 2>&1 | grep "Compiling conduit-ui"
(empty — conduit-ui was NOT recompiled)
```

## Caveats

- `mold` linker config (planned in tier 15) was not committed — would
  require every developer to install `mold` + `clang` first. Adding it
  locally as `.cargo/config.toml` should knock another second or two off
  each link step.
- Numbers above are single-run wall-clock; ±0.5s noise is normal. The
  *relative* picture (web/ui edits halving in time) is the load-bearing
  outcome.
