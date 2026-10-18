## Agent skills

### Issue tracker

Issues live in GitHub Issues for `jacobdrury/reviewdesk` (via `gh`). See `docs/agents/issue-tracker.md`.

### Triage labels

Default vocabulary: `needs-triage`, `needs-info`, `ready-for-agent`, `ready-for-human`, `wontfix`. See `docs/agents/triage-labels.md`.

### Domain docs

Single-context: root `GLOSSARY.md` and `docs/adr/`. See `docs/agents/domain.md`.

### Local persistence

Local-first Pull requests and PR cache: UI reads **LocalStore**; Providers sync in the background; embedded Turso (no Turso Cloud required). Read [ADR-0001](docs/adr/0001-localstore-turso-local-first.md) before changing persistence, sync, or Provider hot-path reads.

### UI performance

Snappy scroll, layout, and disclosure for Overview, Diff, Pull requests, and any scrollable pane. Read `docs/agents/ui-performance.md` before making those choices — it names **Zed** as the GPUI reference (look-here table, Files dual-pane policy, bounded flex under pinned chrome).
