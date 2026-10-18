# Research: gpui-kit Diff and Annotation contract

**Ticket:** [#4](https://github.com/jacobdrury/reviewdesk/issues/4)  
**Question:** What is the concrete contract for feeding a Patch into gpui-kit Diff (`DiffFile::parse`, `DiffState`, events) and for placing in-memory Annotations (`DiffAnnotation`, related events)? What must the app own versus what the kit owns, for a first-slice stub with no forge writeback?

**Inspected:** `gpui-kit` / `gpui-component` **0.7.3**, git tag `v0.7.3` @ `67265a36e260ffe03c5aaebdcd8d654dbf892eab` ([GitHub](https://github.com/longbridge/gpui-kit/releases/tag/v0.7.3), [crates.io gpui-kit](https://crates.io/crates/gpui-kit/0.7.3), [docs.rs gpui-component](https://docs.rs/gpui-component/0.7.3/gpui_component/diff/index.html)).  
**Primary sources:** [Diff component docs](https://gpui-kit.com/component/diff), [`website/component/diff.md`](https://github.com/longbridge/gpui-kit/blob/v0.7.3/website/component/diff.md), `crates/component/src/diff/*`, `examples/tig`, `crates/story/src/stories/diff_story.rs`, `crates/kit/tests/diff.rs`.

---

## Verdict (first-slice stub)

1. Load a unified/Git patch string in the app (GitHub API for Review Desk).
2. `DiffFile::parse(&patch)` once (or on a background executor), then install into a retained `Entity<DiffState>` via `DiffState::new` / `set_files`.
3. Render `Diff::new(&state)` with a bounded height. Do not recreate state each frame.
4. Keep comment bodies and draft state in app memory. Pass placement-only `DiffAnnotation` values into `Diff::annotations` and draw UI in `render_annotation`.
5. Create drafts with `on_add_annotation` (receives `DiffLineRange`). There are **no** annotation variants on `DiffEvent`.
6. Skip forge submit and persistence for the stub.

Ticket type names (`DiffFile::parse`, `DiffState`, `DiffAnnotation`, `DiffEvent`) match 0.7.3. Import them from `gpui_kit::component::diff`, not the crate root.

---

## Crate map

| Dependency | Role in 0.7.3 |
| --- | --- |
| `gpui-kit = "0.7.3"` | App-facing umbrella; default features enable `component` + `assets`. Pins GPUI snapshot (`gpui-pre =0.3.9`). |
| `gpui-component` 0.7.3 | Implements Diff under `gpui_kit::component::diff`. |
| Call `gpui_kit::init(cx)` | Required before opening windows that use components. |

Release notes for Diff (readonly viewer, app owns patch/comments/saving) and the `examples/tig` history browser: [v0.7.3 release](https://github.com/longbridge/gpui-kit/releases/tag/v0.7.3).

---

## Patch → Diff contract

### What the kit does

- Parses an **externally supplied** unified or Git patch into `Vec<DiffFile>` without computing old/new diffs or loading repository files. ([docs](https://gpui-kit.com/component/diff); [`DiffFile::parse`](https://docs.rs/gpui-component/0.7.3/gpui_component/diff/struct.DiffFile.html#method.parse))
- Retains focus, viewport, folding, selection, and background syntax/inline emphasis in one `DiffState` for all files. ([`DiffState`](https://docs.rs/gpui-component/0.7.3/gpui_component/diff/struct.DiffState.html))
- Virtualizes a readonly `Diff` surface (Unified or Split). Diff owns scrolling; give it a bounded height. ([docs](https://gpui-kit.com/component/diff))

### What the app must do

| Step | Contract |
| --- | --- |
| Obtain patch text | App-owned (Git, forge API, AI, fixture). Kit never fetches. |
| Parse | `DiffFile::parse(patch) -> Result<Vec<DiffFile>, DiffParseError>`. Handle errors in the app. |
| Retain state | `let diff = cx.new(\|cx\| DiffState::new(files, cx));` store as `Entity<DiffState>`. |
| Present | `Diff::new(&self.diff).h(...).w_full()` during render only. |
| Replace patch | `state.set_files(updated, cx)`. Resets expansion, selection, conflict resolutions, and scrolling. |
| Large patches | Parse off-thread; install only if the result still matches the current revision (tig pattern). |

Minimal shape from the official docs:

```rust
use gpui_kit::component::diff::{Diff, DiffFile, DiffState};

let files = DiffFile::parse(patch)?;
let diff = cx.new(|cx| DiffState::new(files, cx));
// render:
Diff::new(&self.diff).h(rems(24.)).w_full()
```

Accepted input: unified (`---`, `+++`, `@@`) and Git diff text, including `git format-patch`, `diff.noprefix`, `diff.mnemonicPrefix`. Unsupported combined diffs and malformed hunks yield `DiffParseError` with `line()` / `message()`. ([docs](https://gpui-kit.com/component/diff))

Other constructors (not required for the PR-patch slice): `DiffFile::unchanged(path, text)`, `DiffFile::parse_conflicts(path, working_text)`.

`DiffFile::path()` is the stable file id for annotations, selection, collapse, and navigation. Line numbers are **one-based source coordinates** on `DiffSide::Original` / `Modified`, independent of fold/mode. ([docs](https://gpui-kit.com/component/diff))

### Reference: `examples/tig`

At `v0.7.3`, tig loads commit patches with `git show --patch`, then `DiffFile::parse`, keeps `Arc<Vec<DiffFile>>` for the sidebar, and installs **one file at a time** into a retained `Entity<DiffState>` via `set_files([file], cx)`. History loads on a background executor with a revision counter so stale tasks cannot overwrite newer selection. ([`examples/tig/src/repository.rs`](https://github.com/longbridge/gpui-kit/blob/v0.7.3/examples/tig/src/repository.rs), [`examples/tig/src/app.rs`](https://github.com/longbridge/gpui-kit/blob/v0.7.3/examples/tig/src/app.rs))

Review Desk can either feed all PR files into one `DiffState` (docs default) or mirror tig’s one-file viewport; both are valid kit usage.

---

## Annotation contract

### Ownership split

| Owner | Responsibility |
| --- | --- |
| **App** | Stable unique IDs; comment/draft body and UI; when to add/remove/move annotations; mapping ID → content; forge writeback and persistence (none in first slice). |
| **Kit** | Placement slots under a line or file header; virtualized measurement; hover/selection “add” affordance when `on_add_annotation` is set; excluding annotation text from source copy. |

Docs: “Keep comment content and draft state in the application” and “Applications own patch loading, review comments and saving.” ([Annotations](https://gpui-kit.com/component/diff); [v0.7.3 release](https://github.com/longbridge/gpui-kit/releases/tag/v0.7.3))

### Placement API

`DiffAnnotation` is placement metadata only (id + target), not the comment payload:

```rust
DiffAnnotation::line(id, DiffLinePosition::new(path, DiffSide::Modified, line))
DiffAnnotation::file(id, path)
```

Accessors: `id()`, `path()`, `position()` (`None` for file annotations). ([`DiffAnnotation`](https://docs.rs/gpui-component/0.7.3/gpui_component/diff/struct.DiffAnnotation.html))

Wire during render:

```rust
Diff::new(&self.diff)
    .annotations(annotations)
    .render_annotation(|annotation, _, cx| { /* look up app content by annotation.id() */ })
    .on_add_annotation(cx.listener(|this, range: &DiffLineRange, _, cx| {
        this.start_comment(range.clone(), cx);
    }))
```

Rules from docs:

- Supply **both** `annotations` and `render_annotation`.
- IDs must be stable and unique within the Diff.
- Hidden-context annotations appear when that line is revealed; invalid/missing-side positions are not rendered.
- Height may change freely; kit remeasures. Split mode stretches the paired row to the taller side.
- `annotation_content` is a compatibility alias for `render_annotation`.

In-tree models of the stub:

- Story: owns `comments: Vec<{id, range, body, resolved}>`, maps to `DiffAnnotation::line` on `range.end_side()` / `range.end()`, renders from a snapshot in `render_annotation`, pushes new drafts in `on_add_annotation`. ([`diff_story.rs`](https://github.com/longbridge/gpui-kit/blob/v0.7.3/crates/story/src/stories/diff_story.rs))
- Kit UI test: `Review` holds `annotations: Vec<DiffAnnotation>` plus optional `InputState` inside the annotation slot. ([`crates/kit/tests/diff.rs`](https://github.com/longbridge/gpui-kit/blob/v0.7.3/crates/kit/tests/diff.rs))

---

## Events vs annotation hooks

`DiffState` implements `EventEmitter<DiffEvent>`. `DiffEvent` is `#[non_exhaustive]`:

```rust
SelectionStarted(DiffLineRange)
SelectionChanged(Option<DiffLineRange>)
SelectionEnded(DiffLineRange)
FileExpanded(SharedString)
FileCollapsed(SharedString)
ConflictResolved(SharedString, usize)
```

([`DiffEvent`](https://docs.rs/gpui-component/0.7.3/gpui_component/diff/enum.DiffEvent.html); [Events](https://gpui-kit.com/component/diff))

Programmatic `set_selected_lines` / `set_file_collapsed` / `resolve_conflict` do **not** emit.

**Annotation-related surface is not `DiffEvent`.** Starting a comment is the `on_add_annotation` builder (callback with `&DiffLineRange`). Optional helpers: `on_line_click`, `on_line_hover`. Match `DiffEvent` with a wildcard arm.

For the stub, subscribe to selection/collapse only if useful (e.g. mark-file-viewed later). Annotation create/edit can ignore `DiffEvent` entirely.

---

## App vs kit checklist (no forge writeback)

| Concern | App | Kit |
| --- | --- | --- |
| Fetch PR patch from GitHub | Yes | No |
| Parse / install files | Call `parse` + `new`/`set_files` | Parser + retained presentation state |
| Multi-file list / inbox chrome | Yes | Optional (use List/Table/etc.) |
| Readonly diff chrome, fold, syntax, scroll | Configure / host | Owns |
| In-memory draft comments | Own `Vec` + bodies | Placement + add button + measure |
| Persist / submit review | Out of slice | Not provided |
| Conflict merge writeback | N/A for PR patch view | Conflict UI only if you feed `parse_conflicts` |

---

## Suggested first-slice wiring

```text
reviewdesk-core          fetch patch string for a PR
reviewdesk (view)
  Entity<DiffState>      created once; set_files when patch arrives
  Vec<LocalAnnotation>   { id, range or position, body }
  render:
    Diff::new(&diff)
      .annotations(map LocalAnnotation → DiffAnnotation)
      .render_annotation(|a, …| lookup body by a.id())
      .on_add_annotation(|range, …| push draft LocalAnnotation)
```

Pin `gpui-kit = "0.7.3"` (or `"0.7"` if Cargo resolves to that). Do not mix with an unrelated Zed `gpui` main. Enable tree-sitter language features as needed for syntax; Diff still renders change colors without them.

---

## API name notes (vs ticket wording)

| Ticket assumption | 0.7.3 reality |
| --- | --- |
| `DiffFile::parse`, `DiffState`, `DiffAnnotation`, `DiffEvent` | Present with those names. |
| “gpui-kit Diff” | Implemented in `gpui-component`, imported via `gpui_kit::component::diff`. |
| “related events” for annotations | Use `on_add_annotation` (+ optional line click/hover). **No** annotation variants on `DiffEvent`. |
| `render_annotation` / `annotation_content` | Prefer `render_annotation`; `annotation_content` is a legacy alias. |

---

## Sources (pinned)

1. https://gpui-kit.com/component/diff (mirrors `website/component/diff.md` @ `v0.7.3`)
2. https://docs.rs/gpui-component/0.7.3/gpui_component/diff/
3. https://github.com/longbridge/gpui-kit/releases/tag/v0.7.3
4. https://github.com/longbridge/gpui-kit/tree/v0.7.3/examples/tig
5. https://github.com/longbridge/gpui-kit/blob/v0.7.3/crates/story/src/stories/diff_story.rs
6. https://github.com/longbridge/gpui-kit/blob/v0.7.3/crates/kit/tests/diff.rs
7. https://crates.io/crates/gpui-kit/0.7.3 and https://crates.io/crates/gpui-component/0.7.3
