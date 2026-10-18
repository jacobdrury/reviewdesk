# ADR-0001: Local-first persistence with embedded Turso (LocalStore)

**Status:** Accepted  
**Date:** 2026-10-18  
**Context:** [Issue #25 — Local-first persistence with Turso (LocalStore)](https://github.com/jacobdrury/reviewdesk/issues/25)

## Decision

Review Desk persists Inbox, pull-request metadata, and cached Patches locally through a single **`LocalStore`** seam in `reviewdesk-core`. The default implementation is **embedded Turso Database** (in-process, SQLite-compatible) in `reviewdesk-localstore`. The UI reads from LocalStore on the hot path; **Providers** sync in the background and write into LocalStore. No Turso Cloud account or remote database is required for local features.

## Rationale

- **Snappy UI:** Listing and opening pull requests should not wait on network round-trips when data is already on disk. Background sync updates rows; the shell renders from local state first (see cache-miss behavior below).
- **Relational local cache:** Future Linear-style filter/group/sort views need SQL over tables, not a key-value blob store.
- **One seam:** UI and sync both depend on the async `LocalStore` trait. Only the Turso crate talks to the engine; `reviewdesk-github` stays Provider-only.
- **Desktop shape:** Managed Postgres and Turso Cloud sync are the wrong default for an offline-capable desktop client in this slice.

## Architecture

```
┌─────────────┐     read (hot path)      ┌──────────────┐
│  gpui UI    │ ───────────────────────► │  LocalStore  │
│  (Inbox,    │                          │  (trait)     │
│   Overview, │     write (sync)         └──────┬───────┘
│   Files)    │ ◄──────────────────────────────┤
└─────────────┘                                │
                                               ▼
                                    ┌──────────────────────┐
                                    │ TursoLocalStore      │
                                    │ (embedded DB file)   │
                                    └──────────────────────┘
        ┌──────────────┐
        │ Provider     │ ── background fetch ──► upsert / reconcile
        │ (e.g. GitHub)│
        └──────────────┘
```

- **Startup:** Open LocalStore at a per-user app data path, run ordered idempotent migrations, then kick off background Inbox (and related) sync.
- **Cache miss:** Show empty or stale local rows immediately; trigger sync. Do not block first paint on Provider I/O.
- **Writers:** Provider sync owns Provider-sourced tables. Serialize writes on a connection per Turso’s same-connection writer rules.
- **Secrets:** Provider credentials stay in the existing credentials path—not in Inbox/PR cache tables.

## Schema and portability (SQLite escape hatch)

- Use **vanilla SQLite-compatible** types and SQL. Avoid experimental Turso-only features unless a later ADR opts in.
- Prefer WAL where supported.
- Keep large **Patch** blobs in separate storage from Inbox listing rows so list queries stay fast.
- The on-disk file and schema should remain conceptually portable: if embedded Turso becomes unsuitable, we can target **plain SQLite** (for example via `rusqlite`) with the same relational model rather than re-litigating RocksDB vs Postgres.

Known Turso compatibility gaps that are acceptable for this product (see upstream `COMPAT.md` when in doubt): no `WITH RECURSIVE`, limited window functions, no INSTEAD OF triggers on views, and similar.

## Rejected alternatives

| Option | Why not (for this slice) |
| --- | --- |
| RocksDB | No relational query model for views |
| Postgres (managed / local server) | Wrong operational shape for a desktop local-first client |
| Turso Cloud / multi-device sync | Out of scope; not required for offline local use |
| Provider-as-hot-path for Inbox | Network latency and rate limits on every navigation |

## UI performance

Local persistence does **not** relax virtualization rules. The Inbox still feeds a virtualized list (`uniform_list` or equivalent); loading from disk is not an excuse to lay out unbounded rows every frame. See [UI performance](../agents/ui-performance.md#local-persistence-and-inbox).

## Consequences

- New workspace crate `reviewdesk-localstore` for open, migrate, and (as tickets land) query/upsert APIs.
- Tests exercise LocalStore through its public async API with a temp database path.
- Annotations remain in-memory until a later issue changes that (per product glossary).

## References

- Parent spec: [issue #25](https://github.com/jacobdrury/reviewdesk/issues/25)
- Agent UI guidance: [docs/agents/ui-performance.md](../agents/ui-performance.md)
- Domain vocabulary: [GLOSSARY.md](../../GLOSSARY.md)
