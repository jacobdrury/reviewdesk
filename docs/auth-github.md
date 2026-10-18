# GitHub auth and scopes (`gh` / PAT)

Review Desk uses **one github.com identity** via the existing credential path: `GITHUB_TOKEN`, then `GH_TOKEN`, then `gh auth token` (`crates/reviewdesk/src/auth.rs`). Credentials are **never** stored in LocalStore (see [ADR-0001](adr/0001-localstore-turso-local-first.md)).

This doc is the day-to-day readiness checklist for classic PAT / `gh` scopes. It distills research from wayfinder tickets [#47](https://github.com/jacobdrury/reviewdesk/issues/47)–[#50](https://github.com/jacobdrury/reviewdesk/issues/50). In-app OAuth and GitHub App install flows are out of scope for the current map.

## Recommended classic PAT / `gh` setup

For private repositories and full Destination writeback, use a **classic** personal access token (or `gh auth login` yielding equivalent classic scopes) with:

| Scope | Why |
| --- | --- |
| **`repo`** | Pull request read/write (reviews, comments, threads, merge, metadata, checks, Actions re-run). Also resolves notification `subject` URLs for private repos. |
| **`notifications`** | REST notification Inbox (`GET /notifications`, mark read/done, mute). Overlaps some subscription APIs; still required (or covered by `repo`) for the notifications endpoints. |

`repo` alone also satisfies the notifications API’s documented “`notifications` **or** `repo`” requirement. Prefer documenting both so least-privilege readers understand Inbox needs the notifications family.

Public-only prototypes can use `public_repo` instead of `repo` for review/comment write paths, but **Inbox notifications still need classic PAT** support as below.

## Inbox / notifications (hard constraint)

| Fact | Implication |
| --- | --- |
| REST `/notifications` supports **classic PAT only** (not fine-grained PATs, as documented today) | A fine-grained-only token cannot power the notification Inbox |
| Scope: **`notifications` or `repo`** | Classic token must include one of these |
| Resolving private `subject.url` needs **`repo`** | Inbox → open PR in Review Desk needs `repo` for private subjects |

There is no public GraphQL query to list the user’s notification threads; Inbox sync is REST-shaped.

## Fine-grained PAT gaps

Fine-grained tokens can cover much of **Pull requests** / **Contents** / **Actions** / **Metadata** for review, merge, checks, and CODEOWNERS file reads, but:

- They **do not** enumerate `/notifications` — treat notification Inbox as **classic-PAT-required**.
- GraphQL mutations (e.g. auto-merge enable/disable) need equivalent pull-request write permissions; there is no separate “GraphQL scope.”
- Repo setting `allow_auto_merge` may need **Administration** write — out of band from normal review tokens; the app toggles auto-merge on the PR when the repo already allows it.

## Write areas → access (classic)

| Area | Classic access |
| --- | --- |
| Reviews, inline threads, suggestions (as comment body), conversation comments | `repo` / `public_repo` |
| Resolve / unresolve threads (GraphQL) | Same repository write access |
| Labels, assignees, reviewers, milestone, draft↔ready | `repo` / `public_repo` |
| Merge | `repo` / `public_repo` |
| Auto-merge enable/disable | GraphQL only; repository write (PR permissions) |
| Checks rollup read | `repo` (private) |
| Actions re-run | `repo` |
| CODEOWNERS raw file + changed paths | Contents / pull files via `repo` |
| Notification triage | Classic `notifications` or `repo` |

## Operator checklist

1. `gh auth login` (classic scopes) or export `GITHUB_TOKEN` / `GH_TOKEN` with the table above.
2. Confirm Inbox works only with a classic token; fine-grained-only setups get Pull requests / Review pane reads-writes where permitted, not notification REST.
3. Do not paste tokens into LocalStore, config files in the repo, or issue trackers.
