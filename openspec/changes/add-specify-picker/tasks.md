## 1. Spec-Kit Detection Module

- [x] 1.1 Create `src/git/specify.rs` with `SpecifySpec { spec_id, remaining_tasks, total_tasks }` struct and `fetch_specify_specs(repo_path: &Path) -> Vec<SpecifySpec>` scanning `.specify/specs/*/tasks.md`
- [x] 1.2 Export `SpecifySpec` and `fetch_specify_specs` from `src/git/mod.rs`

## 2. Event and Effect Types

- [x] 2.1 Replace `AppEvent::OpenSpecsFetched { repo_id, specs: Vec<OpenSpec> }` with `AppEvent::AllSpecsFetched { repo_id, open_specs: Vec<OpenSpec>, specify_specs: Vec<SpecifySpec> }` in `src/ui/events.rs`
- [x] 2.2 Add `InputMode::SelectingSpecifySpec` variant in `src/ui/events.rs`
- [x] 2.3 Replace `Effect::FetchOpenSpecs { repo_id }` with `Effect::FetchAllSpecs { repo_id }` in `src/ui/effect.rs`
- [x] 2.4 Add `Effect::ShowSpecifyPicker { repo_id: Uuid, issue: Option<GithubIssue> }` in `src/ui/effect.rs`
- [x] 2.5 Add `specify_spec: Option<SpecifySpec>` field to `Effect::CreateWorkspace` in `src/ui/effect.rs`

## 3. SpecifyPicker UI Component

- [x] 3.1 Create `src/ui/components/specify_picker.rs` with `SpecifyPickerState` (mirrors `SpecPickerState` but holds `Vec<SpecifySpec>`) including loading, visible, pending_show, sort, scroll, and issue fields
- [x] 3.2 Implement `SpecifyPicker` Ratatui widget in `specify_picker.rs` (mirrors `SpecPicker`; title = "Select Specify Spec"; item column = `spec_id`)
- [x] 3.3 Export `SpecifyPicker` and `SpecifyPickerState` from `src/ui/components/mod.rs`

## 4. App State

- [x] 4.1 Add `pub specify_picker_state: SpecifyPickerState` field to the app state struct in `src/ui/app_state.rs` and initialise it

## 5. App Effect and Event Handlers

- [x] 5.1 Update `RemoteSynced` handler in `src/ui/app.rs` to push `Effect::FetchAllSpecs` instead of `Effect::FetchOpenSpecs`
- [x] 5.2 Replace the `Effect::FetchOpenSpecs` handler with `Effect::FetchAllSpecs` in `src/ui/app.rs`: spawn one blocking task that calls both `fetch_open_specs()` and `fetch_specify_specs()`, sends `AllSpecsFetched`
- [x] 5.3 Replace the `AppEvent::OpenSpecsFetched` handler with `AppEvent::AllSpecsFetched` in `src/ui/app.rs`: load both result sets; if `pending_show`, prefer spec-kit picker (non-empty specify_specs) over openspec picker, else skip
- [x] 5.4 Update `Effect::ShowSpecPicker` handler in `src/ui/app.rs`: if specify picker is loading or has specs, set `specify_picker_state.pending_show = true` and `InputMode::SelectingSpecifySpec`; else fall through to existing openspec logic
- [x] 5.5 Add `Effect::ShowSpecifyPicker` handler in `src/ui/app.rs` (mirrors `ShowSpecPicker` but operates on `specify_picker_state`)
- [x] 5.6 Update `Effect::CreateWorkspace` handler in `src/ui/app.rs` to extend the naming match to `(&issue, &spec, &specify_spec)`, deriving workspace/branch names from `ss.spec_id` in the `(None, None, Some(ss))` arm
- [x] 5.7 Add `specify_picker_state.tick()` to the spinner tick path in `src/ui/app.rs`
- [x] 5.8 Add `SpecifyPicker` render call in `src/ui/app.rs` when `specify_picker_state.visible`

## 6. Input Handling

- [x] 6.1 Add `handle_specify_picker_key()` in `src/ui/app/app_input.rs` (mirrors `handle_spec_picker_key()`): Up/k, Down/j, s (sort), Enter (select → `CreateWorkspace { specify_spec: Some(...) }`), Esc (skip)
- [x] 6.2 Route `InputMode::SelectingSpecifySpec` to `handle_specify_picker_key()` in the input dispatch in `src/ui/app/app_input.rs`

## 7. Verification

- [x] 7.1 Run `cargo fmt --check && cargo clippy -- -D warnings && cargo test` and confirm all pass
