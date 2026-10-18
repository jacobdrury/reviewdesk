# UI performance

How to keep Review Desk snappy when building Overview, Diff, Inbox, or any scrollable pane.

Read this before choosing a scroll or disclosure layout. Coding-standards judgement for review lives in root `CODING_STANDARDS.md` (Snappy UI).

**Zed is the default reference** for scroll, list, sticky chrome, and disclosure on GPUI. Prefer matching Zed’s patterns over inventing a new layout. Review Desk keeps gpui-kit primitives (`TextView::scrollable`, `uniform_list`, Diff’s variable-height `list`); use the table below when you need a concrete example in upstream Zed.

## Goal

Unbounded content must not lay out or paint the whole document on every scroll frame. Prefer one virtualized scroll surface for a pane. Tradeoffs that break that rule need engineer approval on the ticket or PR.

## Look here in Zed

| Principle | What it means in Review Desk | Where to look in [zed-industries/zed](https://github.com/zed-industries/zed) |
| --- | --- | --- |
| **Virtualize first** | Inbox and Files file list use `uniform_list`; Overview uses `TextView::scrollable`; Diff uses gpui-kit’s variable-height `list`. Do not lay out unbounded content in fit mode inside a scrolling div. | [`crates/gpui/src/elements/uniform_list.rs`](https://github.com/zed-industries/zed/blob/main/crates/gpui/src/elements/uniform_list.rs), [`crates/gpui/src/elements/list.rs`](https://github.com/zed-industries/zed/blob/main/crates/gpui/src/elements/list.rs) (`ListState`, scroll-to) |
| **Flatten collapsed rows** | Closed `<details>` emit toggle fences only; open bodies inline as normal markdown blocks in the composed Overview document. File tree UIs omit children of collapsed folders from the visible row model. | [`crates/project_panel/src/project_panel.rs`](https://github.com/zed-industries/zed/blob/main/crates/project_panel/src/project_panel.rs) (`visible_entries`, expanded-dir filtering) |
| **One vertical scroll owner** | Overview is one composed markdown document with one vertical scrollbar (title, meta, toggles, and body scroll together). Inbox is one list scroller under its header. | [`crates/agent_ui/src/conversation_view.rs`](https://github.com/zed-industries/zed/blob/main/crates/agent_ui/src/conversation_view.rs) (single `list` + `ListState` for the thread) |
| **Sticky as list decoration** | Sticky path/hunk headers must not add a second vertical scroller; they ride on the existing virtualized list as decorations. | [`crates/ui/src/components/sticky_items.rs`](https://github.com/zed-industries/zed/blob/main/crates/ui/src/components/sticky_items.rs) (`UniformListDecoration`, sticky on the list surface) |
| **Horizontal-only nested scroll** | Wide code, tables, or diff hunks may use horizontal overflow inside a row; the pane’s vertical wheel still drives the pane’s vertical owner. | GPUI [`overflow_x_scroll`](https://github.com/zed-industries/zed/blob/main/crates/gpui/src/elements/div.rs) on inner content; avoid pairing it with a second vertical scroller for the same pane |

## Review Desk panes

### Overview (one pane, one vertical scrollbar)

Title, meta, disclosure toggles, and body are composed into **one** `TextView::scrollable` document. Do not pin meta above a nested scrollable body.

### Files tab (two panes, two vertical scrollbars)

The Files tab is a **horizontal split**, not one nested scroll region:

1. **File list pane** — virtualized path list under the “FILES” header, with its own vertical scrollbar.
2. **Diff pane** — gpui-kit Diff for the selected file, with its own vertical scrollbar.

Two vertical scrollbars in one tab is intentional: each pane owns its vertical scroll. Do not merge the list and Diff into a single scroller without engineer approval on the ticket or PR.

### Inbox

Same shape as other single-pane lists: pinned “INBOX” header, then one `uniform_list` scroller below (bounded height under the header).

## gpui-kit patterns

### TextView fit vs scrollable

| Mode | When | Cost |
| --- | --- | --- |
| Default fit (`TextView::markdown` without `.scrollable(true)`) | Small labels, titles, short prose | Lays out every block. Fine when the content is tiny. |
| `.scrollable(true)` | PR bodies, long markdown, anything that can grow without a fixed bound | Virtualizes by block via `list`. Parent must give a **bounded height** (`size_full` / `flex_1().min_h_0()` chain to the pane). |

Do not put a large fit-mode `TextView` inside `overflow_y_scroll`. That combination is the usual source of Overview jank: the div scrolls, but every frame still lays out the full document.

### One scrollbar per pane

The Overview (and similar **single-pane** surfaces) should feel like one surface: title, meta, toggles, and body share a single virtualized scroller.

Avoid:

- A pinned header or toggle row that leaves a smaller nested `TextView::scrollable` for the rest
- Nested **vertical** scroll regions (outer `overflow_y_scroll` + inner vertical virtualized scroll) unless the ticket records engineer approval
- Mounting heavy disclosure bodies while they are closed

The Files tab is the documented exception: two panes, two vertical scrollbars (see above).

### Bounded flex under pinned chrome

Pinned chrome (title bar, Inbox header, Review tab bar, Files header) is fine when the virtualized child sits in a **`flex_1().min_h_0()`** (or equivalent) chain so the scroller gets a max height. Pinned chrome that is not itself a scroller is allowed; pinned chrome that shrinks the rest into a **nested vertical scroller** for the main content is not.

### Disclosure (`<details>` / `<summary>`)

GitHub and Renovate bodies use HTML `<details>`. gpui-kit `TextView` does not turn those into toggles by itself.

Working pattern used by Overview:

1. Parse `<details>` out of the PR body into segments (`overview_body`).
2. Compose one markdown document for the whole pane.
3. Closed details emit a small custom fence (`reviewdesk-details`) rendered as a toggle (`DetailsTogglePlugin`).
4. Open details emit the same toggle fence, then **inline the body as normal markdown blocks** after it so the parent scrollable `TextView` virtualizes headings, lists, and paragraphs.
5. Default closed. Do not build or mount open-body elements while closed.
6. Provider-supplied `<details open>` with a **heavy** nested body (at or above `HEAVY_DETAILS_BODY_MIN_CHARS` in `overview_body`) still composes **closed** on first paint; the reviewer toggle inlines the body. Light Provider-open bodies may still open immediately. Covered by compose tests in `overview_body`.

Closed-by-default is both UX (matches GitHub) and perf (Renovate release notes are huge).

## Heavy fixture

When a ticket touches scroll, markdown, lists, or disclosure, name a concrete heavy case in acceptance criteria. Examples:

- **Overview:** one scrollbar stays smooth with a Renovate / Dependabot release-notes body (for example `home-assistant/core` image updates), including when large `<details>` sections are opened.
- **Files / Diff:** file list and Diff stay responsive on a **multi-file Patch** (many paths; large hunks on at least one file).

If the design needs nested vertical scroll or pinned chrome that shrinks content into a nested scroller, the ticket must say the engineer approved that tradeoff.

## Checklist before you call a UI change done

- [ ] Unbounded content uses a virtualized surface (`TextView::scrollable`, `list`, or `uniform_list`), not fit layout in a scrolling div
- [ ] Single-pane surfaces (Overview, Inbox, each Files sub-pane) have one vertical scrollbar unless a recorded tradeoff says otherwise; Files tab is two panes (list + Diff), each with its own vertical scrollbar
- [ ] Virtualized children under pinned chrome sit in a bounded `flex_1().min_h_0()` (or equivalent) flex chain so virtualization engages
- [ ] Sticky headers (if any) are list decorations on the existing vertical owner, not a second vertical scroller
- [ ] Nested scroll is horizontal-only inside rows; the pane’s vertical wheel drives the pane’s vertical owner
- [ ] Heavy disclosure starts closed and does not mount its body until open
- [ ] A named heavy fixture is in the ticket acceptance criteria and you have reasoned about it (Renovate/Dependabot Overview; multi-file Patch for Files/Diff)
