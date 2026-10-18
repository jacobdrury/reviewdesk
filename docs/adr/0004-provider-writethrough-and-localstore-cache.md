# Provider write-through and LocalStore cache scope

**Status:** Accepted  
**Date:** 2026-10-18  
**Context:** [Grill Provider write methods and LocalStore cache scope](https://github.com/jacobdrury/reviewdesk/issues/56) on [Day-to-day ready: GitHub review and manage desk](https://github.com/jacobdrury/reviewdesk/issues/46)  
**Depends on:** [ADR-0001](0001-localstore-turso-local-first.md); pending-draft LocalStore policy in [ADR-0003](0003-pending-review-localstore-not-annotations.md) (#54)

## Decision

ADR-0001’s LocalStore-first **read** path stays in force when Provider writeback lands. The UI paints from LocalStore and never calls `Provider` or writes the DB on the hot path. Mutations and cache updates go through an **orchestration layer in the `reviewdesk` app crate** (today’s sync/command path): call named `Provider` write methods, **write-through LocalStore only after Provider success**, then **targeted refetch** of related keys; when that refetch completes it **replaces** the write-through row. No optimistic LocalStore mutations; no full-store wipe on a single mutation.

`Provider` remains **one trait** with Provider-agnostic named read/write methods (not a command enum, not per-surface traits, no GitHub types on the trait). GitHub REST/GraphQL mapping stays in `reviewdesk-github`. Split the overloaded first-slice `list_inbox` (PR search) into **notification list** (Inbox) and **pull-request list** (Pull requests / Views).

## LocalStore inventory

**Cached (survive restart):** notification Inbox rows; Pull requests list rows; PR detail/metadata (including derived CODEOWNERS match summary when Metadata needs it); Patch; Conversation threads; Pipelines/check rollup; Pending review drafts per ADR-0003.

**Not Provider-synced cache (may still live in app persistence):** full CI logs; raw CODEOWNERS file blob (fetch on demand, store derived summary on detail); **Views** (local product data — filters, sort, group-by — persisted in embedded Turso via LocalStore APIs but never fetched from or written through the Provider); credentials (existing `gh`/PAT path — see [auth-github](../auth-github.md)).

**Sync shape:** Inbox notifications and Pull requests lists use snapshot reconcile (upsert + drop absent rows). Detail, Patch, threads, checks, and draft use keyed put/get; list sync never wipes those. Lists fill on startup/interval; surface entities load on open/focus (paint cache first, background refresh if present).

## Rejected alternatives

| Option | Why not |
| --- | --- |
| UI calls `Provider` writes directly | Breaks single writer story; easy to skip LocalStore updates |
| Optimistic LocalStore before Provider ack | Forces rollback paths; refetch-after-success is enough with in-memory in-flight UI |
| `mutate(Command)` / many domain traits | Harder to navigate; GitHub is one identity implementing one seam |
| Fine-grained-PAT-only auth for Inbox | Notifications REST is classic-PAT-only per research |

## Consequences

- Sibling wayfinder tickets name which mutations exist and their UX; this ADR fixes how those methods attach and how cache stays coherent.
- Exact SQL column layouts for new tables land at implement time; the inventory above is the product/architecture bound.
- Auth scopes for the documented `gh`/PAT path live in [docs/auth-github.md](../auth-github.md).
