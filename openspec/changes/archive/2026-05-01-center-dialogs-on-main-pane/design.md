## Context

In `src/ui/app.rs`, the `draw()` method splits the terminal into a sidebar and a main pane. The split produces `right_area` (everything to the right of the sidebar). All dialog components receive an `area: Rect` argument and center themselves within it using `(area.width - dialog_width) / 2` and `area.x` as the base offset. Currently every dialog render call passes `size` (the full terminal `Rect`), so dialogs center across the full width including the sidebar column.

`right_area` is already computed before any dialog is rendered and is in scope for the rest of `draw()`. When no sidebar is visible, `right_area == size`, so substituting it is safe in all cases.

## Goals / Non-Goals

**Goals:**
- Dialogs appear centered in the visible main pane when the sidebar is open.
- No behavior change when the sidebar is hidden.

**Non-Goals:**
- Changing dialog component internals or the `DialogFrame` centering algorithm.
- Handling any other sidebar-width-aware layout (e.g., mouse hit-testing, which already uses its own stored area).

## Decisions

**Pass `right_area` to all dialog render calls instead of `size`.**

The centering math lives inside each dialog's `render()` method and operates on the `area` rect it receives. Passing a different rect requires no changes to dialog components — it's purely a call-site change.

Alternative considered: adding a `main_pane: Rect` field to each dialog state and computing centering there. Rejected — unnecessary complexity; the area is already available at the call site.

**Use `right_area` (full sidebar-excluded height) rather than `content_area` (which also strips the footer row).**

Dialogs should be vertically centered over the entire main pane including the footer row, matching what a user perceives as the "content area". Using `content_area` would shift dialogs one row up, which is barely perceptible but incorrect.

## Risks / Trade-offs

- [Large diff] The substitution touches ~30 call sites across two blocks in `draw()`, plus the `render_theme_picker` helper. All changes are mechanical (find `size` → replace with `right_area`) with no logic changes — low regression risk.
- [Shadowed variable] The CloningRepository and RemovingProject blocks assign `let content_area = DialogFrame::new(...).render(size, ...)`. The `content_area` here is a local shadow, not `draw()`'s outer `content_area`. Both `size` arguments in those calls must be replaced with `right_area`.
