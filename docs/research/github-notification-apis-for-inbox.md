# GitHub notification APIs for an Inbox

Research for [issue #47](https://github.com/jacobdrury/reviewdesk/issues/47) (parent map [#46](https://github.com/jacobdrury/reviewdesk/issues/46)). Facts only; no product decisions.

Research date: 2026-10-18. Primary sources: [REST notifications](https://docs.github.com/en/rest/activity/notifications), [About notifications](https://docs.github.com/en/account-and-profile/managing-subscriptions-and-notifications-on-github/setting-up-notifications/about-notifications), [Inbox filters](https://docs.github.com/en/subscriptions-and-notifications/reference/inbox-filters), [OAuth scopes](https://docs.github.com/en/apps/oauth-apps/building-oauth-apps/scopes-for-oauth-apps), [Issue/PR search qualifiers](https://docs.github.com/en/search-github/searching-on-github/searching-issues-and-pull-requests), [REST search](https://docs.github.com/en/rest/search/search), [REST watching](https://docs.github.com/en/rest/activity/watching), [GraphQL Activity reference](https://docs.github.com/en/graphql/reference/activity), live GraphQL introspection via `gh api graphql` (2026-10-18).

## Summary

| Need | Best API | Caveat |
| --- | --- | --- |
| GitHub.com notification inbox parity (threads, read/done, reasons) | REST `GET /notifications` (+ thread mutations) | **Classic PAT only**; scopes `notifications` or `repo` |
| Open PRs where you are a requested reviewer (Review Desk v1 Inbox) | GraphQL `search` or REST `GET /search/issues` | 30 searches/min; no read/done; state snapshot not “what changed” |
| @mentions, comments, merge state as **events** | Notifications REST (`reason`) or per-issue timeline | Notifications aggregate by **thread**; timeline is per-PR |
| CI failed on watched PRs | Notifications (`reason:ci-activity` / `ci_activity`) if user opted in | Not the same as `status:failure` search; REST reason text emphasizes runs **you triggered** |
| Subscribe / mute thread or repo | REST thread + repo subscription endpoints; GraphQL `updateSubscription` | “Saved” and some inbox UI filters have **no** public write API |

There is **no** GraphQL query in the public schema to list the authenticated user’s notification threads (verified by schema introspection). GraphQL helps with **search** and **subscription state** on issues/PRs/repos, not with mirroring github.com/notifications.

---

## 1. REST: notification threads (canonical inbox API)

### Authentication and scopes

- Official note: notification REST endpoints **only support personal access tokens (classic)** — not fine-grained PATs as documented today. ([REST notifications — About](https://docs.github.com/en/rest/activity/notifications#about-github-notifications))
- Required OAuth/classic scope: **`notifications`** **or** **`repo`**. ([same](https://docs.github.com/en/rest/activity/notifications#about-github-notifications))
- To resolve `subject.url` / `latest_comment_url` into full issue, pull request, or commit records, **`repo`** scope is required when those subjects are in private repositories. ([same](https://docs.github.com/en/rest/activity/notifications#about-github-notifications))
- OAuth `notifications` scope grants: read notifications; mark threads read; watch/unwatch repos; read/write/delete thread subscriptions. ([OAuth scopes](https://docs.github.com/en/apps/oauth-apps/building-oauth-apps/scopes-for-oauth-apps))

Fine-grained PAT permission matrices checked via [endpoints for fine-grained tokens](https://docs.github.com/en/rest/authentication/endpoints-available-for-fine-grained-personal-access-tokens) do not enumerate `/notifications` — consistent with classic-only support on the notifications page.

### Mental model

- One **thread** = ongoing discussion on an **issue**, **pull request**, or **commit** (not each comment separately). ([REST notifications — About](https://docs.github.com/en/rest/activity/notifications#about-github-notifications))
- Threads sort by **`updated_at`** (most recent first). ([List notifications](https://docs.github.com/en/rest/activity/notifications#list-notifications-for-the-authenticated-user))
- **`reason`** on a thread reflects the **latest** qualifying event’s category; it can change over time (e.g. `author` → `mention`). ([About notification reasons](https://docs.github.com/en/rest/activity/notifications#about-notification-reasons))

### Polling model

Designed for conditional polling:

- Response includes **`Last-Modified`**; send **`If-Modified-Since`** on the next poll.
- **`304 Not Modified`** when nothing changed — documented as **not consuming** the current rate limit budget for that check. ([REST notifications — About](https://docs.github.com/en/rest/activity/notifications#about-github-notifications))
- Obey **`X-Poll-Interval`** (seconds; may increase under load). ([same](https://docs.github.com/en/rest/activity/notifications#about-github-notifications))

General REST rate limits still apply for non-304 responses ([Rate limits](https://docs.github.com/en/rest/using-the-rest-api/rate-limits-for-the-rest-api)); notifications have no separate documented primary limit beyond that.

### List endpoints and query parameters

| Method | Path | Role |
| --- | --- | --- |
| `GET` | `/notifications` | All threads for the authenticated user |
| `GET` | `/repos/{owner}/{repo}/notifications` | Threads for one repo |

Shared query parameters ([list user](https://docs.github.com/en/rest/activity/notifications#list-notifications-for-the-authenticated-user), [list repo](https://docs.github.com/en/rest/activity/notifications#list-repository-notifications-for-the-authenticated-user)):

| Parameter | Meaning |
| --- | --- |
| `all` | If `true`, include read threads (default `false` = unread only) |
| `participating` | If `true`, only threads where the user participates or is @mentioned |
| `since` / `before` | ISO 8601 bounds on **`updated_at`** |
| `page` / `per_page` | Pagination (`per_page` max **50** on `/notifications`) |

### Thread payload shape (`Thread`)

Documented fields on list/get thread ([Thread schema](https://docs.github.com/en/rest/activity/notifications#list-notifications-for-the-authenticated-user)):

| Field | Type | Notes |
| --- | --- | --- |
| `id` | string | Thread id (path param docs call it `thread_id` integer — use value from `id`) |
| `repository` | `Minimal Repository` | Full repo metadata blob |
| `subject` | object | See below |
| `reason` | string | Latest reason enum (see §3) |
| `unread` | boolean | Read state |
| `updated_at` | string | Thread activity time |
| `last_read_at` | string or null | When marked read |
| `url` | string | API URL for this thread |
| `subscription_url` | string | Thread subscription API URL |

**`subject`** (minimal pointer, not full PR/issue body):

| Field | Meaning |
| --- | --- |
| `title` | Thread title |
| `type` | Subject kind (e.g. PullRequest, Issue — use `subject.url` for authoritative type) |
| `url` | API URL of the subject |
| `latest_comment_url` | API URL of latest comment on the thread |

Clients must follow `subject.url` / GraphQL node lookup for merge state, checks, review comments, etc.

### Mutations: read, done, subscribe

| Action | REST | HTTP | Notes |
| --- | --- | --- | --- |
| Mark **all** read | `PUT /notifications` | 202/205 | Body: optional `last_read_at` — only older than this cutoff marked read |
| Mark **repo** read | `PUT /repos/{owner}/{repo}/notifications` | 202/205 | Same `last_read_at` semantics |
| Mark **one thread** read | `PATCH /notifications/threads/{thread_id}` | 205 | Same as clicking a row in github.com/notifications |
| Mark **done** (remove from inbox) | `DELETE /notifications/threads/{thread_id}` | 204 | Same as “Done” in the web inbox ([About notifications](https://docs.github.com/en/account-and-profile/managing-subscriptions-and-notifications-on-github/setting-up-notifications/about-notifications)) |
| Get thread subscription | `GET /notifications/threads/{thread_id}/subscription` | 200 | `subscribed`, `ignored`, `reason`, … |
| Mute / subscribe thread | `PUT /notifications/threads/{thread_id}/subscription` | 200 | Body: `ignored` (boolean) — ignore future updates until comment/@mention |
| Unsubscribe thread | `DELETE /notifications/threads/{thread_id}/subscription` | 204 | Mutes until participation; watching repo may still notify |

**Not exposed on REST** (web inbox only, per docs gap):

- **Saved** (`is:saved` in inbox UI — [Inbox filters](https://docs.github.com/en/subscriptions-and-notifications/reference/inbox-filters)) — no mark-saved endpoint in [REST notifications](https://docs.github.com/en/rest/activity/notifications).
- Inbox **custom filter** queries (`reason:`, `is:done`, etc.) — documented for github.com/notifications UI, not as a public “search notifications” API.

### Repository watch (related)

Separate from threads but affects volume of `subscribed` / `state_change` / CI notifications:

| Method | Path | Purpose |
| --- | --- | --- |
| `GET/PUT/DELETE` | `/repos/{owner}/{repo}/subscription` | Watch repo; `ignored: true` blocks all repo notifications |
| `GET` | `/user/subscriptions` | Repositories the user watches |

([REST watching](https://docs.github.com/en/rest/activity/watching))

July 2026 access restrictions: listing other users’ watches/subscribers is limited; `GET /users/{username}/subscriptions` is deprecated toward empty responses. ([REST watching — New access restrictions](https://docs.github.com/en/rest/activity/watching))

---

## 2. Notification `reason` values vs Inbox categories

REST [reason table](https://docs.github.com/en/rest/activity/notifications#about-notification-reasons) and inbox [reason: filters](https://docs.github.com/en/subscriptions-and-notifications/reference/inbox-filters) align on the main review-workflow cases:

| Inbox-style event | REST `reason` | Inbox filter |
| --- | --- | --- |
| Review requested (user or team) | `review_requested` | `reason:review-requested` |
| @mention | `mention` | `reason:mention` |
| Team @mention | `team_mention` | `reason:team-mention` |
| Comment on thread you participate in | `comment` | (participating / `reason:comment`) |
| Your PR/issue activity | `author` | `reason:author` |
| Merge, close, reopen | `state_change` | `reason:state-change` |
| Watching repo activity | `subscribed` | — |
| CI / Actions | `ci_activity` | `reason:ci-activity` |
| Deployment approval | `approval_requested` | — |
| Assigned | `assign` | `reason:assign` |

**Review comments** (inline on the diff) do not get their own `reason`; they bump the PR **thread** and typically surface as `comment`, `mention`, `review_requested`, `author`, or `subscribed` depending on participation.

**Auto-merge**: no separate `reason`. When merge completes, expect **`state_change`** on the PR thread ([reason:state-change](https://docs.github.com/en/subscriptions-and-notifications/reference/inbox-filters)). Auto-merge enable/disable is ordinary PR/issue activity on the same thread.

**CI failures on watched PRs**:

- Inbox filter copy: `reason:ci-activity` = “CI update, such as a new workflow run status”. ([Inbox filters](https://docs.github.com/en/subscriptions-and-notifications/reference/inbox-filters))
- REST reason table: `ci_activity` = “A GitHub Actions workflow run **that you triggered** was completed.” ([REST notifications](https://docs.github.com/en/rest/activity/notifications#about-notification-reasons))
- User setting: under **Actions** on notification settings, opt in to GitHub and/or email; optional **“Only notify for failed workflows”** for watched repos with Actions. ([Managing GitHub Actions notifications](https://docs.github.com/en/subscriptions-and-notifications/how-tos/managing-github-actions-notifications))

So “CI failed on a PR I watch but did not trigger” depends on **notification settings**, not only API shape; the API still delivers it as a PR/check-related thread with `ci_activity` when GitHub generates a notification.

---

## 3. GraphQL

### Listing notifications

Public schema introspection (2026-10-18, `gh api graphql`) shows **no** root query fields containing `notification` for listing threads. Inbox listing is **REST-only** for third-party clients.

[GraphQL changelog 2024](https://docs.github.com/en/graphql/overview/changelog/2024) once listed `markNotificationAsDone` on `Mutation`; that mutation **does not** appear in the current public schema (introspection returns no `MarkNotificationAsDoneInput`). Treat **mark done** as REST `DELETE /notifications/threads/{id}` unless GitHub documents GraphQL again.

### Subscriptions (mute/watch) on entities

[Activity reference — `Subscribable` / `SubscribableThread`](https://docs.github.com/en/graphql/reference/activity): issues, pull requests, repos, etc. expose viewer subscription state.

- Mutation **`updateSubscription`** with `UpdateSubscriptionInput`: `subscribableId`, `state` (`SubscriptionState`: `SUBSCRIBED`, `UNSUBSCRIBED`, `IGNORED`). (Schema introspection 2026-10-18.)
- This adjusts **future** notifications for that entity; it does **not** list or triage existing inbox rows.

### Search (current Review Desk approach)

`search(query: $q, type: ISSUE, …)` returns issues **and** pull requests. Qualifiers useful for inbox-like **slices** ([Searching issues and PRs](https://docs.github.com/en/search-github/searching-on-github/searching-issues-and-pull-requests)):

| Qualifier | Use |
| --- | --- |
| `is:open is:pr review-requested:@me` | Pending review requests (Review Desk default query in `reviewdesk-github`) |
| `user-review-requested:@me` | Direct review requests only |
| `review-involves:@me` | Includes completed reviews (stays in results) |
| `author:@me` / `commenter:@me` / `mentions:@me` / `involves:@me` | Participation slices |
| `team-review-requested:ORG/TEAM` | Team queue |
| `status:failure` / `status:success` | Commit/check status on PRs (search index, not notification stream) |
| `merged:`, `updated:` | Merge/time filters |

REST equivalent: `GET /search/issues?q=…` — same query string, **30 requests/minute** authenticated, up to **1000** results, `incomplete_results` possible. ([REST search — Rate limit](https://docs.github.com/en/rest/search/search#rate-limit))

GraphQL/REST search **cannot**:

- Mark read/done/unsubscribe at notification granularity
- Return `reason` or unread state
- Represent “new comment since last visit” without local cursor + extra fetches

---

## 4. Workarounds when notifications API is insufficient

| Approach | Delivers | Limits |
| --- | --- | --- |
| **Issue/PR search** | Open PR set matching role (reviewer, author, mention) | Rate limit; no triage; approximate “inbox” |
| **Per-PR timeline** | `review_requested`, `merged`, `head_ref_force_pushed`, etc. ([Issue events REST](https://docs.github.com/en/rest/issues/events)) | Requires known PR; not global inbox |
| **Checks API** | `GET /repos/.../commits/{ref}/check-runs`, PR head SHA polling | Per repo/PR; no user-level fan-in |
| **Public events** | `GET /users/{user}/events`, `/events` | Not notification-shaped; public events API warns **30s–6h** latency ([REST events](https://docs.github.com/en/rest/activity/events)) |
| **Feeds** | Atom/JSON timelines via `GET /feeds` | Legacy token model for private feeds; not a triage inbox ([REST feeds](https://docs.github.com/en/rest/activity/feeds)) |
| **Webhooks** | `pull_request`, `pull_request_review`, `check_run`, `check_suite`, etc. ([Webhook events](https://docs.github.com/en/webhooks/webhook-events-and-payloads)) | Per installation/repo; app must aggregate and store user-visible inbox locally |

Review Desk today implements Inbox via **GraphQL search**, not notifications REST (`crates/reviewdesk-github/src/query.rs`, `client.rs`).

---

## 5. Gaps forcing UI parity work or hybrid designs

1. **Classic PAT only** for notifications — blocks fine-grained-only and complicates least-privilege apps ([REST notifications](https://docs.github.com/en/rest/activity/notifications#about-github-notifications)).
2. **No GraphQL list** for threads; no documented GraphQL “mark done” in current schema.
3. **Thread granularity** — one row per PR/issue; multiple review rounds collapse unless `updated_at` / local diffing is used.
4. **Thin `subject`** — no check conclusion, merge queue, or auto-merge flag in the notification payload; extra API round-trips required.
5. **Saved** state — UI/query `is:saved` ([Inbox filters](https://docs.github.com/en/subscriptions-and-notifications/reference/inbox-filters)) without REST counterpart.
6. **Done vs read** — REST supports both; **search-based Inbox** has no GitHub-side done bit (local-only in LocalStore per [ADR-0001](docs/adr/0001-localstore-turso-local-first.md)).
7. **CI semantics** — notification `ci_activity` vs search `status:failure` vs Checks API measure different things (see §2).
8. **Participating filter** — REST `participating=true` approximates “involved” but is not identical to search `involves:@me` semantics.
9. **Retention** — non-saved notifications age out after **3 months** on GitHub ([About notifications — retention](https://docs.github.com/en/account-and-profile/managing-subscriptions-and-notifications-on-github/setting-up-notifications/about-notifications)); long-lived local cache may diverge.
10. **Review-requested search** drops PRs after you submit a review ([review-requested qualifier](https://docs.github.com/en/search-github/searching-on-github/searching-issues-and-pull-requests)); notifications may still update the thread for other reasons.

---

## 6. Mapping to Review Desk “notification-shaped Inbox”

For a **github.com/notifications-like** experience (read/done, reasons, @mentions, CI on subscribed PRs, merge alerts):

- **Primary source**: REST notifications polling + thread mutations + optional `subject` hydration (GraphQL node or REST pulls/issues).
- **Complement**: keep search for “open PRs I must review” queue (current product Inbox definition in [GLOSSARY.md](../../GLOSSARY.md)) where GitHub has no “done” concept.

For **auth planning**: assume **`notifications` + `repo`** (classic) or OAuth app with equivalent scopes; plan for classic token UX until GitHub documents fine-grained support on `/notifications`.

---

## Sources (quick index)

| Topic | URL |
| --- | --- |
| REST notifications | https://docs.github.com/en/rest/activity/notifications |
| About notifications (done, saved, retention) | https://docs.github.com/en/account-and-profile/managing-subscriptions-and-notifications-on-github/setting-up-notifications/about-notifications |
| Inbox filter reference | https://docs.github.com/en/subscriptions-and-notifications/reference/inbox-filters |
| Actions notification settings | https://docs.github.com/en/subscriptions-and-notifications/how-tos/managing-github-actions-notifications |
| OAuth `notifications` scope | https://docs.github.com/en/apps/oauth-apps/building-oauth-apps/scopes-for-oauth-apps |
| Issue/PR search qualifiers | https://docs.github.com/en/search-github/searching-on-github/searching-issues-and-pull-requests |
| REST search limits | https://docs.github.com/en/rest/search/search |
| REST watching | https://docs.github.com/en/rest/activity/watching |
| GraphQL Activity (Subscribable) | https://docs.github.com/en/graphql/reference/activity |
