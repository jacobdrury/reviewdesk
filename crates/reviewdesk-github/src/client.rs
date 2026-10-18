use async_trait::async_trait;
use reqwest::{Client, StatusCode};
use reviewdesk_core::{
    Credentials, InboxItem, InboxOptions, Patch, Provider, ProviderError, PullRequestDetail,
    PullRequestId,
};
use serde_json::json;

use crate::error_map::from_status;
use crate::id::parse_pull_request_id;
use crate::map::{
    GraphQlResponse, PullRequestData, SearchData, map_inbox_item, map_pull_request_detail,
};
use crate::query::inbox_search_query;

const GRAPHQL_URL: &str = "https://api.github.com/graphql";
const API_BASE: &str = "https://api.github.com";
const USER_AGENT: &str = "reviewdesk";

const INBOX_QUERY: &str = r#"
query Inbox($q: String!) {
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
    async fn list_inbox(&self, options: &InboxOptions) -> Result<Vec<InboxItem>, ProviderError> {
        let q = inbox_search_query(options);
        let data: SearchData = self.graphql(INBOX_QUERY, json!({ "q": q })).await?;
        let mut items = Vec::new();
        for node in data.search.nodes.into_iter().flatten() {
            items.push(map_inbox_item(&node)?);
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
}
