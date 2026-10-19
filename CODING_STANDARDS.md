# Coding standards

Judgement for the `code-review` skill. Apply every section that the diff touches.

Mechanical rules (fmt, clippy, banned APIs) belong in `moon` tasks and linters, not here.

## Snappy UI

The product should feel immediate. Unbounded content must not lay out or paint the whole document on every scroll frame.

**Zed is the reference** for scroll, list, sticky chrome, and disclosure on GPUI. Match Zed’s patterns (virtualize first, flatten collapsed rows, one vertical scroll owner per pane, sticky as list decoration, horizontal-only nested scroll) unless a ticket records engineer approval. The short “look here in Zed” table and pane rules live in `docs/agents/ui-performance.md`.

### One scroll surface

For Overview, Pull requests, and each vertical region inside a pane:

- Unbounded content scrolls on one virtualized surface (`TextView::scrollable`, `list`, or `uniform_list`).
- Overview is one composed document with one vertical scrollbar (title, meta, and body together).
- Fail a fit-mode `TextView` (or other full layout) inside `overflow_y_scroll` when the content can grow without a bound.
- Fail a pinned header or toggle that shrinks the pane into a nested **vertical** scroller for the rest of the content, unless the PR or ticket records engineer approval.
- Fail nested **vertical** scroll regions (outer div scroll + inner vertical virtualized scroll) without that recorded approval.
- Virtualized children under pinned chrome (Pull requests header, Review tab bar, Files header) must use a bounded `flex_1().min_h_0()` (or equivalent) chain so the scroller gets a max height.

### Files tab (two panes)

The Files tab is a horizontal split: **file list** and **Diff** are two panes, each with its own vertical scrollbar. That is not a nested-scroll violation. Fail diffs that merge them into one scroller without engineer approval on the ticket or PR.

### Disclosure

Heavy disclosure (GitHub `<details>`, Renovate release notes, long collapsed sections):

- Starts closed by default.
- Closed state must not mount or lay out the hidden body.
- Open body should join the parent virtualized surface as normal blocks when possible, not one tall nested element.

### Engineer-approved tradeoffs

Nested **vertical** scroll, pinned chrome that shrinks the content viewport into a nested vertical scroller, eager-mounted disclosure, or fit layout for unbounded bodies is allowed only when the ticket or PR states the engineer approved that tradeoff and why. Horizontal-only overflow inside a row (wide code or tables) while the pane keeps one vertical owner is the normal pattern; see ui-performance.

### Patterns

gpui-kit scroll, disclosure, and the Renovate Overview worked example: `docs/agents/ui-performance.md`.
