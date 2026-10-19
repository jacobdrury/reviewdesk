use std::time::Duration;

use async_trait::async_trait;
use reqwest::header::HeaderMap;
use reqwest::{Client, StatusCode};
use reviewdesk_core::{
    Credentials, MergeMethod, NotificationListOptions, NotificationPage, NotificationThreadId,
    Patch, PendingDiffReviewComment, PipelinesSnapshot, Provider, ProviderError, PullRequestDetail,
    PullRequestId, PullRequestListItem, PullRequestListOptions, PullRequestThreads, RerunTarget,
    ReviewSubmitEvent, ThreadId, ThreadStatus, TimelineConversationComment, codeowners_breakdown,
    parse_codeowners,
};
use serde_json::json;

use crate::error_map::from_status;
use crate::id::parse_pull_request_id;
use crate::map::{
    GraphQlResponse, PullRequestData, PullRequestNodeIdData, SearchData, ViewerData,
    map_pull_request_detail, map_pull_request_list_item, merge_method_to_graphql,
    merge_method_to_rest,
};
use crate::notification::{NotificationThreadPayload, map_notification_thread};
use crate::pipelines_map::{PipelinesData, map_pipelines};
use crate::query::pull_request_search_queries;
use crate::threads_map::{
    ReviewConnection, ReviewThreadConnection, ReviewThreadNode, ReviewThreadsData,
    ReviewThreadsPullRequest, diff_side_to_rest, find_viewer_pending_review_id,
    map_pull_request_threads, parse_review_comment_id, parse_review_create_id,
    review_submit_event_to_rest,
};

const GRAPHQL_URL: &str = "https://api.github.com/graphql";
const API_BASE: &str = "https://api.github.com";
const USER_AGENT: &str = "reviewdesk";
const REST_ACCEPT: &str = "application/vnd.github+json";

const VIEWER_QUERY: &str = r#"
query Viewer {
  viewer { login }
}
"#;

const PULL_REQUEST_SEARCH_QUERY: &str = r#"
query PullRequestSearch($q: String!) {
  search(query: $q, type: ISSUE, first: 50) {
    nodes {
      ... on PullRequest {
        number
        title
        url
        updatedAt
        isDraft
        author { login avatarUrl }
        repository {
          nameWithOwner
          owner { login }
          name
        }
        labels(first: 20) { nodes { name color } }
        reviewDecision
        viewerLatestReview { state }
      }
    }
  }
}
"#;

const PULL_REQUEST_PIPELINES_QUERY: &str = r#"
query PullRequestPipelines($owner: String!, $name: String!, $number: Int!) {
  repository(owner: $owner, name: $name) {
    pullRequest(number: $number) {
      statusCheckRollup {
        contexts(first: 100) {
          nodes {
            __typename
            ... on CheckRun {
              databaseId
              name
              status
              conclusion
              startedAt
              completedAt
              isRequired(pullRequestNumber: $number)
              checkSuite {
                id
                app { name }
                workflowRun {
                  databaseId
                  runNumber
                  createdAt
                  workflow { name }
                }
              }
            }
            ... on StatusContext {
              id
              context
              state
              createdAt
              isRequired(pullRequestNumber: $number)
            }
          }
        }
      }
    }
  }
}
"#;

const PULL_REQUEST_QUERY: &str = r#"
query PullRequest($owner: String!, $name: String!, $number: Int!) {
  repository(owner: $owner, name: $name) {
    autoMergeAllowed
    pullRequest(number: $number) {
      number
      title
      url
      updatedAt
      isDraft
      body
      author { login avatarUrl }
      repository {
        nameWithOwner
        owner { login }
        name
      }
      labels(first: 20) { nodes { name color } }
      reviewDecision
      viewerLatestReview { state }
      reviewRequests(first: 20) {
        nodes {
          requestedReviewer {
            ... on User { login avatarUrl }
          }
        }
      }
      headRefName
      baseRefName
      mergeable
      state
      autoMergeRequest { enabledAt }
      assignees(first: 20) {
        nodes { login avatarUrl }
      }
    }
  }
}
"#;

const PULL_REQUEST_NODE_ID_QUERY: &str = r#"
query PullRequestNodeId($owner: String!, $name: String!, $number: Int!) {
  repository(owner: $owner, name: $name) {
    pullRequest(number: $number) { id }
  }
}
"#;

const ENABLE_AUTO_MERGE_MUTATION: &str = r#"
mutation EnablePullRequestAutoMerge($input: EnablePullRequestAutoMergeInput!) {
  enablePullRequestAutoMerge(input: $input) {
    clientMutationId
  }
}
"#;

const DISABLE_AUTO_MERGE_MUTATION: &str = r#"
mutation DisablePullRequestAutoMerge($input: DisablePullRequestAutoMergeInput!) {
  disablePullRequestAutoMerge(input: $input) {
    clientMutationId
  }
}
"#;

const CODEOWNERS_PATHS: [&str; 3] = [".github/CODEOWNERS", "CODEOWNERS", "docs/CODEOWNERS"];

const PULL_REQUEST_REVIEW_THREADS_QUERY: &str = r#"
query PullRequestReviewThreads($owner: String!, $name: String!, $number: Int!, $after: String) {
  repository(owner: $owner, name: $name) {
    pullRequest(number: $number) {
      headRefOid
      reviews(states: [PENDING], first: 5) {
        nodes { id databaseId }
      }
      reviewThreads(first: 100, after: $after) {
        pageInfo { hasNextPage endCursor }
        nodes {
          id
          isResolved
          isOutdated
          path
          line
          startLine
          diffSide
          startDiffSide
          subjectType
          comments(first: 100) {
            nodes {
              id
              databaseId
              body
              state
              author { login }
            }
          }
        }
      }
    }
  }
}
"#;

const RESOLVE_REVIEW_THREAD: &str = r#"
mutation ResolveReviewThread($threadId: ID!) {
  resolveReviewThread(input: { threadId: $threadId }) {
    thread { id isResolved }
  }
}
"#;

const UNRESOLVE_REVIEW_THREAD: &str = r#"
mutation UnresolveReviewThread($threadId: ID!) {
  unresolveReviewThread(input: { threadId: $threadId }) {
    thread { id isResolved }
  }
}
"#;

pub struct GitHubProvider {
    client: Client,
    credentials: Credentials,
}

impl GitHubProvider {
    async fn fetch_pull_request_node_id(
        &self,
        id: &PullRequestId,
    ) -> Result<String, ProviderError> {
        let (owner, name, number) = parse_pull_request_id(id)?;
        let data: PullRequestNodeIdData = self
            .graphql(
                PULL_REQUEST_NODE_ID_QUERY,
                json!({
                    "owner": owner,
                    "name": name,
                    "number": number as i64,
                }),
            )
            .await?;
        data.repository
            .and_then(|r| r.pull_request)
            .and_then(|pr| pr.id)
            .ok_or(ProviderError::NotFound)
    }

    pub fn new(credentials: Credentials) -> Result<Self, ProviderError> {
        let client = Client::builder()
            .user_agent(USER_AGENT)
            .build()
            .map_err(|e| ProviderError::other(e.to_string()))?;
        Ok(Self {
            client,
            credentials,
        })
    }

    async fn graphql<T: serde::de::DeserializeOwned>(
        &self,
        query: &str,
        variables: serde_json::Value,
    ) -> Result<T, ProviderError> {
        let response = self
            .client
            .post(GRAPHQL_URL)
            .bearer_auth(self.credentials.token())
            .json(&json!({ "query": query, "variables": variables }))
            .send()
            .await
            .map_err(|e| ProviderError::other(e.to_string()))?;

        let status = response.status();
        let body = response
            .text()
            .await
            .map_err(|e| ProviderError::other(e.to_string()))?;

        if !status.is_success() {
            return Err(from_status(status.as_u16(), &body));
        }

        let parsed: GraphQlResponse<T> = serde_json::from_str(&body)
            .map_err(|e| ProviderError::other(format!("graphql decode: {e}")))?;

        if let Some(errors) = parsed.errors {
            let message = errors
                .into_iter()
                .map(|e| e.message)
                .collect::<Vec<_>>()
                .join("; ");
            let lower = message.to_lowercase();
            if lower.contains("bad credentials")
                || lower.contains("requires authentication")
                || lower.contains("unauthorized")
            {
                return Err(ProviderError::Auth);
            }
            if lower.contains("rate limit") {
                return Err(ProviderError::RateLimit);
            }
            if lower.contains("could not resolve") || lower.contains("not found") {
                return Err(ProviderError::NotFound);
            }
            return Err(ProviderError::other(message));
        }

        parsed
            .data
            .ok_or_else(|| ProviderError::other("graphql response missing data"))
    }

    async fn load_review_threads(
        &self,
        id: &PullRequestId,
    ) -> Result<PullRequestThreads, ProviderError> {
        let (owner, name, number) = parse_pull_request_id(id)?;
        let mut after: Option<String> = None;
        let mut head_ref_oid = None;
        let mut pending_nodes = Vec::new();
        let mut all_thread_nodes: Vec<ReviewThreadNode> = Vec::new();

        loop {
            let data: ReviewThreadsData = self
                .graphql(
                    PULL_REQUEST_REVIEW_THREADS_QUERY,
                    json!({
                        "owner": owner,
                        "name": name,
                        "number": number as i64,
                        "after": after,
                    }),
                )
                .await?;
            let pull_request = data
                .repository
                .and_then(|repository| repository.pull_request)
                .ok_or(ProviderError::NotFound)?;

            if head_ref_oid.is_none() {
                head_ref_oid = pull_request.head_ref_oid.clone();
            }
            if pending_nodes.is_empty() {
                pending_nodes = pull_request.reviews.nodes;
            }
            for thread in pull_request.review_threads.nodes.into_iter().flatten() {
                all_thread_nodes.push(thread);
            }
            if !pull_request.review_threads.page_info.has_next_page {
                break;
            }
            after = pull_request.review_threads.page_info.end_cursor;
        }

        let merged = ReviewThreadsPullRequest {
            head_ref_oid,
            review_threads: ReviewThreadConnection {
                page_info: crate::threads_map::PageInfo {
                    has_next_page: false,
                    end_cursor: None,
                },
                nodes: all_thread_nodes.into_iter().map(Some).collect(),
            },
            reviews: ReviewConnection {
                nodes: pending_nodes,
            },
        };
        Ok(map_pull_request_threads(id, &merged))
    }

    async fn head_ref_oid(&self, id: &PullRequestId) -> Result<String, ProviderError> {
        let (owner, name, number) = parse_pull_request_id(id)?;
        let data: ReviewThreadsData = self
            .graphql(
                PULL_REQUEST_REVIEW_THREADS_QUERY,
                json!({
                    "owner": owner,
                    "name": name,
                    "number": number as i64,
                    "after": serde_json::Value::Null,
                }),
            )
            .await?;
        data.repository
            .and_then(|repository| repository.pull_request)
            .and_then(|pull_request| pull_request.head_ref_oid)
            .ok_or(ProviderError::NotFound)
    }

    async fn ensure_pending_review_id(
        &self,
        owner: &str,
        repo: &str,
        number: u64,
        viewer_login: &str,
        head_commit: &str,
    ) -> Result<u64, ProviderError> {
        let list_url = format!("{API_BASE}/repos/{owner}/{repo}/pulls/{number}/reviews");
        let response = self
            .client
            .get(&list_url)
            .bearer_auth(self.credentials.token())
            .header("Accept", REST_ACCEPT)
            .send()
            .await
            .map_err(|e| ProviderError::other(e.to_string()))?;
        let status = response.status();
        let body = response
            .text()
            .await
            .map_err(|e| ProviderError::other(e.to_string()))?;
        if !status.is_success() {
            return Err(from_status(status.as_u16(), &body));
        }
        if let Some(id) = find_viewer_pending_review_id(&body, viewer_login)? {
            return Ok(id);
        }

        let create_url = format!("{API_BASE}/repos/{owner}/{repo}/pulls/{number}/reviews");
        let response = self
            .client
            .post(&create_url)
            .bearer_auth(self.credentials.token())
            .header("Accept", REST_ACCEPT)
            .json(&json!({ "commit_id": head_commit }))
            .send()
            .await
            .map_err(|e| ProviderError::other(e.to_string()))?;
        let status = response.status();
        let body = response
            .text()
            .await
            .map_err(|e| ProviderError::other(e.to_string()))?;
        if !status.is_success() {
            return Err(from_status(status.as_u16(), &body));
        }
        parse_review_create_id(&body)
    }

    async fn post_pending_diff_comment(
        &self,
        owner: &str,
        repo: &str,
        number: u64,
        review_id: u64,
        head_commit: &str,
        comment: &PendingDiffReviewComment,
    ) -> Result<(), ProviderError> {
        let side = diff_side_to_rest(comment.anchor.side);
        let mut payload = json!({
            "body": comment.body,
            "commit_id": head_commit,
            "path": comment.anchor.path,
            "line": comment.anchor.end_line,
            "side": side,
            "pull_request_review_id": review_id,
        });
        if comment.anchor.start_line != comment.anchor.end_line {
            payload["start_line"] = json!(comment.anchor.start_line);
            payload["start_side"] = json!(side);
        }

        let url = format!("{API_BASE}/repos/{owner}/{repo}/pulls/{number}/comments");
        let response = self
            .client
            .post(&url)
            .bearer_auth(self.credentials.token())
            .header("Accept", REST_ACCEPT)
            .json(&payload)
            .send()
            .await
            .map_err(|e| ProviderError::other(e.to_string()))?;
        let status = response.status();
        let body = response
            .text()
            .await
            .map_err(|e| ProviderError::other(e.to_string()))?;
        if !status.is_success() {
            return Err(from_status(status.as_u16(), &body));
        }
        let _ = parse_review_comment_id(&body)?;
        Ok(())
    }

    async fn post_pending_reply(
        &self,
        owner: &str,
        repo: &str,
        number: u64,
        root_comment_id: u64,
        body: &str,
    ) -> Result<(), ProviderError> {
        let url = format!(
            "{API_BASE}/repos/{owner}/{repo}/pulls/{number}/comments/{root_comment_id}/replies"
        );
        let response = self
            .client
            .post(&url)
            .bearer_auth(self.credentials.token())
            .header("Accept", REST_ACCEPT)
            .json(&json!({ "body": body }))
            .send()
            .await
            .map_err(|e| ProviderError::other(e.to_string()))?;
        let status = response.status();
        let body_text = response
            .text()
            .await
            .map_err(|e| ProviderError::other(e.to_string()))?;
        if !status.is_success() {
            return Err(from_status(status.as_u16(), &body_text));
        }
        let _ = parse_review_comment_id(&body_text)?;
        Ok(())
    }

    async fn find_pending_review_id(
        &self,
        owner: &str,
        repo: &str,
        number: u64,
        viewer_login: &str,
    ) -> Result<Option<u64>, ProviderError> {
        let list_url = format!("{API_BASE}/repos/{owner}/{repo}/pulls/{number}/reviews");
        let response = self
            .client
            .get(&list_url)
            .bearer_auth(self.credentials.token())
            .header("Accept", REST_ACCEPT)
            .send()
            .await
            .map_err(|e| ProviderError::other(e.to_string()))?;
        let status = response.status();
        let body = response
            .text()
            .await
            .map_err(|e| ProviderError::other(e.to_string()))?;
        if !status.is_success() {
            return Err(from_status(status.as_u16(), &body));
        }
        find_viewer_pending_review_id(&body, viewer_login)
    }
}

#[async_trait]
impl Provider for GitHubProvider {
    async fn current_user(&self) -> Result<String, ProviderError> {
        let data: ViewerData = self.graphql(VIEWER_QUERY, json!({})).await?;
        Ok(data.viewer.login)
    }

    async fn list_pull_requests(
        &self,
        options: &PullRequestListOptions,
    ) -> Result<Vec<PullRequestListItem>, ProviderError> {
        let mut items = Vec::new();
        let mut seen = std::collections::HashSet::new();
        for q in pull_request_search_queries(options) {
            let data: SearchData = self
                .graphql(PULL_REQUEST_SEARCH_QUERY, json!({ "q": q }))
                .await?;
            for node in data.search.nodes.into_iter().flatten() {
                let item = map_pull_request_list_item(&node)?;
                if seen.insert(item.id.clone()) {
                    items.push(item);
                }
            }
        }
        Ok(items)
    }

    async fn fetch_patch(&self, id: &PullRequestId) -> Result<Patch, ProviderError> {
        let (owner, repo, number) = parse_pull_request_id(id)?;
        let url = format!("{API_BASE}/repos/{owner}/{repo}/pulls/{number}");

        let response = self
            .client
            .get(&url)
            .bearer_auth(self.credentials.token())
            .header("Accept", "application/vnd.github.diff")
            .send()
            .await
            .map_err(|e| ProviderError::other(e.to_string()))?;

        let status = response.status();
        let body = response
            .text()
            .await
            .map_err(|e| ProviderError::other(e.to_string()))?;

        match status {
            StatusCode::OK => Ok(Patch::new(body)),
            other => Err(from_status(other.as_u16(), &body)),
        }
    }

    async fn fetch_pull_request(
        &self,
        id: &PullRequestId,
    ) -> Result<PullRequestDetail, ProviderError> {
        let (owner, name, number) = parse_pull_request_id(id)?;
        let data: PullRequestData = self
            .graphql(
                PULL_REQUEST_QUERY,
                json!({
                    "owner": owner,
                    "name": name,
                    "number": number as i64,
                }),
            )
            .await?;

        let repository = data.repository.ok_or(ProviderError::NotFound)?;
        let allow_auto_merge = repository.auto_merge_allowed.unwrap_or(false);
        let node = repository.pull_request.ok_or(ProviderError::NotFound)?;
        let mut detail = map_pull_request_detail(&node, allow_auto_merge)?;
        enrich_codeowners(self, &mut detail).await;
        Ok(detail)
    }

    async fn set_pull_request_labels(
        &self,
        id: &PullRequestId,
        labels: &[String],
    ) -> Result<(), ProviderError> {
        let (owner, repo, number) = parse_pull_request_id(id)?;
        let url = format!("{API_BASE}/repos/{owner}/{repo}/issues/{number}/labels");
        let response = self
            .client
            .put(&url)
            .bearer_auth(self.credentials.token())
            .json(&json!({ "labels": labels }))
            .send()
            .await
            .map_err(|e| ProviderError::other(e.to_string()))?;
        map_write_response(response).await
    }

    async fn set_pull_request_assignees(
        &self,
        id: &PullRequestId,
        assignees: &[String],
    ) -> Result<(), ProviderError> {
        let (owner, repo, number) = parse_pull_request_id(id)?;
        let url = format!("{API_BASE}/repos/{owner}/{repo}/issues/{number}/assignees");
        let response = self
            .client
            .post(&url)
            .bearer_auth(self.credentials.token())
            .json(&json!({ "assignees": assignees }))
            .send()
            .await
            .map_err(|e| ProviderError::other(e.to_string()))?;
        map_write_response(response).await
    }

    async fn request_pull_request_reviewer(
        &self,
        id: &PullRequestId,
        login: &str,
    ) -> Result<(), ProviderError> {
        let (owner, repo, number) = parse_pull_request_id(id)?;
        let url = format!("{API_BASE}/repos/{owner}/{repo}/pulls/{number}/requested_reviewers");
        let response = self
            .client
            .post(&url)
            .bearer_auth(self.credentials.token())
            .json(&json!({ "reviewers": [login] }))
            .send()
            .await
            .map_err(|e| ProviderError::other(e.to_string()))?;
        map_write_response(response).await
    }

    async fn remove_pull_request_reviewer(
        &self,
        id: &PullRequestId,
        login: &str,
    ) -> Result<(), ProviderError> {
        let (owner, repo, number) = parse_pull_request_id(id)?;
        let url = format!("{API_BASE}/repos/{owner}/{repo}/pulls/{number}/requested_reviewers");
        let response = self
            .client
            .delete(&url)
            .bearer_auth(self.credentials.token())
            .json(&json!({ "reviewers": [login] }))
            .send()
            .await
            .map_err(|e| ProviderError::other(e.to_string()))?;
        map_write_response(response).await
    }

    async fn rerequest_pull_request_reviewer(
        &self,
        id: &PullRequestId,
        login: &str,
    ) -> Result<(), ProviderError> {
        self.remove_pull_request_reviewer(id, login).await?;
        self.request_pull_request_reviewer(id, login).await
    }

    async fn set_pull_request_draft(
        &self,
        id: &PullRequestId,
        is_draft: bool,
    ) -> Result<(), ProviderError> {
        let (owner, repo, number) = parse_pull_request_id(id)?;
        let url = format!("{API_BASE}/repos/{owner}/{repo}/pulls/{number}");
        let response = self
            .client
            .patch(&url)
            .bearer_auth(self.credentials.token())
            .json(&json!({ "draft": is_draft }))
            .send()
            .await
            .map_err(|e| ProviderError::other(e.to_string()))?;
        map_write_response(response).await
    }

    async fn set_pull_request_base(
        &self,
        id: &PullRequestId,
        base_ref: &str,
    ) -> Result<(), ProviderError> {
        let (owner, repo, number) = parse_pull_request_id(id)?;
        let url = format!("{API_BASE}/repos/{owner}/{repo}/pulls/{number}");
        let response = self
            .client
            .patch(&url)
            .bearer_auth(self.credentials.token())
            .json(&json!({ "base": base_ref }))
            .send()
            .await
            .map_err(|e| ProviderError::other(e.to_string()))?;
        map_write_response(response).await
    }

    async fn fetch_pipelines(
        &self,
        id: &PullRequestId,
    ) -> Result<PipelinesSnapshot, ProviderError> {
        let (owner, name, number) = parse_pull_request_id(id)?;
        let data: PipelinesData = self
            .graphql(
                PULL_REQUEST_PIPELINES_QUERY,
                json!({
                    "owner": owner,
                    "name": name,
                    "number": number as i64,
                }),
            )
            .await?;
        map_pipelines(data, number)
    }

    async fn rerun_check(
        &self,
        id: &PullRequestId,
        target: &RerunTarget,
    ) -> Result<(), ProviderError> {
        let (owner, repo, _number) = parse_pull_request_id(id)?;
        let url = match target {
            RerunTarget::WorkflowRun { run_id } => {
                format!("{API_BASE}/repos/{owner}/{repo}/actions/runs/{run_id}/rerun")
            }
            RerunTarget::WorkflowJob { job_id } => {
                format!("{API_BASE}/repos/{owner}/{repo}/actions/jobs/{job_id}/rerun")
            }
        };

        let response = self
            .client
            .post(&url)
            .bearer_auth(self.credentials.token())
            .header("Accept", REST_ACCEPT)
            .send()
            .await
            .map_err(|e| ProviderError::other(e.to_string()))?;
        ensure_success(response).await
    }

    async fn merge_pull_request(
        &self,
        id: &PullRequestId,
        method: MergeMethod,
    ) -> Result<(), ProviderError> {
        let (owner, repo, number) = parse_pull_request_id(id)?;
        let url = format!("{API_BASE}/repos/{owner}/{repo}/pulls/{number}/merge");
        let response = self
            .client
            .put(&url)
            .bearer_auth(self.credentials.token())
            .json(&json!({ "merge_method": merge_method_to_rest(method) }))
            .send()
            .await
            .map_err(|e| ProviderError::other(e.to_string()))?;
        map_write_response(response).await
    }

    async fn set_pull_request_auto_merge(
        &self,
        id: &PullRequestId,
        enabled: bool,
        method: MergeMethod,
    ) -> Result<(), ProviderError> {
        let node_id = self.fetch_pull_request_node_id(id).await?;
        if enabled {
            let _: serde_json::Value = self
                .graphql(
                    ENABLE_AUTO_MERGE_MUTATION,
                    json!({
                        "input": {
                            "pullRequestId": node_id,
                            "mergeMethod": merge_method_to_graphql(method),
                        }
                    }),
                )
                .await?;
        } else {
            let _: serde_json::Value = self
                .graphql(
                    DISABLE_AUTO_MERGE_MUTATION,
                    json!({ "input": { "pullRequestId": node_id } }),
                )
                .await?;
        }
        Ok(())
    }

    async fn list_notifications(
        &self,
        options: &NotificationListOptions,
    ) -> Result<NotificationPage, ProviderError> {
        let url = format!("{API_BASE}/notifications?all={}", options.all);
        let mut request = self
            .client
            .get(&url)
            .bearer_auth(self.credentials.token())
            .header("Accept", REST_ACCEPT);
        if let Some(since) = &options.if_modified_since {
            request = request.header("If-Modified-Since", since);
        }

        let response = request
            .send()
            .await
            .map_err(|e| ProviderError::other(e.to_string()))?;

        let status = response.status();
        let poll_interval = poll_interval_from_headers(response.headers());
        let last_modified = last_modified_from_headers(response.headers());

        if status == StatusCode::NOT_MODIFIED {
            return Ok(NotificationPage {
                threads: Vec::new(),
                poll_interval,
                last_modified,
                not_modified: true,
            });
        }

        let body = response
            .text()
            .await
            .map_err(|e| ProviderError::other(e.to_string()))?;

        if !status.is_success() {
            return Err(from_status(status.as_u16(), &body));
        }

        let payloads: Vec<NotificationThreadPayload> = serde_json::from_str(&body)
            .map_err(|e| ProviderError::other(format!("notifications decode: {e}")))?;
        let threads = payloads
            .iter()
            .filter_map(map_notification_thread)
            .collect();

        Ok(NotificationPage {
            threads,
            poll_interval,
            last_modified,
            not_modified: false,
        })
    }

    async fn mark_notification_read(&self, id: &NotificationThreadId) -> Result<(), ProviderError> {
        let url = format!("{API_BASE}/notifications/threads/{}", id.as_str());
        let response = self
            .client
            .patch(&url)
            .bearer_auth(self.credentials.token())
            .header("Accept", REST_ACCEPT)
            .send()
            .await
            .map_err(|e| ProviderError::other(e.to_string()))?;
        ensure_success(response).await
    }

    async fn mark_notification_done(&self, id: &NotificationThreadId) -> Result<(), ProviderError> {
        let url = format!("{API_BASE}/notifications/threads/{}", id.as_str());
        let response = self
            .client
            .delete(&url)
            .bearer_auth(self.credentials.token())
            .header("Accept", REST_ACCEPT)
            .send()
            .await
            .map_err(|e| ProviderError::other(e.to_string()))?;
        ensure_success(response).await
    }

    async fn fetch_pull_request_threads(
        &self,
        id: &PullRequestId,
    ) -> Result<PullRequestThreads, ProviderError> {
        self.load_review_threads(id).await
    }

    async fn add_pending_diff_review_comment(
        &self,
        id: &PullRequestId,
        comment: PendingDiffReviewComment,
    ) -> Result<PullRequestThreads, ProviderError> {
        let (owner, repo, number) = parse_pull_request_id(id)?;
        let viewer = self.current_user().await?;
        let head_commit = self.head_ref_oid(id).await?;
        let review_id = self
            .ensure_pending_review_id(&owner, &repo, number, &viewer, &head_commit)
            .await?;

        if let Some(thread_id) = &comment.reply_to_thread {
            let snapshot = self.load_review_threads(id).await?;
            let root_id = snapshot
                .threads
                .iter()
                .find(|thread| &thread.id == thread_id)
                .and_then(|thread| thread.comments.first())
                .map(|c| c.id.as_str())
                .ok_or(ProviderError::NotFound)?;
            let root_comment_id = root_id
                .parse::<u64>()
                .map_err(|_| ProviderError::other("review comment id is not numeric"))?;
            self.post_pending_reply(&owner, &repo, number, root_comment_id, &comment.body)
                .await?;
        } else {
            self.post_pending_diff_comment(
                &owner,
                &repo,
                number,
                review_id,
                &head_commit,
                &comment,
            )
            .await?;
        }

        self.load_review_threads(id).await
    }

    async fn submit_pending_review(
        &self,
        id: &PullRequestId,
        event: ReviewSubmitEvent,
        summary_body: Option<String>,
    ) -> Result<PullRequestThreads, ProviderError> {
        let (owner, repo, number) = parse_pull_request_id(id)?;
        let viewer = self.current_user().await?;
        let snapshot = self.load_review_threads(id).await?;
        let review_id = snapshot
            .pending_review
            .as_ref()
            .and_then(|pending| pending.provider_review_id.as_ref())
            .and_then(|raw| raw.parse::<u64>().ok())
            .or(self
                .find_pending_review_id(&owner, &repo, number, &viewer)
                .await?);
        let review_id =
            review_id.ok_or_else(|| ProviderError::other("no pending review to submit"))?;

        let url =
            format!("{API_BASE}/repos/{owner}/{repo}/pulls/{number}/reviews/{review_id}/events");
        let mut payload = json!({ "event": review_submit_event_to_rest(event) });
        if let Some(body) = summary_body {
            payload["body"] = json!(body);
        }
        let response = self
            .client
            .post(&url)
            .bearer_auth(self.credentials.token())
            .header("Accept", REST_ACCEPT)
            .json(&payload)
            .send()
            .await
            .map_err(|e| ProviderError::other(e.to_string()))?;
        let status = response.status();
        let body = response
            .text()
            .await
            .map_err(|e| ProviderError::other(e.to_string()))?;
        if !status.is_success() {
            return Err(from_status(status.as_u16(), &body));
        }
        self.load_review_threads(id).await
    }

    async fn discard_pending_review(
        &self,
        id: &PullRequestId,
    ) -> Result<PullRequestThreads, ProviderError> {
        let (owner, repo, number) = parse_pull_request_id(id)?;
        let viewer = self.current_user().await?;
        let review_id = self
            .find_pending_review_id(&owner, &repo, number, &viewer)
            .await?
            .ok_or_else(|| ProviderError::other("no pending review to discard"))?;

        let url = format!("{API_BASE}/repos/{owner}/{repo}/pulls/{number}/reviews/{review_id}");
        let response = self
            .client
            .delete(&url)
            .bearer_auth(self.credentials.token())
            .header("Accept", REST_ACCEPT)
            .send()
            .await
            .map_err(|e| ProviderError::other(e.to_string()))?;
        let status = response.status();
        let body = response
            .text()
            .await
            .map_err(|e| ProviderError::other(e.to_string()))?;
        if !status.is_success() {
            return Err(from_status(status.as_u16(), &body));
        }
        self.load_review_threads(id).await
    }

    async fn set_thread_status(
        &self,
        id: &PullRequestId,
        thread_id: &ThreadId,
        status: ThreadStatus,
    ) -> Result<PullRequestThreads, ProviderError> {
        let _ = id;
        let query = match status {
            ThreadStatus::Resolved => RESOLVE_REVIEW_THREAD,
            ThreadStatus::Open => UNRESOLVE_REVIEW_THREAD,
            ThreadStatus::Outdated => {
                return Err(ProviderError::other(
                    "outdated threads cannot be resolved via GitHub",
                ));
            }
        };
        self.graphql::<serde_json::Value>(query, json!({ "threadId": thread_id.as_str() }))
            .await?;
        self.load_review_threads(id).await
    }

    async fn publish_timeline_comment(
        &self,
        id: &PullRequestId,
        comment: TimelineConversationComment,
    ) -> Result<PullRequestThreads, ProviderError> {
        let (owner, repo, number) = parse_pull_request_id(id)?;
        let url = format!("{API_BASE}/repos/{owner}/{repo}/issues/{number}/comments");
        let response = self
            .client
            .post(&url)
            .bearer_auth(self.credentials.token())
            .header("Accept", REST_ACCEPT)
            .json(&json!({ "body": comment.body }))
            .send()
            .await
            .map_err(|e| ProviderError::other(e.to_string()))?;
        let status = response.status();
        let body = response
            .text()
            .await
            .map_err(|e| ProviderError::other(e.to_string()))?;
        if !status.is_success() {
            return Err(from_status(status.as_u16(), &body));
        }
        self.load_review_threads(id).await
    }
}

fn poll_interval_from_headers(headers: &HeaderMap) -> Option<Duration> {
    headers
        .get("x-poll-interval")
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.parse::<u64>().ok())
        .map(Duration::from_secs)
}

fn last_modified_from_headers(headers: &HeaderMap) -> Option<String> {
    headers
        .get("last-modified")
        .and_then(|value| value.to_str().ok())
        .map(|value| value.to_string())
}

async fn ensure_success(response: reqwest::Response) -> Result<(), ProviderError> {
    let status = response.status();
    if status.is_success() {
        return Ok(());
    }
    let body = response.text().await.unwrap_or_default();
    Err(from_status(status.as_u16(), &body))
}

async fn map_write_response(response: reqwest::Response) -> Result<(), ProviderError> {
    let status = response.status();
    let body = response
        .text()
        .await
        .map_err(|e| ProviderError::other(e.to_string()))?;
    if status.is_success() {
        return Ok(());
    }
    Err(from_status(status.as_u16(), &body))
}

async fn enrich_codeowners(provider: &GitHubProvider, detail: &mut PullRequestDetail) {
    let Ok((owner, repo, number)) = parse_pull_request_id(&detail.id) else {
        return;
    };
    let paths = match fetch_changed_paths(provider, &owner, &repo, number).await {
        Ok(paths) => paths,
        Err(_) => return,
    };
    let source = match fetch_codeowners_source(provider, &owner, &repo, &detail.base_ref).await {
        Ok(source) => source,
        Err(_) => return,
    };
    let rules = parse_codeowners(&source);
    detail.codeowners_rules = codeowners_breakdown(&rules, &paths);
}

async fn fetch_changed_paths(
    provider: &GitHubProvider,
    owner: &str,
    repo: &str,
    number: u64,
) -> Result<Vec<String>, ProviderError> {
    let url = format!("{API_BASE}/repos/{owner}/{repo}/pulls/{number}/files?per_page=100");
    let response = provider
        .client
        .get(&url)
        .bearer_auth(provider.credentials.token())
        .send()
        .await
        .map_err(|e| ProviderError::other(e.to_string()))?;
    let status = response.status();
    let body = response
        .text()
        .await
        .map_err(|e| ProviderError::other(e.to_string()))?;
    if !status.is_success() {
        return Err(from_status(status.as_u16(), &body));
    }
    let files: Vec<PullFileRow> = serde_json::from_str(&body)
        .map_err(|e| ProviderError::other(format!("pull files decode: {e}")))?;
    Ok(files.into_iter().map(|f| f.filename).collect())
}

#[derive(serde::Deserialize)]
struct PullFileRow {
    filename: String,
}

async fn fetch_codeowners_source(
    provider: &GitHubProvider,
    owner: &str,
    repo: &str,
    base_ref: &str,
) -> Result<String, ProviderError> {
    if base_ref.is_empty() {
        return Err(ProviderError::NotFound);
    }
    for path in CODEOWNERS_PATHS {
        let url = format!("{API_BASE}/repos/{owner}/{repo}/contents/{path}?ref={base_ref}");
        let response = provider
            .client
            .get(&url)
            .bearer_auth(provider.credentials.token())
            .header("Accept", "application/vnd.github.raw")
            .send()
            .await
            .map_err(|e| ProviderError::other(e.to_string()))?;
        if response.status() == StatusCode::NOT_FOUND {
            continue;
        }
        let status = response.status();
        let body = response
            .text()
            .await
            .map_err(|e| ProviderError::other(e.to_string()))?;
        if status.is_success() {
            return Ok(body);
        }
        return Err(from_status(status.as_u16(), &body));
    }
    Err(ProviderError::NotFound)
}
