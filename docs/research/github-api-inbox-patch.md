# GitHub API for Inbox listing and Patch fetch

Research for [issue #3](https://github.com/jacobdrury/reviewdesk/issues/3). Scope is github.com only (not GHES). Auth is an existing `gh` login or a PAT. No OAuth app yet.

## Recommendation

| Need | Surface | Why |
| --- | --- | --- |
| Inbox (requested reviewer, optional author) | REST `GET /search/issues` with issue/PR search qualifiers, or GraphQL `search(..., type: ISSUE)` with the same query string | There is no dedicated "my review requests" list endpoint. Search is the documented way to filter by `review-requested` / `author` across repos the token can see. |
| Patch for gpui-kit `DiffFile::parse` | REST `GET /repos/{owner}/{repo}/pulls/{pull_number}` with `Accept: application/vnd.github.diff` | Returns unified `diff --git` text (git diff format). GraphQL `PullRequest.files` has no patch body. |
| Auth | Token from `gh auth login` / `gh auth token`, or classic PAT with `repo` (and typically `read:org`) | Diff/patch responses accept `repo`. Fine-grained PATs cannot call `GET /search/issues`, so they are a poor fit for cross-repo Inbox. |

## 1. Inbox listing

### Chosen API

**Primary:** REST Search issues and pull requests.

```http
GET /search/issues?q=is:open+is:pr+review-requested:@me
```

Optional author union (same endpoint, richer `q`):

```text
is:open is:pr (review-requested:@me OR author:@me)
```

Sort example: `sort=updated&order=desc`.

Sources:

- [REST API endpoints for search](https://docs.github.com/en/rest/search/search#search-issues-and-pull-requests): `GET /search/issues`; `q` uses the same qualifiers as the web UI; up to 100 results per page; search has its own rate limit (30 authenticated requests per minute for this family of endpoints, excluding code search).
- [Searching issues and pull requests](https://docs.github.com/en/search-github/searching-on-github/searching-issues-and-pull-requests):
  - `is:pr` / `type:pr` restrict to pull requests.
  - `review-requested:USERNAME` matches PRs where that person is requested; team membership review requests for that user are included; after the person reviews, they drop out of this qualifier.
  - `user-review-requested:@me` matches only direct user review requests (not team).
  - `author:@me` matches PRs you authored; `@me` works on username qualifiers.
- [Understanding the search syntax](https://docs.github.com/en/search-github/getting-started-with-searching-on-github/understanding-the-search-syntax): `@me` is valid on username qualifiers.
- Search query length / operator caps: REST search docs allow at most five `AND`, `OR`, or `NOT` operators and cap query length; see [Constructing a search query / limitations](https://docs.github.com/en/rest/search/search#limitations-on-query-length).

**Equivalent GraphQL:** root `search` with `type: ISSUE` and the same `query` string. `SearchType.ISSUE` is documented as returning issue-shaped search results; pull requests appear as `PullRequest` nodes in the result union.

```graphql
query Inbox($q: String!) {
  search(query: $q, type: ISSUE, first: 50) {
    issueCount
    nodes {
      ... on PullRequest {
        number
        title
        url
        updatedAt
        repository { nameWithOwner }
        author { login }
      }
    }
  }
}
```

Sources:

- GraphQL endpoint and auth: [Forming calls with GraphQL](https://docs.github.com/en/graphql/guides/forming-calls-with-graphql) (`https://api.github.com/graphql`, `Authorization: Bearer TOKEN`).
- Result union includes `PullRequest`: [SearchResultItem](https://docs.github.com/en/graphql/reference/unions#searchresultitem).
- CLI helper: [Using GraphQL clients](https://docs.github.com/en/graphql/overview/explorer) documents `gh api graphql`.

GraphQL is useful when Inbox rows need fields that would otherwise require a second REST call per PR. It does not replace REST for Patch text (see below).

### Qualifier choice for Review Desk Inbox

| Intent | Qualifier | Note |
| --- | --- | --- |
| Open PRs where you are still a requested reviewer (user or team) | `is:open is:pr review-requested:@me` | Matches product Inbox wording; clears after you submit a review. |
| Direct request only | `user-review-requested:@me` | Excludes team-only requests. |
| Also show your own open PRs | add `OR author:@me` | Keep the expression under the five-operator limit. |
| Still show after you reviewed | `review-involves:@me` | Broader than "requested"; not the default Inbox filter. |

### Alternatives rejected for Inbox

| Approach | Why not for this slice |
| --- | --- |
| `GET /repos/{owner}/{repo}/pulls` | Per-repository only; no cross-repo "requested of me" filter. |
| Notifications API | Different model (threads, reasons); not the review-request search model. |
| GraphQL `viewer` repository crawl + `reviewRequests` | Does not scale; search already encodes the filter. |

### Rate limit and result caps (Inbox)

- Authenticated search: 30 requests/minute for `/search/issues` ([About search / Rate limit](https://docs.github.com/en/rest/search/search#rate-limit)).
- At most 1,000 search results total per query ([About search](https://docs.github.com/en/rest/search/search#about-search)).
- GraphQL uses a separate points budget ([Rate limits and query limits for the GraphQL API](https://docs.github.com/en/graphql/overview/rate-limits-and-node-limits-for-the-graphql-api)).

## 2. Patch fetch for Diff parsing

### Chosen API

```http
GET /repos/{owner}/{repo}/pulls/{pull_number}
Accept: application/vnd.github.diff
```

Response body is plain text in git unified diff form (`diff --git a/... b/...`, `---` / `+++`, hunks). GitHub documents this media type as the git-diff format and points at the Git documentation.

Also supported on the same URL:

```http
Accept: application/vnd.github.patch
```

That returns a mailbox-style patch (`From`, `Date`, `Subject: [PATCH]`, then the unified diff). Prefer **`.diff`** for `DiffFile::parse`, which expects unified/split patch text rather than email headers.

Sources:

- [Get a pull request](https://docs.github.com/en/rest/pulls/pulls#get-a-pull-request): "Pass the appropriate media type to fetch diff and patch formats"; lists `application/vnd.github.diff` and refers to git-diff.
- [Getting started with the REST API / Media types](https://docs.github.com/en/rest/using-the-rest-api/getting-started-with-the-rest-api#media-types): commits, comparisons, and pull requests support custom media types `diff`, `patch`, and `sha` (`application/vnd.github.diff`, `application/vnd.github.patch`).
- Pull request JSON also exposes `diff_url` and `patch_url` ([List / get pull request schemas](https://docs.github.com/en/rest/pulls/pulls)); those are HTML/site URLs for the same formats. Prefer the API endpoint with an `Accept` header so the call stays on `api.github.com` with the same token.

### Secondary: per-file patches

```http
GET /repos/{owner}/{repo}/pulls/{pull_number}/files
```

Each `Diff Entry` may include a `patch` string (unified hunk for that file). Responses paginate (default 30, max 100 per page) and cap at 3,000 files.

Use this when you need a file list and metadata alongside hunks. Concatenating `patch` fields can approximate a multi-file unified diff, but the whole-PR `.diff` media type is the direct match for "one Patch for the Diff surface."

Source: [List pull requests files](https://docs.github.com/en/rest/pulls/pulls#list-pull-requests-files).

### GraphQL cannot supply Patch text

`PullRequest.files` → `PullRequestChangedFile` exposes `path`, `additions`, `deletions`, `changeType`, `viewerViewedState`. There is no patch/diff string field.

Source: [PullRequest / PullRequestChangedFile](https://docs.github.com/en/graphql/reference/objects) (Pulls reference).

### Diff size limits

GitHub truncates large diffs (UI and API share these caps):

- Total PR diff: 20,000 loadable lines or 1 MB raw diff data.
- Single file: 20,000 lines or 500 KB.
- Max files in a single diff view: 300.

Source: [Repository limits / Diff limits](https://docs.github.com/en/repositories/creating-and-managing-repositories/repository-limits#diff-limits).

`reviewdesk-core` should treat oversized PRs as an expected failure mode (partial or empty Patch), not as a bug in the client parser alone.

## 3. Auth scopes (`gh` login or PAT)

### Using existing `gh` login

1. User runs `gh auth login` (browser or token). Default host is `github.com`.
2. App reads the token with `gh auth token` (or env vars documented under `gh help environment`).
3. Call `api.github.com` / `api.github.com/graphql` with `Authorization: Bearer <token>`.

Sources:

- [gh auth login](https://cli.github.com/manual/gh_auth_login): stores a token; `--with-token` requires a classic PAT with at least `repo`, `read:org`, and `gist`; `--scopes` can request extras; points at [Scopes for OAuth apps](https://docs.github.com/en/apps/oauth-apps/building-oauth-apps/scopes-for-oauth-apps).
- [gh auth token](https://cli.github.com/manual/gh_auth_token): prints the stored token for the host/account.

Observed default scopes on a normal `gh auth login` session include `repo`, `read:org`, and `gist` (and often `admin:public_key` if SSH key upload ran). That set is enough for Inbox search and private-repo PR diffs.

### Classic PAT

| Scope | Role for this slice |
| --- | --- |
| `repo` | Required for private repository content and PR diffs. Diff/patch responses advertise `X-Accepted-OAuth-Scopes: repo`. Grants full access to public and private repos (broader than read-only). |
| `public_repo` | Enough only if Inbox and Patches are limited to public repos. |
| `read:org` | Needed for org/team visibility that search and team review requests rely on; also part of `gh`'s minimum for `--with-token`. |
| `gist` | Not required by Inbox/Patch APIs; required only if you feed the token through `gh auth login --with-token`. |

Source: [Scopes for OAuth apps](https://docs.github.com/en/apps/oauth-apps/building-oauth-apps/scopes-for-oauth-apps) (`repo`, `public_repo`, `read:org`, `gist`).

### Fine-grained PAT

GitHub recommends fine-grained tokens when they fit, but:

- Fine-grained tokens are limited to one user or organization resource owner and cannot cover "all orgs I belong to" in one token ([Managing your personal access tokens](https://docs.github.com/en/authentication/keeping-your-account-and-data-secure/managing-your-personal-access-tokens)).
- The published list of fine-grained-compatible endpoints under **search** includes only `GET /search/labels`, not `GET /search/issues` ([Endpoints available for fine-grained personal access tokens](https://docs.github.com/en/rest/authentication/endpoints-available-for-fine-grained-personal-access-tokens#search)).
- For a single known repo, PR read endpoints need **Pull requests: Read** (and related permissions as listed per endpoint) ([Permissions required for fine-grained personal access tokens](https://docs.github.com/en/rest/authentication/permissions-required-for-fine-grained-personal-access-tokens)).

For the first-slice Inbox (cross-repo review requests), prefer **`gh` OAuth token or classic PAT with `repo` (+ `read:org`)**. Fine-grained may work later for Patch fetch against an allowlisted set of repos, not for Search-based Inbox.

### GraphQL auth note

GraphQL accepts the same PAT / OAuth user token. Required scopes follow the data requested ([Forming calls with GraphQL / Authenticating](https://docs.github.com/en/graphql/guides/forming-calls-with-graphql)).

## Suggested `reviewdesk-core` wiring

1. Resolve token: `gh auth token` or configured PAT / `GH_TOKEN`.
2. Inbox: `GET /search/issues` with `q=is:open is:pr review-requested:@me` (optional `OR author:@me`), map items to domain Pull request rows (`repository_url` / `html_url` / `number` / `title` / `user` / `updated_at`).
3. On open: `GET /repos/{owner}/{repo}/pulls/{n}` with `Accept: application/vnd.github.diff`; pass body bytes/string into `DiffFile::parse`.
4. Optionally call `/pulls/{n}/files` for file-tree chrome without using it as the sole Patch source.
5. Handle search rate limits and diff truncation explicitly in UX later (called out as still open on the map).

## Sources (primary)

- https://docs.github.com/en/rest/search/search
- https://docs.github.com/en/search-github/searching-on-github/searching-issues-and-pull-requests
- https://docs.github.com/en/search-github/getting-started-with-searching-on-github/understanding-the-search-syntax
- https://docs.github.com/en/rest/pulls/pulls
- https://docs.github.com/en/rest/using-the-rest-api/getting-started-with-the-rest-api#media-types
- https://docs.github.com/en/repositories/creating-and-managing-repositories/repository-limits
- https://docs.github.com/en/graphql/guides/forming-calls-with-graphql
- https://docs.github.com/en/graphql/reference/unions#searchresultitem
- https://docs.github.com/en/graphql/overview/rate-limits-and-node-limits-for-the-graphql-api
- https://docs.github.com/en/apps/oauth-apps/building-oauth-apps/scopes-for-oauth-apps
- https://docs.github.com/en/authentication/keeping-your-account-and-data-secure/managing-your-personal-access-tokens
- https://docs.github.com/en/rest/authentication/endpoints-available-for-fine-grained-personal-access-tokens
- https://docs.github.com/en/rest/authentication/permissions-required-for-fine-grained-personal-access-tokens
- https://cli.github.com/manual/gh_auth_login
- https://cli.github.com/manual/gh_auth_token
