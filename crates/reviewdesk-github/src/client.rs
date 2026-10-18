use std::time::Duration;

use async_trait::async_trait;
use reqwest::header::HeaderMap;
use reqwest::{Client, StatusCode};
use reviewdesk_core::{
    Credentials, MergeMethod, NotificationListOptions, NotificationPage, NotificationThreadId,
    Patch, PendingDiffReviewComment, Provider, ProviderError, PullRequestDetail, PullRequestId,
    PullRequestListItem, PullRequestListOptions, PullRequestThreads, ReviewSubmitEvent,
    codeowners_breakdown, parse_codeowners,
};
use serde_json::json;

use crate::error_map::from_status;
use crate::id::parse_pull_request_id;
use crate::map::{
    GraphQlResponse, PullRequestData, PullRequestNodeIdData, SearchData, map_pull_request_detail,
    map_pull_request_list_item, merge_method_to_graphql, merge_method_to_rest,
};
use crate::notification::{NotificationThreadPayload, map_notification_thread};
use crate::query::pull_request_search_query;

const GRAPHQL_URL: &str = "https://api.github.com/graphql";
const API_BASE: &str = "https://api.github.com";
const USER_AGENT: &str = "reviewdesk";
const REST_ACCEPT: &str = "application/vnd.github+json";

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
}

#[async_trait]
impl Provider for GitHubProvider {
    async fn list_pull_requests(
        &self,
        options: &PullRequestListOptions,
    ) -> Result<Vec<PullRequestListItem>, ProviderError> {
        let q = pull_request_search_query(options);
        let data: SearchData = self
            .graphql(PULL_REQUEST_SEARCH_QUERY, json!({ "q": q }))
            .await?;
        let mut items = Vec::new();
        for node in data.search.nodes.into_iter().flatten() {
            items.push(map_pull_request_list_item(&node)?);
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
        _id: &PullRequestId,
    ) -> Result<PullRequestThreads, ProviderError> {
        Err(ProviderError::other(
            "GitHub review threads fetch is not implemented yet",
        ))
    }

    async fn add_pending_diff_review_comment(
        &self,
        _id: &PullRequestId,
        _comment: PendingDiffReviewComment,
    ) -> Result<PullRequestThreads, ProviderError> {
        Err(ProviderError::other(
            "GitHub pending review comments are not implemented yet",
        ))
    }

    async fn submit_pending_review(
        &self,
        _id: &PullRequestId,
        _event: ReviewSubmitEvent,
        _summary_body: Option<String>,
    ) -> Result<PullRequestThreads, ProviderError> {
        Err(ProviderError::other(
            "GitHub pending review submit is not implemented yet",
        ))
    }

    async fn discard_pending_review(
        &self,
        _id: &PullRequestId,
    ) -> Result<PullRequestThreads, ProviderError> {
        Err(ProviderError::other(
            "GitHub pending review discard is not implemented yet",
        ))
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
