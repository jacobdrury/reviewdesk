# UI performance

How to keep Review Desk snappy when building Overview, Diff, Inbox, or any scrollable pane.

Read this before choosing a scroll or disclosure layout. Coding-standards judgement for review lives in root `CODING_STANDARDS.md` (Snappy UI).

## Goal

Unbounded content must not lay out or paint the whole document on every scroll frame. Prefer one virtualized scroll surface for a pane. Tradeoffs that break that rule need engineer approval on the ticket or PR.

## Local persistence and Inbox

[LocalStore](../adr/0001-localstore-turso-local-first.md) makes Inbox and PR data available without waiting on the network, but the list UI must still **virtualize**: use `uniform_list` (or equivalent) so only visible rows lay out each frame. Loading from disk is not a reason to mount or measure every Inbox row on every scroll tick. Background Provider sync updates LocalStore; the virtualized Inbox reads from the store and refreshes when data changes.

## gpui-kit patterns

### TextView fit vs scrollable

| Mode | When | Cost |
| --- | --- | --- |
| Default fit (`TextView::markdown` without `.scrollable(true)`) | Small labels, titles, short prose | Lays out every block. Fine when the content is tiny. |
| `.scrollable(true)` | PR bodies, long markdown, anything that can grow without a fixed bound | Virtualizes by block via `list`. Parent must give a **bounded height** (`size_full` / `flex_1().min_h_0()` chain to the pane). |

Do not put a large fit-mode `TextView` inside `overflow_y_scroll`. That combination is the usual source of Overview jank: the div scrolls, but every frame still lays out the full document.

### One scrollbar per pane

The Overview (and similar panes) should feel like one surface: title, meta, toggles, and body share a single virtualized scroller.

Avoid:

- A pinned header or toggle row that leaves a smaller nested `TextView::scrollable` for the rest
- Nested scroll regions (outer `overflow_y_scroll` + inner scrollable) unless the ticket records engineer approval
- Mounting heavy disclosure bodies while they are closed

### Disclosure (`<details>` / `<summary>`)

GitHub and Renovate bodies use HTML `<details>`. gpui-kit `TextView` does not turn those into toggles by itself.

Working pattern used by Overview:

1. Parse `<details>` out of the PR body into segments (`overview_body`).
2. Compose one markdown document for the whole pane.
3. Closed details emit a small custom fence (`reviewdesk-details`) rendered as a toggle (`DetailsTogglePlugin`).
4. Open details emit the same toggle fence, then **inline the body as normal markdown blocks** after it so the parent scrollable `TextView` virtualizes headings, lists, and paragraphs.
5. Default closed. Do not build or mount open-body elements while closed.

Closed-by-default is both UX (matches GitHub) and perf (Renovate release notes are huge).

## Heavy fixture

When a ticket touches scroll, markdown, lists, or disclosure, name a concrete heavy case in acceptance criteria. Example:

- Overview stays one scrollbar and stays smooth with a Renovate / Dependabot release-notes body open (for example `home-assistant/core` image updates).

If the design needs nested scroll or pinned chrome, the ticket must say the engineer approved that tradeoff.

## Checklist before you call a UI change done

- [ ] Unbounded content uses a virtualized surface (`TextView::scrollable`, `list`, or `uniform_list`), not fit layout in a scrolling div
- [ ] The pane has one scrollbar unless a recorded tradeoff says otherwise
- [ ] Heavy disclosure starts closed and does not mount its body until open
- [ ] A named heavy fixture is in the ticket acceptance criteria and you have reasoned about it
