# Coding standards

Judgement for the `code-review` skill. Apply every section that the diff touches.

Mechanical rules (fmt, clippy, banned APIs) belong in `moon` tasks and linters, not here.

## Snappy UI

The product should feel immediate. Unbounded content must not lay out or paint the whole document on every scroll frame.

### One scroll surface

For Overview, Diff, Inbox, and any scrollable pane:

- Unbounded content scrolls on one virtualized surface (`TextView::scrollable`, `list`, or `uniform_list`).
- Fail a fit-mode `TextView` (or other full layout) inside `overflow_y_scroll` when the content can grow without a bound.
- Fail a pinned header or toggle that shrinks the pane into a nested scroller for the rest of the content, unless the PR or ticket records engineer approval.
- Fail nested scroll regions (outer div scroll + inner virtualized scroll) without that recorded approval.

### Disclosure

Heavy disclosure (GitHub `<details>`, Renovate release notes, long collapsed sections):

- Starts closed by default.
- Closed state must not mount or lay out the hidden body.
- Open body should join the parent virtualized surface as normal blocks when possible, not one tall nested element.

### Engineer-approved tradeoffs

Nested scroll, pinned chrome that shrinks the content viewport, eager-mounted disclosure, or fit layout for unbounded bodies is allowed only when the ticket or PR states the engineer approved that tradeoff and why.

### Patterns

gpui-kit scroll, disclosure, and the Renovate Overview worked example: `docs/agents/ui-performance.md`.
