# GitHub checks, mergeability, and auto-merge APIs (primary sources)

Facts only. No product decisions. Covers what official GitHub REST and GraphQL APIs expose for a pull request’s checks, required checks, rollup state, re-run, mergeability, and auto-merge—including auth scopes and documented API/UI gaps.

Research date: 2026-10-18. Sources are [docs.github.com](https://docs.github.com) REST/GraphQL reference pages and first-party guides linked below.

## 1. Mental model

| Concept | What it is | Primary read APIs |
| --- | --- | --- |
| **Check runs** | Rich CI results (GitHub Apps / GitHub Actions). | REST [check runs](https://docs.github.com/en/rest/checks/runs); GraphQL [`CheckRun`](https://docs.github.com/en/graphql/reference/checks#checkrun) via [`PullRequest.statusCheckRollup`](https://docs.github.com/en/graphql/reference/pulls#pullrequest) |
| **Commit statuses** | Legacy context-based statuses (`context`, `state`). | REST [commit statuses](https://docs.github.com/en/rest/commits/statuses); same GraphQL rollup union |
| **Rollup** | Combined checks + statuses for a commit. | GraphQL [`StatusCheckRollup`](https://docs.github.com/en/graphql/reference/commits#statuscheckrollup) (`state`, `contexts`); REST has separate combined status for **statuses only** at `GET .../commits/{ref}/status` |
| **Required checks** | Branch protection / ruleset policy on the **base** branch, evaluated against the PR **head** commit. | REST [rules for a branch](https://docs.github.com/en/rest/repos/rules#get-rules-for-a-branch), [branch protection required status checks](https://docs.github.com/en/rest/branches/branch-protection#get-status-checks-protection); GraphQL `isRequired` on [`CheckRun`](https://docs.github.com/en/graphql/reference/checks#checkrun) / [`StatusContext`](https://docs.github.com/en/graphql/reference/commits#statuscontext) |
| **Mergeability** | Conflict-free merge + policy gates (reviews, checks, draft, behind base, etc.). | REST `mergeable`, `mergeable_state` on [Get a pull request](https://docs.github.com/en/rest/pulls/pulls#get-a-pull-request); GraphQL `mergeable`, `mergeStateStatus` |
| **Auto-merge** | Queue merge when all requirements pass; stores intended **merge method** and commit message fields. | Read: REST `auto_merge` on pull; GraphQL `autoMergeRequest`. Write: GraphQL [`enablePullRequestAutoMerge`](https://docs.github.com/en/graphql/reference/pulls#enablepullrequestautomerge) / [`disablePullRequestAutoMerge`](https://docs.github.com/en/graphql/reference/pulls#disablepullrequestautomerge) only (no REST enable/disable in [Pulls REST](https://docs.github.com/en/rest/pulls/pulls)) |

GitHub Actions workflows produce **check runs**, not commit statuses ([About status checks](https://docs.github.com/en/pull-requests/collaborating-with-pull-requests/collaborating-on-repositories-with-code-quality-features/about-status-checks)).

## 2. Pull request–centric read path

Typical sequence for one open PR:

1. **PR metadata + merge fields** — `GET /repos/{owner}/{repo}/pulls/{pull_number}` ([Get a pull request](https://docs.github.com/en/rest/pulls/pulls#get-a-pull-request)) or GraphQL `repository.pullRequest(number:)` selecting `headRefOid`, `baseRefName`, `mergeable`, `mergeStateStatus`, `statusCheckRollup`, `autoMergeRequest`, `reviewDecision`.
2. **Head SHA** — REST `head.sha`; GraphQL `headRefOid`.
3. **All checks/statuses on head** — prefer GraphQL `pullRequest.statusCheckRollup { state contexts(first: n) { nodes { ... on CheckRun { name conclusion status isRequired(pullRequestId: ...) } ... on StatusContext { context state isRequired(pullRequestId: ...) } } } }` ([StatusCheckRollup](https://docs.github.com/en/graphql/reference/commits#statuscheckrollup), [`StatusState`](https://docs.github.com/en/graphql/reference/commits#statusstate)).
4. **REST equivalent (split)** — `GET /repos/{owner}/{repo}/commits/{ref}/check-runs` ([List check runs for a Git reference](https://docs.github.com/en/rest/checks/runs#list-check-runs-for-a-git-reference)) plus `GET .../commits/{ref}/status` and `GET .../commits/{ref}/statuses` ([commit statuses](https://docs.github.com/en/rest/commits/statuses)). Note: `.../status` combined `state` is defined over **status contexts**, not check runs ([Get the combined status for a specific reference](https://docs.github.com/en/rest/commits/statuses#get-the-combined-status-for-a-specific-reference)).
5. **Which checks are required for this PR** — GraphQL: `isRequired` with `pullRequestId` / `pullRequestNumber` on each rollup node ([CheckRun](https://docs.github.com/en/graphql/reference/checks#checkrun), [StatusContext](https://docs.github.com/en/graphql/reference/commits#statuscontext)). REST: `GET /repos/{owner}/{repo}/rules/branches/{branch}` with `branch = pull.base.ref` ([Get rules for a branch](https://docs.github.com/en/rest/repos/rules#get-rules-for-a-branch)) and/or `GET .../branches/{branch}/protection/required_status_checks` ([Get status checks protection](https://docs.github.com/en/rest/branches/branch-protection#get-status-checks-protection)).

## 3. Capability matrix

| Need | REST | GraphQL |
| --- | --- | --- |
| List check runs for PR head | `GET /repos/{owner}/{repo}/commits/{ref}/check-runs` (ref = head SHA) | `pullRequest.statusCheckRollup.contexts` → `CheckRun` |
| List legacy status contexts | `GET .../commits/{ref}/statuses` | Rollup → `StatusContext` |
| Rollup `state` (combined) | No single REST field; combine check runs + `.../status` | `statusCheckRollup.state` (`StatusState`: `ERROR`, `EXPECTED`, `FAILURE`, `PENDING`, `SUCCESS`) |
| Per-context required for PR | Infer from rules/branch protection | `CheckRun.isRequired` / `StatusContext.isRequired` |
| Required check policy (names) | `GET .../rules/branches/{base}`; branch protection endpoints | `Ref` / ruleset types in [Branches reference](https://docs.github.com/en/graphql/reference/branches) (`RequiredStatusChecksParameters`) |
| Merge conflict only | `mergeable` (bool, may be `null`) | `mergeable` (`MergeableState`: `MERGEABLE`, `CONFLICTING`, `UNKNOWN`) |
| Full merge gate status | `mergeable_state` (string on pull object) | `mergeStateStatus` (`BEHIND`, `BLOCKED`, `CLEAN`, `DIRTY`, `DRAFT`, `HAS_HOOKS`, `UNKNOWN`, `UNSTABLE`) — [MergeStateStatus](https://docs.github.com/en/graphql/reference/enums#mergestatestatus) |
| Read auto-merge + merge method | `auto_merge` (`merge_method`: `merge` \| `squash` \| `rebase`, `commit_title`, `commit_message`, `enabled_by`) — nullable | `autoMergeRequest { mergeMethod enabledAt enabledBy commitHeadline commitBody }` — [AutoMergeRequest](https://docs.github.com/en/graphql/reference/pulls#automergerequest) |
| Enable / disable auto-merge on PR | **Not documented** in [Pulls REST](https://docs.github.com/en/rest/pulls/pulls) | `enablePullRequestAutoMerge`, `disablePullRequestAutoMerge` |
| Re-run GitHub Actions | `POST .../actions/runs/{run_id}/rerun`, `.../rerun-failed-jobs`, `POST .../actions/jobs/{job_id}/rerun` ([Workflow runs](https://docs.github.com/en/rest/actions/workflow-runs)) | (Use REST; link runs via check run / Actions APIs) |
| Re-request check (App checks) | `POST .../check-runs/{check_run_id}/rerequest`, `POST .../check-suites/{check_suite_id}/rerequest` ([Checks](https://docs.github.com/en/rest/checks/runs#rerequest-a-check-run)) | N/A |
| Update PR branch (behind base) | `PUT .../pulls/{pull_number}/update-branch` | `updatePullRequestBranch` ([Pulls mutations](https://docs.github.com/en/graphql/reference/pulls#updatepullrequestbranch)) |
| Repo allows auto-merge | `allow_auto_merge` on repo; set via `PATCH /repos/{owner}/{repo}` | `Repository.allowAutoMerge` (read via repo object) |

## 4. Re-run and re-request

### GitHub Actions (typical PR checks)

Documented REST ([Re-run a workflow](https://docs.github.com/en/rest/actions/workflow-runs#re-run-a-workflow), [Re-run failed jobs](https://docs.github.com/en/rest/actions/workflow-runs#re-run-failed-jobs-from-a-workflow-run), [Re-run a job](https://docs.github.com/en/rest/actions/workflow-runs#re-run-a-job-from-a-workflow-run)):

- `POST /repos/{owner}/{repo}/actions/runs/{run_id}/rerun`
- `POST /repos/{owner}/{repo}/actions/runs/{run_id}/rerun-failed-jobs`
- `POST /repos/{owner}/{repo}/actions/jobs/{job_id}/rerun`

UI behavior (30-day window, privilege of original actor, rerun limits) is documented in [Re-running workflows and jobs](https://docs.github.com/en/actions/how-tos/manage-workflow-runs/re-run-workflows-and-jobs).

Discover `run_id` from Actions API (e.g. list runs for `head_sha`) or from check run metadata; the Checks guide notes checks can be re-run from the UI and Apps receive `check_run` webhooks ([Using the REST API to interact with checks](https://docs.github.com/en/rest/guides/using-the-rest-api-to-interact-with-checks)).

### Check Runs API (GitHub App checks)

- `POST /repos/{owner}/{repo}/check-runs/{check_run_id}/rerequest` — fires `check_run` webhook `rerequested`; **403** if not rerequestable or check does not belong to the authenticated GitHub App ([Rerequest a check run](https://docs.github.com/en/rest/checks/runs#rerequest-a-check-run)).
- `POST /repos/{owner}/{repo}/check-suites/{check_suite_id}/rerequest` ([Check suites](https://docs.github.com/en/rest/checks/suites#rerequest-a-check-suite)).

Creating/updating check runs requires a **GitHub App** with `checks:write`; OAuth/users can **view** only ([Checks guide](https://docs.github.com/en/rest/guides/using-the-rest-api-to-interact-with-checks)).

## 5. Mergeability

### REST

[Get a pull request](https://docs.github.com/en/rest/pulls/pulls#get-a-pull-request) returns:

- `mergeable` — `true`, `false`, or `null` while GitHub computes a test merge commit ([Checking mergeability](https://docs.github.com/en/rest/guides/using-the-rest-api-to-interact-with-your-git-database#checking-mergeability-of-pull-requests)).
- `mergeable_state` — string (same response schema); use with `mergeable` and poll until `mergeable` is non-null.
- `rebaseable` — optional boolean.

Mergeability is **not** updated until something triggers a test merge (e.g. GET/create/edit pull, or viewing merge UI in the web UI) — do not rely on stale `refs/pull/*/merge` ([Checking mergeability](https://docs.github.com/en/rest/guides/using-the-rest-api-to-interact-with-your-git-database#checking-mergeability-of-pull-requests)).

### GraphQL

On `PullRequest` ([reference](https://docs.github.com/en/graphql/reference/pulls#pullrequest)):

- `mergeable` — conflict-only (`MergeableState`).
- `mergeStateStatus` — includes check/review/draft/behind semantics ([MergeStateStatus](https://docs.github.com/en/graphql/reference/enums#mergestatestatus)).
- `potentialMergeCommit` — test merge commit when computable.
- `viewerCanEnableAutoMerge`, `viewerCanDisableAutoMerge`, `viewerCanUpdatePullRequestHeadRef` — capability hints for the current token user.

## 6. Auto-merge

### Repository setting

Auto-merge must be allowed on the repo (`allow_auto_merge`, default `false`) via Settings or `PATCH /repos/{owner}/{repo}` with `allow_auto_merge` ([Repositories REST](https://docs.github.com/en/rest/repos/repos#update-a-repository), [Managing auto-merge](https://docs.github.com/en/repositories/configuring-branches-and-merges-in-your-repository/configuring-pull-request-merges/managing-auto-merge-for-pull-requests-in-your-repository)).

### Per pull request

**Read**

- REST: `auto_merge` object on pull (null when off) — fields include `merge_method`, `commit_title`, `commit_message`, `enabled_by` ([Pull response schema](https://docs.github.com/en/rest/pulls/pulls#get-a-pull-request)).
- GraphQL: `autoMergeRequest` — `mergeMethod` (`PullRequestMergeMethod`: `MERGE`, `SQUASH`, `REBASE`), `commitHeadline`, `commitBody`, `enabledAt`, `enabledBy` ([AutoMergeRequest](https://docs.github.com/en/graphql/reference/pulls#automergerequest)).

**Write (enable / disable)**

- GraphQL only:
  - `enablePullRequestAutoMerge(input: { pullRequestId, mergeMethod?, commitHeadline?, commitBody?, expectedHeadOid?, authorEmail? })` — default merge method `MERGE`; merge-queue branches ignore method/message inputs ([EnablePullRequestAutoMergeInput](https://docs.github.com/en/graphql/reference/pulls#enablepullrequestautomergeinput)).
  - `disablePullRequestAutoMerge(input: { pullRequestId })`.
- UI notes: enable is offered when the PR cannot merge immediately (checks/reviews pending) ([Automatically merging a pull request](https://docs.github.com/en/pull-requests/how-tos/merge-and-close-pull-requests/automatically-merging-a-pull-request)).

There is **no** documented REST route to enable/disable auto-merge on a single pull request in the official [Pulls REST](https://docs.github.com/en/rest/pulls/pulls) reference (as of research date).

## 7. Scopes and permissions

### OAuth / classic PAT

| Area | Documented scope / access |
| --- | --- |
| Private repo check runs (read) | `repo` ([Get a check run](https://docs.github.com/en/rest/checks/runs#get-a-check-run)) |
| Commit statuses (read/write) | `repo` or targeted `repo:status` ([About commit statuses](https://docs.github.com/en/rest/commits/statuses#about-commit-statuses)) |
| Actions re-run | `repo` ([Re-run a workflow](https://docs.github.com/en/rest/actions/workflow-runs#re-run-a-workflow)) |
| Pull request read (incl. merge fields) | `repo` or `public_repo` for public repos ([OAuth scopes](https://docs.github.com/en/apps/oauth-apps/building-oauth-apps/scopes-for-oauth-apps)) |
| GraphQL mutations (auto-merge, update branch) | Effectively repository write access; insufficient permissions → GraphQL `401` ([Choosing permissions for GraphQL](https://docs.github.com/en/apps/creating-github-apps/registering-a-github-app/choosing-permissions-for-a-github-app)) |

Use response headers `X-OAuth-Scopes` / `X-Accepted-OAuth-Scopes` ([Scopes for OAuth apps](https://docs.github.com/en/apps/oauth-apps/building-oauth-apps/scopes-for-oauth-apps)).

### Fine-grained PAT (representative)

From [Permissions required for fine-grained PATs](https://docs.github.com/en/rest/authentication/permissions-required-for-fine-grained-personal-access-tokens):

| Permission | Examples relevant to this research |
| --- | --- |
| **Pull requests** (read) | `GET .../pulls/{pull_number}` |
| **Pull requests** (write) | `PUT .../pulls/{pull_number}/merge`, `PUT .../pulls/{pull_number}/update-branch` |
| **Actions** (read/write) | Workflow run list/get; `POST .../rerun`, `rerun-failed-jobs`, `jobs/{job_id}/rerun` |
| **Contents** (read) | `GET .../commits/{ref}/status`, `GET .../commits/{ref}/statuses` |
| **Metadata** (read) | `GET .../rules/branches/{branch}`, `GET .../rulesets` |
| **Administration** (write) | `PATCH .../repos/{owner}/{repo}` (`allow_auto_merge`) |

GraphQL auto-merge mutations are not listed per-endpoint in that table; they require equivalent repository write / pull-request permissions in practice ([GraphQL permissions](https://docs.github.com/en/apps/creating-github-apps/registering-a-github-app/choosing-permissions-for-a-github-app)).

### GitHub App

From [Permissions required for GitHub Apps](https://docs.github.com/en/rest/authentication/permissions-required-for-github-apps):

| Permission | Endpoints |
| --- | --- |
| **Checks** read | List/get check runs and suites for a ref |
| **Checks** write | Create/update check runs; `rerequest` check run/suite |
| **Actions** read/write | Workflow runs and re-run endpoints (installation/user tokens as documented) |
| **Pull requests** read/write | Pull endpoints including merge and update-branch |

Check run **rerequest** additionally requires the check to be **rerequestable** and owned by the calling App ([403 behavior](https://docs.github.com/en/rest/checks/runs#rerequest-a-check-run)).

## 8. Documented API vs web UI gaps

Only items stated or implied by official docs:

| UI capability | API limitation / difference | Source |
| --- | --- | --- |
| Enable/disable auto-merge on a PR | No REST enable/disable documented; use GraphQL mutations | [Pulls REST](https://docs.github.com/en/rest/pulls/pulls) vs [GraphQL pulls mutations](https://docs.github.com/en/graphql/reference/pulls#enablepullrequestautomerge) |
| Re-run failed / single job from Checks tab | Users use Actions re-run endpoints with `run_id` / `job_id`; not exposed as a single “re-run this row” PR endpoint | [Workflow runs REST](https://docs.github.com/en/rest/actions/workflow-runs) |
| Re-run third-party App check from UI | `check-runs/.../rerequest` only for the **creating GitHub App**; others get 403 | [Rerequest a check run](https://docs.github.com/en/rest/checks/runs#rerequest-a-check-run) |
| Rich Checks tab (grouping, logs, UI-only affordances) | API exposes check run fields, annotations, Actions logs separately—not one “Checks UI” payload | [CheckRun](https://docs.github.com/en/graphql/reference/checks#checkrun), [About status checks](https://docs.github.com/en/pull-requests/collaborating-with-pull-requests/collaborating-on-repositories-with-code-quality-features/about-status-checks) |
| Combined green/red for **both** checks and statuses in one REST call | REST `commits/{ref}/status` rollup is for **commit statuses** only; check runs need `check-runs` or GraphQL rollup | [Commit statuses](https://docs.github.com/en/rest/commits/statuses), [StatusCheckRollup](https://docs.github.com/en/graphql/reference/commits#statuscheckrollup) |
| Mergeability without triggering test merge | Test merge commit created on GET/create/edit pull (or UI merge box); git ref polling alone is discouraged | [Checking mergeability](https://docs.github.com/en/rest/guides/using-the-rest-api-to-interact-with-your-git-database#checking-mergeability-of-pull-requests) |
| Anyone with write can set any status check state on a repo | API allows posting statuses with write access; branch protection/rules may still require specific integrations | [About status checks](https://docs.github.com/en/pull-requests/collaborating-with-pull-requests/collaborating-on-repositories-with-code-quality-features/about-status-checks) |
| Skipped required check still “green” | Skipped jobs report success and do not block merge if required | [About status checks](https://docs.github.com/en/pull-requests/collaborating-with-pull-requests/collaborating-on-repositories-with-code-quality-features/about-status-checks) |
| Check run “requested actions” buttons in Checks tab | Integrator must handle `check_run.requested_action` webhook; not a generic user REST action | [Using the REST API to interact with checks](https://docs.github.com/en/rest/guides/using-the-rest-api-to-interact-with-checks) |

## 9. Example GraphQL query (single PR)

```graphql
query ($owner: String!, $repo: String!, $number: Int!) {
  repository(owner: $owner, name: $repo) {
    pullRequest(number: $number) {
      id
      headRefOid
      baseRefName
      mergeable
      mergeStateStatus
      viewerCanEnableAutoMerge
      viewerCanDisableAutoMerge
      autoMergeRequest {
        mergeMethod
        commitHeadline
        commitBody
        enabledAt
      }
      statusCheckRollup {
        state
        contexts(first: 100) {
          checkRunCount
          statusContextCount
          nodes {
            __typename
            ... on CheckRun {
              name
              status
              conclusion
              isRequired(pullRequestNumber: $number)
              detailsUrl
              permalink
            }
            ... on StatusContext {
              context
              state
              isRequired(pullRequestNumber: $number)
              targetUrl
            }
          }
        }
      }
    }
  }
}
```

## 10. Source index

- [REST: Check runs](https://docs.github.com/en/rest/checks/runs)
- [REST: Check suites](https://docs.github.com/en/rest/checks/suites)
- [REST: Commit statuses](https://docs.github.com/en/rest/commits/statuses)
- [REST: Pull requests](https://docs.github.com/en/rest/pulls/pulls)
- [REST: Workflow runs (re-run)](https://docs.github.com/en/rest/actions/workflow-runs)
- [REST: Rules (branch rulesets)](https://docs.github.com/en/rest/repos/rules)
- [REST: Branch protection (required status checks)](https://docs.github.com/en/rest/branches/branch-protection)
- [REST: Repositories (allow_auto_merge)](https://docs.github.com/en/rest/repos/repos)
- [Guide: Checking mergeability](https://docs.github.com/en/rest/guides/using-the-rest-api-to-interact-with-your-git-database#checking-mergeability-of-pull-requests)
- [Guide: Checks API](https://docs.github.com/en/rest/guides/using-the-rest-api-to-interact-with-checks)
- [GraphQL: Pulls reference](https://docs.github.com/en/graphql/reference/pulls)
- [GraphQL: Commits / StatusCheckRollup](https://docs.github.com/en/graphql/reference/commits)
- [GraphQL: Checks / CheckRun](https://docs.github.com/en/graphql/reference/checks)
- [GraphQL: Enums (MergeStateStatus, MergeableState, StatusState)](https://docs.github.com/en/graphql/reference/enums)
- [OAuth scopes](https://docs.github.com/en/apps/oauth-apps/building-oauth-apps/scopes-for-oauth-apps)
- [Fine-grained PAT permissions](https://docs.github.com/en/rest/authentication/permissions-required-for-fine-grained-personal-access-tokens)
- [GitHub App permissions](https://docs.github.com/en/rest/authentication/permissions-required-for-github-apps)
- [About status checks (product)](https://docs.github.com/en/pull-requests/collaborating-with-pull-requests/collaborating-on-repositories-with-code-quality-features/about-status-checks)
- [Automatically merging a pull request (product)](https://docs.github.com/en/pull-requests/how-tos/merge-and-close-pull-requests/automatically-merging-a-pull-request)
