# GitHub review threads, reviews, and suggestion blocks (primary sources)

Facts only. No product decisions. Research for [issue #49](https://github.com/jacobdrury/reviewdesk/issues/49) (parent map [#46](https://github.com/jacobdrury/reviewdesk/issues/46)).

Research date: 2026-10-18. Sources are [GitHub REST](https://docs.github.com/en/rest), [GitHub GraphQL](https://docs.github.com/en/graphql/reference/pulls), and GitHub Help articles linked below.

## 1. Vocabulary (GitHub)

| Concept | Meaning | Source |
| --- | --- | --- |
| Issue comment | Timeline comment on the PR as a whole (same API surface as issues) | [Working with comments](https://docs.github.com/en/rest/guides/working-with-comments) |
| Pull request review | A review submission (`APPROVE`, `REQUEST_CHANGES`, `COMMENT`, or `PENDING`) optionally with a body and inline comments | [REST: Pull request reviews](https://docs.github.com/en/rest/pulls/reviews) |
| Review comment | Inline comment on a portion of the unified diff (line or file) | [REST: Pull request review comments](https://docs.github.com/en/rest/pulls/comments) |
| Review thread | Comments grouped on one diff anchor; can be resolved/unresolved | GraphQL `PullRequestReviewThread` in [GraphQL: Pull requests](https://docs.github.com/en/graphql/reference/pulls) |
| Suggestion block | Markdown fenced block `` ```suggestion `` … `` ``` `` in a review comment body proposing replacement text for the anchored lines | [Reviewing proposed changes](https://docs.github.com/en/pull-requests/how-tos/review-pull-requests/reviewing-proposed-changes-in-a-pull-request), [Incorporating feedback](https://docs.github.com/en/pull-requests/how-tos/review-pull-requests/incorporating-feedback-in-your-pull-request) |

GitHub REST treats every pull request as an issue (`issue_number` == `pull_number` for PR-scoped issue endpoints). Review-inline and review-summary operations use the pulls endpoints or GraphQL pull types instead.

**Do not confuse** REST `/issues/{issue_number}/suggestions` with code suggestion blocks: that API is for agent-proposed **issue metadata** changes (labels, assignees, etc.) and explicitly does **not** support pull requests. See [REST: List issue suggestions](https://docs.github.com/en/rest/issues/issues#list-issue-suggestions).

---

## 2. Read APIs

### 2.1 Conversation (timeline + threads)

| Need | REST | GraphQL |
| --- | --- | --- |
| PR body / metadata | `GET /repos/{owner}/{repo}/pulls/{pull_number}` | `PullRequest` fields on `repository.pullRequest(number:)` |
| Issue-style timeline comments | `GET /repos/{owner}/{repo}/issues/{issue_number}/comments` | `PullRequest.comments` connection |
| All inline review comments (flat) | `GET /repos/{owner}/{repo}/pulls/{pull_number}/comments` | `PullRequestReviewComment` via reviews or threads |
| **Threads** with resolve/outdated/line anchor | No first-class thread list; reconstruct via `in_reply_to_id` on flat comments | `PullRequest.reviewThreads(first/last, after/before)` → `PullRequestReviewThread` |
| Review summaries | `GET /repos/{owner}/{repo}/pulls/{pull_number}/reviews` | `PullRequest.reviews(states: …)` |
| Per-user latest submitted review | Filter REST reviews | `PullRequest.viewerLatestReview`, `latestReviews` (excludes pending) |
| Merge/review gate | `GET …/pulls/{n}` (`mergeable`, etc.) | `reviewDecision`, `mergeStateStatus` |

**Thread fields (GraphQL only on `PullRequestReviewThread`):** `isResolved`, `isOutdated`, `isCollapsed`, `path`, `line`, `startLine`, `originalLine`, `originalStartLine`, `diffSide`, `startDiffSide`, `subjectType`, `viewerCanReply`, `viewerCanResolve`, `viewerCanUnresolve`, `resolvedBy`, nested `comments`. Source: [GraphQL: PullRequestReviewThread](https://docs.github.com/en/graphql/reference/pulls#pullrequestreviewthread).

**Comment fields useful for Diff:** GraphQL `PullRequestReviewComment` exposes `outdated`, `line`, `startLine`, `originalLine`, `originalStartLine`, `diffHunk`, `state` (`PENDING` \| `SUBMITTED`), `subjectType`, `replyTo`. REST review comment objects expose `line`, `start_line`, `side`, `start_side`, `subject_type`, `diff_hunk`, `in_reply_to_id`, but **not** `outdated` or thread `isResolved` in the documented REST schema. Sources: [GraphQL: PullRequestReviewComment](https://docs.github.com/en/graphql/reference/pulls#pullrequestreviewcomment), [REST: review comment schema](https://docs.github.com/en/rest/pulls/comments).

**Pagination:** GraphQL connections require `first` or `last` between 1 and 100 per page. Source: [Using pagination in the GraphQL API](https://docs.github.com/en/graphql/guides/using-pagination-in-the-graphql-api). REST review comment lists default to 30, max `per_page` 100.

### 2.2 Pending review (viewer)

| Need | REST | GraphQL |
| --- | --- | --- |
| Find viewer’s pending review | List reviews; `state` == `PENDING`, no `submitted_at` | `PullRequest.reviews(states: [PENDING])` filtered by viewer, or inspect comments with `state: PENDING` |
| Pending inline comments | Comments with `pull_request_review_id` set to pending review id; GraphQL `PullRequestReviewComment.state` == `PENDING` | Same via `reviewThreads` / review `comments` |

UI behavior (not REST-specific): before submit, line comments are pending and visible only to the author until the review is submitted. Source: [Reviewing proposed changes — Starting a review](https://docs.github.com/en/pull-requests/how-tos/review-pull-requests/reviewing-proposed-changes-in-a-pull-request).

### 2.3 Suggestion blocks (read)

- **Storage:** Suggestion text lives in the review comment `body` as markdown (`` ```suggestion `` fence). No separate suggestion ID for code suggestions in REST/GraphQL pull APIs.
- **Capability hint:** GraphQL `PullRequest.viewerCanApplySuggestion` indicates whether the viewer may apply suggestions in the UI (write access / fork maintainer rules per Help). Source: [GraphQL: PullRequest](https://docs.github.com/en/graphql/reference/pulls#pullrequest).
- **Apply in product UI:** Documented only as web UI actions (“Commit suggestion”, batch “Commit suggestions”). Source: [Incorporating feedback](https://docs.github.com/en/pull-requests/how-tos/review-pull-requests/incorporating-feedback-in-your-pull-request).

---

## 3. Write APIs

### 3.1 Issue / conversation comments

| Action | REST |
| --- | --- |
| Create timeline comment | `POST /repos/{owner}/{repo}/issues/{issue_number}/comments` |
| Edit / delete | `PATCH` / `DELETE …/issues/comments/{comment_id}` |

Source: [REST: Issue comments](https://docs.github.com/en/rest/issues/comments).

### 3.2 Inline review comments (published immediately)

| Action | REST |
| --- | --- |
| Create on diff | `POST /repos/{owner}/{repo}/pulls/{pull_number}/comments` |
| Reply (thread) | `POST …/pulls/{pull_number}/comments/{comment_id}/replies` — **top-level comment id only**; replies-to-replies not supported |
| Edit / delete | `PATCH` / `DELETE …/pulls/comments/{comment_id}` |

Required anchor (line comments): `commit_id`, `path`, `body`, plus `line` and `side`; for multi-line add `start_line` and `start_side`. File-level: `subject_type: file`. Using `in_reply_to` on create ignores other body fields except `body`. Source: [REST: Create a review comment](https://docs.github.com/en/rest/pulls/comments#create-a-review-comment-for-a-pull-request).

**Deprecation:** `position` (diff hunk index) is “closing down”; prefer `line` / `start_line`. Same note on [Create review](https://docs.github.com/en/rest/pulls/reviews#create-a-review-for-a-pull-request).

**Notifications / abuse:** Create endpoints trigger notifications and may hit secondary rate limits. Source: REST docs above; [Rate limits](https://docs.github.com/en/rest/using-the-rest-api/rate-limits-for-the-rest-api).

### 3.3 Pending review workflow (batch inline + submit)

| Step | REST | GraphQL |
| --- | --- | --- |
| Start pending review | `POST …/pulls/{n}/reviews` with **`event` omitted** → `state: PENDING` | `addPullRequestReview` with `event` omitted / pending flow |
| Add inline to pending | `POST …/pulls/{n}/comments` ( attaches to pending review in practice) or initial `comments[]` on create review | `addPullRequestReviewThread` (**“Adds a new thread to a pending Pull Request Review”**), `addPullRequestReviewThreadReply` |
| Add many comments in one call | `comments[]` on create review (supports `line`, `side`, `start_line`, `start_side`) | `addPullRequestReview` `threads: [DraftPullRequestReviewThread!]` (preferred over deprecated `comments` with `position`) |
| Update pending summary body | `PUT …/pulls/{n}/reviews/{review_id}` (body only) | `updatePullRequestReview` |
| Discard pending | `DELETE …/pulls/{n}/reviews/{review_id}` (pending only) | `deletePullRequestReview` |
| Submit | `POST …/pulls/{n}/reviews/{review_id}/events` with required `event`: `APPROVE`, `REQUEST_CHANGES`, or `COMMENT` | `submitPullRequestReview` with `event: PullRequestReviewEvent!` |
| Submit in one shot | `POST …/reviews` with `event` set and optional `comments[]` / `body` | `addPullRequestReview` with `event: APPROVE` \| `REQUEST_CHANGES` \| `COMMENT` and `threads` |

Sources: [REST: Pull request reviews](https://docs.github.com/en/rest/pulls/reviews), [GraphQL: addPullRequestReview](https://docs.github.com/en/graphql/reference/pulls#addpullrequestreview), [addPullRequestReviewThread](https://docs.github.com/en/graphql/reference/pulls#addpullrequestreviewthread), [submitPullRequestReview](https://docs.github.com/en/graphql/reference/pulls#submitpullrequestreview).

**GraphQL deprecations (still documented with removal notices):** `addPullRequestReviewComment` and diff-relative `position` / `DraftPullRequestReviewComment.position` — GitHub directs callers to `addPullRequestReviewThread`, `addPullRequestReviewThreadReply`, and `threads` on `addPullRequestReview`. Source: [GraphQL: AddPullRequestReviewCommentInput](https://docs.github.com/en/graphql/reference/pulls#addpullrequestreviewcommentinput).

**Pending review constraint:** Help describes a single in-progress review session (“If you have already started a review…”). The API exposes delete-pending and submit flows consistent with one draft review per user per PR; duplicate create attempts may return `422`. Sources: [Reviewing proposed changes](https://docs.github.com/en/pull-requests/how-tos/review-pull-requests/reviewing-proposed-changes-in-a-pull-request), [Delete a pending review](https://docs.github.com/en/rest/pulls/reviews#delete-a-pending-review-for-a-pull-request).

### 3.4 Resolve / unresolve threads

| Action | REST | GraphQL |
| --- | --- | --- |
| Resolve | **No documented REST endpoint** | `resolveReviewThread(input: { threadId })` |
| Unresolve | **No documented REST endpoint** | `unresolveReviewThread(input: { threadId })` |

Optional `resolutionReason` on resolve (Copilot code review threads). Sources: [GraphQL: resolveReviewThread](https://docs.github.com/en/graphql/reference/pulls#resolvereviewthread), [unresolveReviewThread](https://docs.github.com/en/graphql/reference/pulls#unresolvereviewthread).

### 3.5 Review decisions (approve / request changes / comment)

| Action | REST | GraphQL |
| --- | --- | --- |
| Approve / request changes / comment (no inline) | `POST …/reviews` with `event` + `body` when required | `addPullRequestReview` with `event` |
| Dismiss someone else’s review | `PUT …/reviews/{review_id}/dismissals` | `dismissPullRequestReview` |

`REQUEST_CHANGES` and `COMMENT` require review `body` on REST create when `event` is set. Source: [Create a review](https://docs.github.com/en/rest/pulls/reviews#create-a-review-for-a-pull-request).

Decision semantics: [Pull request reviews](https://docs.github.com/en/pull-requests/reference/pull-request-reviews).

### 3.6 Suggestion blocks (write / apply)

| Action | Official API |
| --- | --- |
| **Propose** suggestion | Include `` ```suggestion `` block in review comment `body` (same as any inline comment create path above) |
| **Apply** suggestion (GitHub-native commit on PR branch) | **Not documented** as REST or GraphQL mutation; UI-only batch/single commit per [Incorporating feedback](https://docs.github.com/en/pull-requests/how-tos/review-pull-requests/incorporating-feedback-in-your-pull-request) |
| **Workaround** | Client applies patch locally (parse fence + line range from `diff_hunk` / line fields) and pushes via [Git data API](https://docs.github.com/en/rest/git) — outside GitHub’s “apply suggestion” UI contract |

Multi-line suggestions: select line range in UI before inserting block; API multi-line anchors use `start_line` / `start_side` + `line` / `side`. Sources: [Reviewing proposed changes](https://docs.github.com/en/pull-requests/how-tos/review-pull-requests/reviewing-proposed-changes-in-a-pull-request), [REST: multi-line example](https://docs.github.com/en/rest/pulls/comments#create-a-review-comment-for-a-pull-request).

---

## 4. GraphQL vs REST (tradeoffs)

| Topic | REST | GraphQL |
| --- | --- | --- |
| **Threads + resolve state** | Flat comments; no resolve/unresolve | Native `reviewThreads`, `isResolved`, `isOutdated`, viewer capability flags |
| **Outdated comments** | Not in documented review-comment schema | `PullRequestReviewComment.outdated`, `PullRequestReviewThread.isOutdated` |
| **Anchoring** | Mature `line` / `side` / `start_*`; `position` deprecated | Same line-based model on threads; deprecated `position` on old mutations |
| **Pending review** | Explicit review id + `/events` submit | `addPullRequestReviewThread` requires pending review; `submitPullRequestReview` |
| **Batch pending comments** | `comments[]` on create review | `threads[]` on `addPullRequestReview` |
| **IDs** | Numeric ids (comments, reviews) | Node ids (`PRRC_`, `PRRT_`, `PRR_`) — needed for thread mutations |
| **Rate limits** | Primary REST rate limit + secondary on content creation | Point-based limit; large thread queries cost more nodes. Source: [GraphQL rate limits](https://docs.github.com/en/graphql/overview/rate-limits-and-query-limits-for-the-graphql-api) |
| **Review Desk today** | `reviewdesk-github` already uses GraphQL for inbox/detail reads | Natural fit for thread read + resolve writeback |

Practical split aligned with GitHub’s schema evolution: **GraphQL for thread-shaped reads and resolve/unresolve**; **REST or GraphQL for reviews/comments** depending on whether the client prefers numeric ids or single round-trip mutations. Both require the same auth scopes (below).

---

## 5. Scopes and permissions

### 5.1 Classic OAuth / PAT

| Scope | Review use |
| --- | --- |
| `public_repo` | Public repos: read/write reviews and comments |
| `repo` | Private repos: same |

OAuth apps use scope strings; check `X-OAuth-Scopes` / `X-Accepted-OAuth-Scopes` on responses. Source: [Scopes for OAuth apps](https://docs.github.com/en/apps/oauth-apps/building-oauth-apps/scopes-for-oauth-apps).

### 5.2 Fine-grained PAT / GitHub App (repository)

Repository permission **Pull requests**:

- **Read:** list/get reviews, review comments, PR metadata (`GET …/pulls/{n}/reviews`, `GET …/pulls/{n}/comments`, etc.)
- **Write:** create/submit/delete pending reviews, create/edit/delete review comments, dismiss reviews (`POST …/reviews`, `POST …/comments`, etc.)

Source: [Permissions for fine-grained PATs — Pull requests](https://docs.github.com/en/rest/authentication/permissions-required-for-fine-grained-personal-access-tokens) (table “Repository permissions for Pull requests”).

GraphQL mutations inherit the same repository permissions; there is no separate “GraphQL scope.”

### 5.3 Auth paths Review Desk may use

Local `gh auth token` / `GITHUB_TOKEN` typically yields classic `repo` or fine-grained pull-request access depending on how the user configured `gh auth login`.

---

## 6. Known limits affecting Conversation / Diff writeback

| Limit | Detail | Implication |
| --- | --- | --- |
| **Outdated anchors** | Wrong or stale `commit_id` / line after new commits → comments marked outdated (`outdated` / `isOutdated`) | Writeback must target **head commit OID/SHA** and line numbers in the **current PR diff**; refresh after push |
| **Multi-line** | Requires consistent `start_line`, `start_side`, `line`, `side` (REST) or `startLine`, `startSide`, `line`, `side` (GraphQL) | Diff selection UI must capture a range, not a single byte offset |
| **File-level threads** | `subject_type: file` / `subjectType: FILE` | Conversation on deleted files or whole-file notes |
| **Reply depth** | REST replies only to **top-level** review comment | Thread UI should reply to root comment id |
| **Pending batch** | One draft review session per user per PR (Help + pending delete/submit API) | Batch annotations into one pending review or delete/recreate pending before submit |
| **Immediate vs pending** | REST/CLI paths with `event` set publish review; pending path hides comments until submit | Product must pick “draft review” vs “post immediately” semantics |
| **Resolve** | GraphQL-only | Conversation resolve toggle needs GraphQL `threadId` (`PullRequestReviewThread.id`) |
| **Suggestions apply** | No official apply API | “Apply suggestion” writeback = custom git commit or open GitHub UI |
| **Position-based anchoring** | Deprecated | Do not build new Diff writeback on diff `position` |
| **Pagination** | 100 threads/comments per GraphQL page; REST `per_page` ≤ 100 | Large PRs need cursor paging for full thread sync |
| **Secondary rate limits** | Fast comment/review creation throttled | Debounce bulk writeback from Diff |
| **Protected branches** | Dismiss review may require admin / allowlist | Dismiss actions may fail for non-admins. Source: [Dismiss a review](https://docs.github.com/en/rest/pulls/reviews#dismiss-a-review-for-a-pull-request) |
| **Issue suggestions API** | Not for PR code suggestions | Ignore `/issues/.../suggestions` for review suggestion blocks |

---

## 7. Endpoint / mutation quick reference

### REST (pull request review)

| Method | Path | Purpose |
| --- | --- | --- |
| GET | `/repos/{owner}/{repo}/pulls/{pull_number}/comments` | List inline comments |
| POST | `/repos/{owner}/{repo}/pulls/{pull_number}/comments` | Create inline comment |
| POST | `/repos/{owner}/{repo}/pulls/{pull_number}/comments/{comment_id}/replies` | Reply in thread |
| GET/POST | `/repos/{owner}/{repo}/pulls/{pull_number}/reviews` | List / create review (pending if no `event`) |
| PUT/DELETE | `/repos/{owner}/{repo}/pulls/{pull_number}/reviews/{review_id}` | Update body / delete pending |
| POST | `/repos/{owner}/{repo}/pulls/{pull_number}/reviews/{review_id}/events` | Submit pending (`APPROVE`, `REQUEST_CHANGES`, `COMMENT`) |
| PUT | `/repos/{owner}/{repo}/pulls/{pull_number}/reviews/{review_id}/dismissals` | Dismiss review |

Doc hub: [REST: Pull request reviews](https://docs.github.com/en/rest/pulls/reviews), [REST: Pull request review comments](https://docs.github.com/en/rest/pulls/comments).

### GraphQL (mutations)

| Mutation | Purpose |
| --- | --- |
| `addPullRequestReview` | Create review; optional `threads`, optional immediate `event` |
| `addPullRequestReviewThread` | New thread on **pending** review |
| `addPullRequestReviewThreadReply` | Reply on existing thread |
| `submitPullRequestReview` | Submit pending with `event` |
| `updatePullRequestReview` / `updatePullRequestReviewComment` | Edit bodies |
| `deletePullRequestReview` / `deletePullRequestReviewComment` | Delete pending review or comment |
| `resolveReviewThread` / `unresolveReviewThread` | Thread resolved state |
| `dismissPullRequestReview` | Dismiss submitted review |

Doc hub: [GraphQL: Pull requests](https://docs.github.com/en/graphql/reference/pulls).

### GraphQL (reads for writeback planning)

```graphql
# Illustrative — field names from GitHub schema
repository(owner: "OWNER", name: "REPO") {
  pullRequest(number: N) {
    headRefOid
    viewerCanApplySuggestion
    reviewThreads(first: 100, after: $cursor) {
      pageInfo { hasNextPage endCursor }
      nodes {
        id
        isResolved
        isOutdated
        path
        line
        startLine
        diffSide
        subjectType
        viewerCanResolve
        comments(first: 50) {
          nodes {
            id
            fullDatabaseId
            body
            outdated
            state
            line
            startLine
          }
        }
      }
    }
    reviews(states: PENDING, first: 10) {
      nodes { id state author { login } }
    }
  }
}
```

---

## 8. Primary source index

| Topic | URL |
| --- | --- |
| REST pull request reviews | https://docs.github.com/en/rest/pulls/reviews |
| REST pull request review comments | https://docs.github.com/en/rest/pulls/comments |
| REST working with comments | https://docs.github.com/en/rest/guides/working-with-comments |
| GraphQL pull request reference | https://docs.github.com/en/graphql/reference/pulls |
| GraphQL pagination | https://docs.github.com/en/graphql/guides/using-pagination-in-the-graphql-api |
| GraphQL rate limits | https://docs.github.com/en/graphql/overview/rate-limits-and-query-limits-for-the-graphql-api |
| OAuth scopes | https://docs.github.com/en/apps/oauth-apps/building-oauth-apps/scopes-for-oauth-apps |
| Fine-grained PAT permissions | https://docs.github.com/en/rest/authentication/permissions-required-for-fine-grained-personal-access-tokens |
| Review UI (pending, suggestions) | https://docs.github.com/en/pull-requests/how-tos/review-pull-requests/reviewing-proposed-changes-in-a-pull-request |
| Apply suggestions (UI) | https://docs.github.com/en/pull-requests/how-tos/review-pull-requests/incorporating-feedback-in-your-pull-request |
| Review decision types | https://docs.github.com/en/pull-requests/reference/pull-request-reviews |
