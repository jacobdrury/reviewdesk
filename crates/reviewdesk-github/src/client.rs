use std::time::Duration;

use async_trait::async_trait;
use reqwest::header::HeaderMap;
use reqwest::{Client, StatusCode};
use reviewdesk_core::{
    Credentials, NotificationListOptions, NotificationPage, NotificationThreadId, Patch,
    PendingDiffReviewComment, Provider, ProviderError, PullRequestDetail, PullRequestId,
    PullRequestListItem, PullRequestListOptions, PullRequestThreads, ReviewSubmitEvent,
};
use serde_json::json;

use crate::error_map::from_status;
use crate::id::parse_pull_request_id;
use crate::map::{
    GraphQlResponse, PullRequestData, SearchData, ViewerData, map_pull_request_detail,
    map_pull_request_list_item,
};
use crate::notification::{NotificationThreadPayload, map_notification_thread};
use crate::query::pull_request_search_query;

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

const PULL_REQUEST_QUERY: &str = r#"
query PullRequest($owner: String!, $name: String!, $number: Int!) {
  repository(owner: $owner, name: $name) {
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
    }
  }
}
"#;

pub struct GitHubProvider {
    client: Client,
    credentials: Credentials,
}

impl GitHubProvider {
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
    async fn current_user(&self) -> Result<String, ProviderError> {
        let data: ViewerData = self.graphql(VIEWER_QUERY, json!({})).await?;
        Ok(data.viewer.login)
    }

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

        let node = data
            .repository
            .and_then(|r| r.pull_request)
            .ok_or(ProviderError::NotFound)?;
        map_pull_request_detail(&node)
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

    async fn set_thread_status(
        &self,
        _id: &PullRequestId,
        _thread_id: &reviewdesk_core::ThreadId,
        _status: reviewdesk_core::ThreadStatus,
    ) -> Result<PullRequestThreads, ProviderError> {
        Err(ProviderError::other(
            "GitHub thread resolve is not implemented yet",
        ))
    }

    async fn publish_timeline_comment(
        &self,
        _id: &PullRequestId,
        _comment: reviewdesk_core::TimelineConversationComment,
    ) -> Result<PullRequestThreads, ProviderError> {
        Err(ProviderError::other(
            "GitHub timeline comments are not implemented yet",
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
