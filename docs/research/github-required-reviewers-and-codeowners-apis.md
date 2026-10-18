# GitHub required reviewers, review decision, and CODEOWNERS APIs

Research for [issue #50](https://github.com/jacobdrury/reviewdesk/issues/50) (parent map [#46](https://github.com/jacobdrury/reviewdesk/issues/46)). Facts below come from GitHub’s official docs, the published REST OpenAPI description, and live GraphQL schema introspection (March 2026).

## Recommendation for Review Desk

| Wayfinder need | Prefer | Notes |
| --- | --- | --- |
| Overall review gate (`APPROVED` / `CHANGES_REQUESTED` / `REVIEW_REQUIRED`) | GraphQL `PullRequest.reviewDecision` | Same enum GitHub uses in the merge box; already used in `reviewdesk-github` inbox/detail queries. |
| Who is *requested* (including CODEOWNERS-driven requests) | GraphQL `PullRequest.reviewRequests` + `ReviewRequest.asCodeOwner` | Distinguishes code-owner auto-requests from manual requests; paginate. |
| Per-reviewer approval state | GraphQL `PullRequest.latestReviews` (and optionally `latestOpinionatedReviews`) | Includes `state`, `author`, `commit`, and team context via `onBehalfOf`. No per-rule “satisfied” flag from GitHub. |
| Branch / ruleset policy (counts, code-owner requirement, path-based teams) | REST or GraphQL **rulesets** (and legacy branch protection where still used) | Read `pull_request` rule parameters; filter rulesets by `conditions.refName` vs PR `baseRefName`. |
| Which CODEOWNERS patterns hit which changed files | **Client-side** | Fetch base-branch `CODEOWNERS` + PR changed paths; GitHub does not expose matched owners per file on the PR object. |
| CODEOWNERS parse diagnostics only | REST `GET /repos/{owner}/{repo}/codeowners/errors?ref=` or GraphQL `Repository.codeowners { errors { … } }` | Errors only—not rules or matches. |

**Gist:** GraphQL covers aggregate **review decision** and **review requests/approvals**. **Policy** (required count, `require_code_owner_review`, ruleset **required reviewers** with file patterns) comes from rulesets/branch protection. **CODEOWNERS matching and per-owner satisfaction** must be **derived** from the base-branch file, changed paths, rules, and review history.

---

## 1. Review decision (pull request level)

GitHub exposes a single aggregate review status for a PR—not a breakdown by code-owner rule or required-reviewer team.

### GraphQL (supported, no preview header)

- **Field:** `PullRequest.reviewDecision` → enum `PullRequestReviewDecision`
- **Values:** `APPROVED`, `CHANGES_REQUESTED`, `REVIEW_REQUIRED`
- **Meaning (from schema):** “The current status of this pull request with respect to code review.”

Sources:

- [GraphQL reference — Pull requests (`PullRequest.reviewDecision`)](https://docs.github.com/en/graphql/reference/pulls)
- Live introspection: `PullRequestReviewDecision` enum (via `gh api graphql` `__type` query)

Related merge/review context on the same object:

| Field | Role |
| --- | --- |
| `mergeStateStatus` | `MergeStateStatus` (`BLOCKED`, `CLEAN`, `BEHIND`, …)—mergeability including reviews, status checks, and other gates—not CODEOWNERS-specific. |
| `mergeable` | Conflict-only mergeability, not review completion. |
| `statusCheckRollup` | CI/check rollup for head ref. |

Sources: [GraphQL Pull requests reference](https://docs.github.com/en/graphql/reference/pulls); schema introspection.

### REST

- The published OpenAPI `pull-request` schema includes `mergeable`, `mergeable_state`, `requested_reviewers`, but **not** `review_decision`.
- Some live REST responses may still include a `review_decision` key; treat **GraphQL as canonical** for this field unless/until REST documents it.

Sources:

- [REST — Get a pull request](https://docs.github.com/en/rest/pulls/pulls#get-a-pull-request)
- [GitHub REST OpenAPI — `pull-request` schema](https://github.com/github/rest-api-description) (no `review_decision` property)

### Deriving decision from REST alone

- List reviews: `GET /repos/{owner}/{repo}/pulls/{pull_number}/reviews` and inspect each review’s `state` (`APPROVED`, `CHANGES_REQUESTED`, `COMMENTED`, `DISMISSED`, `PENDING`).
- This does **not** replicate GitHub’s full merge-box logic (stale dismissals, last-push approval, code-owner rules, required counts, blocking cross-PR reviews, etc.).

Source: [REST — Pull request reviews](https://docs.github.com/en/rest/pulls/reviews)

---

## 2. Required reviewers and review policy (not the same as CODEOWNERS)

GitHub has several overlapping mechanisms:

1. **Required approving review count** — N approving reviews from users with write access.
2. **Require review from code owners** — when changed files have owners in `CODEOWNERS`, at least one approving review from **any** owner of those files (per pattern line, any listed owner suffices).
3. **Required reviewers (rulesets)** — up to 15 **teams**, each with **fnmatch/gitignore-style file patterns** and **minimum approvals** from that team (0 = visibility only).

Sources:

- [About protected branches — Require pull request reviews](https://docs.github.com/en/repositories/configuring-branches-and-merges-in-your-repository/managing-protected-branches/about-protected-branches#require-pull-request-reviews-before-merging)
- [Available rules for rulesets — Require a pull request before merging (Required reviewers)](https://docs.github.com/en/repositories/configuring-branches-and-merges-in-your-repository/managing-rulesets/available-rules-for-rulesets#require-a-pull-request-before-merging)
- [About code owners — CODEOWNERS and branch protection](https://docs.github.com/en/repositories/managing-your-repositorys-settings-and-features/customizing-your-repository/about-code-owners#codeowners-and-branch-protection)

There is **no** GraphQL field on `PullRequest` that lists “required reviewers still outstanding” with file scope. Policy is read from **rules**; satisfaction is inferred from **reviews** + **changed files**.

### Rulesets (preferred on many repos)

**REST**

- List: `GET /repos/{owner}/{repo}/rulesets`
- Get: `GET /repos/{owner}/{repo}/rulesets/{ruleset_id}`
- Pull-request rule type: `repository-rule-pull-request` with parameters including:

| Parameter | Meaning |
| --- | --- |
| `required_approving_review_count` | Minimum approving reviews (0–10). |
| `require_code_owner_review` | CODEOWNERS approval required when applicable files change. |
| `require_last_push_approval` | Latest push must be approved by someone other than the pusher. |
| `dismiss_stale_reviews_on_push` | New commits dismiss stale approvals. |
| `required_review_thread_resolution` | All review threads resolved. |
| `required_reviewers` | Array of `{ reviewer, file_patterns, minimum_approvals }` (teams; fnmatch patterns). |

Sources:

- [REST — Get a repository ruleset](https://docs.github.com/en/rest/repos/rules?apiVersion=2022-11-28#get-a-repository-ruleset)
- OpenAPI: `repository-rule-pull-request`, `repository-rule-params-required-reviewer-configuration`

**GraphQL**

- `Repository.rulesets` / `Repository.ruleset(id)` → `RepositoryRuleset.rules` → `RepositoryRule` where `type == PULL_REQUEST` and `parameters` is `PullRequestParameters`:
  - `requiredApprovingReviewCount`, `requireCodeOwnerReview`, `requireLastPushApproval`, `dismissStaleReviewsOnPush`, `requiredReviewThreadResolution`, `requiredReviewers` (`RequiredReviewerConfiguration`: `reviewerId`, `filePatterns`, `minimumApprovals`).

Sources:

- [GraphQL reference — Pull requests (`PullRequestParameters`, `RequiredReviewerConfiguration`)](https://docs.github.com/en/graphql/reference/pulls)
- [GraphQL changelog 2025-11-13 — `RequiredReviewerConfiguration` / `requiredReviewers`](https://docs.github.com/en/graphql/overview/changelog/2025)

**Applying rules to a PR’s base branch**

- Rulesets use `conditions.refName` (`include` / `exclude` patterns, `~DEFAULT_BRANCH`, `~ALL`).
- Match PR `baseRefName` (GraphQL) or `base.ref` (REST) against conditions client-side; GitHub does not return “rules that apply to PR #N” on the PR node.

Source: schema introspection `RepositoryRuleConditions.refName`; [Managing rulesets](https://docs.github.com/en/repositories/configuring-branches-and-merges-in-your-repository/managing-rulesets/managing-rulesets-for-a-repository)

**Preview / beta**

- Docs describe **required reviewers** on rulesets as a product feature (team + path patterns); GraphQL schema text marks `requiredReviewers` as beta in the reference. As of testing against the public API, **`requiredReviewers` is queryable without a GraphQL `Accept` preview media type** (unlike older previews such as `merge-info` for merge metadata).
- Always re-check [GraphQL changelog](https://docs.github.com/en/graphql/overview/changelog) before relying on beta fields in production.

### Legacy branch protection

**REST**

- `GET /repos/{owner}/{repo}/branches/{branch}/protection`
- Nested `required_pull_request_reviews`: `required_approving_review_count`, `require_code_owner_reviews`, `dismiss_stale_reviews`, `require_last_push_approval`, etc.
- **Does not** include ruleset-style `required_reviewers` path→team map (that is rulesets-only).

Source: OpenAPI `protected-branch-pull-request-review`; [REST branch protection](https://docs.github.com/en/rest/branches/branch-protection)

**GraphQL**

- `Repository.branchProtectionRules` → `BranchProtectionRule`: `requiresApprovingReviews`, `requiredApprovingReviewCount`, `requiresCodeOwnerReviews`, `dismissesStaleReviews`, etc.

Source: schema introspection `BranchProtectionRule`

### Rule suites (insights—not a PR wayfinder primary API)

- `GET /repos/{owner}/{repo}/rulesets/rule-suites` and `…/rule-suites/{rule_suite_id}` expose **historical rule evaluations** (often push-oriented) with optional metadata.
- Useful for auditing bypass/fail events; **not** a stable “current required reviewer matrix for this PR” endpoint.

Sources:

- [REST — Rule suites](https://docs.github.com/en/rest/repos/rule-suites)
- [Managing rulesets — Viewing insights](https://docs.github.com/en/repositories/configuring-branches-and-merges-in-your-repository/managing-rulesets/managing-rulesets-for-a-repository#viewing-insights-for-rulesets)

---

## 3. Review requests and individual reviews

### GraphQL

| Type / field | Use |
| --- | --- |
| `PullRequest.reviewRequests` | Outstanding requests; paginate. |
| `ReviewRequest.asCodeOwner` | `true` when the request was created for CODEOWNERS. |
| `ReviewRequest.requestedReviewer` | `User` or `Team` union. |
| `PullRequest.latestReviews` | Latest review per user excluding pending review state (schema description). |
| `PullRequest.latestOpinionatedReviews` | Latest review per user including pending. |
| `PullRequestReview.state` | `APPROVED`, `CHANGES_REQUESTED`, `COMMENTED`, `DISMISSED`, `PENDING`. |
| `PullRequestReview.commit` | OID associated with the review (for stale / last-push logic). |
| `PullRequestReview.onBehalfOf` | Teams the review counts for (team approvals). |

Sources: [GraphQL Pull requests reference](https://docs.github.com/en/graphql/reference/pulls); schema introspection.

### REST

| Endpoint | Use |
| --- | --- |
| `GET …/pulls/{pull_number}` | `requested_reviewers`, `requested_teams` (current requests only). |
| `GET …/pulls/{pull_number}/reviews` | Submitted reviews and states. |
| `GET …/pulls/{pull_number}/requested_reviewers` | Requested users/teams. |

Source: [REST pulls](https://docs.github.com/en/rest/pulls/pulls)

**Gap:** REST/GraphQL do not attach `asCodeOwner` on REST requested reviewers; use GraphQL `reviewRequests.asCodeOwner` for CODEOWNERS attribution.

---

## 4. CODEOWNERS: what APIs provide

GitHub **does not** expose an API that returns “for PR #N, file F matched pattern P with owners O.” UI tooltip behavior is server-side only.

### File location and semantics (for client matching)

- Single effective file per branch, first found: `.github/CODEOWNERS`, then `CODEOWNERS`, then `docs/CODEOWNERS`.
- Evaluated from the **base branch** of the PR (upstream base for forks).
- **Last matching pattern wins**; multiple owners on the **same line** — **any one** approval satisfies code-owner requirement for that file.
- Patterns follow gitignore-like rules with documented exceptions (no `!` negation, no `[ ]` ranges, etc.).

Source: [About code owners](https://docs.github.com/en/repositories/managing-your-repositorys-settings-and-features/customizing-your-repository/about-code-owners)

### Changed paths on a PR

| API | Field | Limits |
| --- | --- | --- |
| GraphQL | `PullRequest.files(first:, after:)` → `PullRequestChangedFile.path` | Paginate; no owner metadata on nodes. |
| REST | `GET …/pulls/{pull_number}/files` | Max **3000** files documented; paginated (default 30/page). |

Sources: [GraphQL Pull requests](https://docs.github.com/en/graphql/reference/pulls); REST OpenAPI `/pulls/{pull_number}/files`

### Fetching CODEOWNERS content

- **REST contents / raw:** `GET /repos/{owner}/{repo}/contents/{path}?ref={baseRef}` for `.github/CODEOWNERS`, `CODEOWNERS`, or `docs/CODEOWNERS` (try in that order), or git blob API.
- **Not provided:** parsed rules, matched paths, or owner validity (beyond errors endpoint).

### CODEOWNERS errors only (syntax / invalid owners)

| API | Returns |
| --- | --- |
| REST | `GET /repos/{owner}/{repo}/codeowners/errors?ref=` → `{ errors: [{ line, column, kind, message, path, … }] }` |
| GraphQL | `Repository.codeowners { errors { line, column, kind, message, path, source, suggestion } }` |

Sources:

- [REST — List CODEOWNERS errors](https://docs.github.com/en/rest/repos/repos#list-codeowners-errors) (OpenAPI `codeowners-errors`)
- [About code owners — invalid lines](https://docs.github.com/en/repositories/managing-your-repositorys-settings-and-features/customizing-your-repository/about-code-owners#codeowners-syntax)
- Schema introspection: `RepositoryCodeowners` exposes only `errors`, not rule text.

`Repository.codeowners` may be `null` when no file exists (observed on repos without CODEOWNERS).

---

## 5. Deriving “matching rules, owners, satisfaction” client-side

Review Desk wayfinder should treat the following as **local computation** backed by API inputs:

### A. CODEOWNERS coverage (when `require_code_owner_review` / `requiresCodeOwnerReviews` is true)

1. Load CODEOWNERS from **PR base ref** (see §4).
2. List all changed file paths (GraphQL `files` pagination preferred for large PRs).
3. For each path, compute winning pattern and owner list (last match wins; replicate GitHub glob semantics).
4. Build unique set of required owner handles (`@user`, `@org/team`) across touched owned files.
5. **Satisfaction:** at least one **`APPROVED`** review on the PR from a user who qualifies as an owner for **every** owned file cluster—or, per GitHub product rules, **any one owner on the line** satisfies that file’s owners; globally, GitHub requires approval from code owners for changed owned content, commonly summarized as needing owner approval covering the owned changes (implementations often treat “union of owners” with **any** approving owner that is in the set for **each** file as sufficient when multiple owners are listed on one line).

Source for “any owner on the line”: [About code owners — branch protection note](https://docs.github.com/en/repositories/managing-your-repositorys-settings-and-features/customizing-your-repository/about-code-owners#codeowners-and-branch-protection)

6. Cross-check `reviewRequests(where asCodeOwner)` for pending owner requests.
7. Use `latestReviews` + `onBehalfOf` for team approvals; compare review `commit.oid` to head when simulating stale / last-push rules.

GitHub’s **`reviewDecision`** already incorporates this gate; per-owner/per-file **satisfaction UI** is custom.

### B. Ruleset required reviewers (path → team)

1. Load applicable rulesets for `baseRefName`.
2. Read `required_reviewers` / `requiredReviewers` entries.
3. Match changed paths using **ruleset fnmatch/gitignore semantics** (includes `!` negation per [Available rules — Required reviewers](https://docs.github.com/en/repositories/configuring-branches-and-merges-in-your-repository/managing-rulesets/available-rules-for-rulesets#require-a-pull-request-before-merging)—**different from CODEOWNERS**).
4. For each team rule that matches at least one changed file, count qualifying `APPROVED` reviews from team members (membership may require separate team API if not on `onBehalfOf`).

### C. Count-based required approvals

- Use `required_approving_review_count` minus count of distinct approving users (respecting dismissals and bot/Copilot attribution rules such as `require_extra_approval_for_unattributed_changes` on rulesets—see ruleset REST example fields).

Source: [Available rules for rulesets](https://docs.github.com/en/repositories/configuring-branches-and-merges-in-your-repository/managing-rulesets/available-rules-for-rulesets)

---

## 6. Preview headers and availability summary

| Capability | GraphQL | REST | Preview / notes |
| --- | --- | --- | --- |
| `reviewDecision` | Yes | Not in OpenAPI schema | Use GraphQL |
| `reviewRequests.asCodeOwner` | Yes | No equivalent | |
| Ruleset `requiredReviewers` | Yes (`PullRequestParameters`) | Yes (`required_reviewers`) | Documented beta in GraphQL reference; queryable without preview header today |
| `requireCodeOwnerReview` on rules | Yes | Yes | |
| Legacy `requiresCodeOwnerReviews` | Yes (`BranchProtectionRule`) | Yes (`require_code_owner_reviews`) | |
| CODEOWNERS file content | Via `contents` / git | Via `contents` / git | Not via `Repository.codeowners` |
| CODEOWNERS parse errors | `Repository.codeowners.errors` | `/codeowners/errors` | |
| Per-file CODEOWNERS match on PR | **No** | **No** | Client-side only |
| Per-team required-reviewer satisfaction | **No** | **No** | Client-side only |
| PR changed files | `PullRequest.files` | `/pulls/.../files` | REST 3000-file cap |

Historical GraphQL previews (e.g. `application/vnd.github.shadow-cat-preview+json` for drafts, comfort-fade for threads) are unrelated to this wayfinder; see [Schema previews](https://docs.github.com/en/graphql/overview/schema-previews).

---

## 7. Suggested GraphQL bundle for Review Desk

Minimal extension beyond current `reviewdesk-github` queries:

```graphql
query PullRequestReviewContext($owner: String!, $name: String!, $number: Int!, $filesAfter: String) {
  repository(owner: $owner, name: $name) {
    pullRequest(number: $number) {
      baseRefName
      reviewDecision
      mergeStateStatus
      reviewRequests(first: 100) {
        nodes { asCodeOwner requestedReviewer { ... on User { login } ... on Team { slug organization { login } } } }
      }
      latestReviews(first: 100) {
        nodes {
          state
          submittedAt
          author { login ... on User { login } ... on Bot { login } }
          commit { oid }
          onBehalfOf(first: 10) { nodes { slug organization { login } } }
        }
      }
      files(first: 100, after: $filesAfter) {
        pageInfo { hasNextPage endCursor }
        nodes { path changeType }
      }
    }
    rulesets(first: 20) {
      nodes {
        name
        target
        conditions { refName { include exclude } }
        rules(first: 50) {
          nodes {
            type
            parameters {
              ... on PullRequestParameters {
                requireCodeOwnerReview
                requiredApprovingReviewCount
                requireLastPushApproval
                dismissStaleReviewsOnPush
                requiredReviewers { reviewerId filePatterns minimumApprovals }
              }
            }
          }
        }
      }
    }
  }
}
```

Fetch CODEOWNERS separately via REST `contents` on `baseRefName` (or cache per base ref). Filter `rulesets` client-side to those whose `refName` conditions include the PR base branch.

---

## 8. Relation to current Review Desk code

`crates/reviewdesk-github` already queries `reviewDecision`, `reviewRequests`, and maps aggregate review state for inbox/detail. It does **not** yet load rulesets, CODEOWNERS, changed files, `asCodeOwner`, or `latestReviews`—all of which wayfinder needs for required-reviewer and CODEOWNERS path UI.

---

## Primary sources (index)

- [About code owners](https://docs.github.com/en/repositories/managing-your-repositorys-settings-and-features/customizing-your-repository/about-code-owners)
- [About protected branches](https://docs.github.com/en/repositories/configuring-branches-and-merges-in-your-repository/managing-protected-branches/about-protected-branches)
- [Available rules for rulesets](https://docs.github.com/en/repositories/configuring-branches-and-merges-in-your-repository/managing-rulesets/available-rules-for-rulesets)
- [GraphQL reference — Pull requests](https://docs.github.com/en/graphql/reference/pulls)
- [GraphQL changelog](https://docs.github.com/en/graphql/overview/changelog)
- [REST — Pulls](https://docs.github.com/en/rest/pulls/pulls)
- [REST — Pull request reviews](https://docs.github.com/en/rest/pulls/reviews)
- [REST — Repositories (CODEOWNERS errors)](https://docs.github.com/en/rest/repos/repos#list-codeowners-errors)
- [REST — Repository rulesets](https://docs.github.com/en/rest/repos/rules)
- [REST — Rule suites](https://docs.github.com/en/rest/repos/rule-suites)
- [GitHub REST OpenAPI description](https://github.com/github/rest-api-description) (`repository-rule-pull-request`, `codeowners-errors`, `pull-request`)
