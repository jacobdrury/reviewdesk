use chrono::{DateTime, Utc};
use reviewdesk_core::{
    ProviderError, PullRequestDetail, PullRequestLabel, PullRequestListItem, ReviewState, Reviewer,
};
use serde::Deserialize;

use crate::id::pull_request_id;

#[derive(Debug, Deserialize)]
pub struct GraphQlResponse<T> {
    pub data: Option<T>,
    pub errors: Option<Vec<GraphQlError>>,
}

#[derive(Debug, Deserialize)]
pub struct GraphQlError {
    pub message: String,
}

#[derive(Debug, Deserialize)]
pub struct ViewerData {
    pub viewer: Viewer,
}

#[derive(Debug, Deserialize)]
pub struct Viewer {
    pub login: String,
}

#[derive(Debug, Deserialize)]
pub struct SearchData {
    pub search: SearchConnection,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SearchConnection {
    pub nodes: Vec<Option<PullRequestNode>>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PullRequestNode {
    pub number: u64,
    pub title: String,
    pub url: String,
    pub updated_at: DateTime<Utc>,
    pub is_draft: bool,
    pub author: Option<Actor>,
    pub repository: Repository,
    pub labels: LabelConnection,
    pub review_decision: Option<String>,
    pub viewer_latest_review: Option<LatestReview>,
    pub body: Option<String>,
    pub review_requests: Option<ReviewRequestConnection>,
}

#[derive(Debug, Deserialize)]
pub struct Actor {
    pub login: String,
    #[serde(rename = "avatarUrl")]
    pub avatar_url: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Repository {
    pub name_with_owner: String,
    pub owner: Owner,
    pub name: String,
}

#[derive(Debug, Deserialize)]
pub struct Owner {
    pub login: String,
}

#[derive(Debug, Deserialize)]
pub struct LabelConnection {
    pub nodes: Vec<LabelNode>,
}

#[derive(Debug, Deserialize)]
pub struct LabelNode {
    pub name: String,
    pub color: String,
}

#[derive(Debug, Deserialize)]
pub struct LatestReview {
    pub state: String,
}

#[derive(Debug, Deserialize)]
pub struct ReviewRequestConnection {
    pub nodes: Vec<ReviewRequestNode>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ReviewRequestNode {
    pub requested_reviewer: Option<RequestedReviewer>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RequestedReviewer {
    pub login: Option<String>,
    pub avatar_url: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct PullRequestData {
    pub repository: Option<RepositoryPull>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RepositoryPull {
    pub pull_request: Option<PullRequestNode>,
}

pub fn map_pull_request_list_item(
    node: &PullRequestNode,
) -> Result<PullRequestListItem, ProviderError> {
    let id = pull_request_id(
        &node.repository.owner.login,
        &node.repository.name,
        node.number,
    )?;

    let author = node.author.as_ref();
    Ok(PullRequestListItem {
        id,
        title: node.title.clone(),
        web_url: node.url.clone(),
        updated_at: node.updated_at,
        author_login: author
            .map(|a| a.login.clone())
            .unwrap_or_else(|| "unknown".into()),
        author_avatar_url: author.and_then(|a| a.avatar_url.clone()),
        repo_label: node.repository.name_with_owner.clone(),
        labels: node
            .labels
            .nodes
            .iter()
            .map(|l| PullRequestLabel {
                name: l.name.clone(),
                color: l.color.clone(),
            })
            .collect(),
        review_state: map_review_state(node),
        is_draft: node.is_draft,
    })
}

pub fn map_pull_request_detail(node: &PullRequestNode) -> Result<PullRequestDetail, ProviderError> {
    let item = map_pull_request_list_item(node)?;
    let reviewers = node
        .review_requests
        .as_ref()
        .map(|c| {
            c.nodes
                .iter()
                .filter_map(|n| {
                    let reviewer = n.requested_reviewer.as_ref()?;
                    let login = reviewer.login.clone()?;
                    Some(Reviewer {
                        login,
                        avatar_url: reviewer.avatar_url.clone(),
                    })
                })
                .collect()
        })
        .unwrap_or_default();

    Ok(PullRequestDetail {
        id: item.id,
        title: item.title,
        body: node.body.clone().unwrap_or_default(),
        author_login: item.author_login,
        author_avatar_url: item.author_avatar_url,
        is_draft: item.is_draft,
        labels: item.labels,
        web_url: item.web_url,
        reviewers,
    })
}

fn map_review_state(node: &PullRequestNode) -> ReviewState {
    if let Some(review) = &node.viewer_latest_review {
        return match review.state.as_str() {
            "APPROVED" => ReviewState::Approved,
            "CHANGES_REQUESTED" => ReviewState::ChangesRequested,
            "COMMENTED" => ReviewState::Commented,
            "DISMISSED" => ReviewState::Dismissed,
            "PENDING" => ReviewState::Pending,
            _ => ReviewState::Unknown,
        };
    }
    match node.review_decision.as_deref() {
        Some("APPROVED") => ReviewState::Approved,
        Some("CHANGES_REQUESTED") => ReviewState::ChangesRequested,
        Some("REVIEW_REQUIRED") => ReviewState::Pending,
        _ => ReviewState::Pending,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const PULL_REQUEST_SEARCH_FIXTURE: &str = r#"{
      "data": {
        "search": {
          "nodes": [
            {
              "number": 12,
              "title": "Add widgets",
              "url": "https://github.com/acme/widgets/pull/12",
              "updatedAt": "2026-10-17T12:00:00Z",
              "isDraft": false,
              "author": { "login": "alice", "avatarUrl": "https://avatars.example/alice" },
              "repository": {
                "nameWithOwner": "acme/widgets",
                "owner": { "login": "acme" },
                "name": "widgets"
              },
              "labels": { "nodes": [{ "name": "ready", "color": "0e8a16" }] },
              "reviewDecision": "REVIEW_REQUIRED",
              "viewerLatestReview": null
            }
          ]
        }
      }
    }"#;

    const DETAIL_FIXTURE: &str = r#"{
      "data": {
        "repository": {
          "pullRequest": {
            "number": 12,
            "title": "Add widgets",
            "url": "https://github.com/acme/widgets/pull/12",
            "updatedAt": "2026-10-17T12:00:00Z",
            "isDraft": false,
            "body": "Please review.",
            "author": { "login": "alice", "avatarUrl": "https://avatars.example/alice" },
            "repository": {
              "nameWithOwner": "acme/widgets",
              "owner": { "login": "acme" },
              "name": "widgets"
            },
            "labels": { "nodes": [] },
            "reviewDecision": "REVIEW_REQUIRED",
            "viewerLatestReview": null,
            "reviewRequests": {
              "nodes": [
                {
                  "requestedReviewer": {
                    "login": "bob",
                    "avatarUrl": "https://avatars.example/bob"
                  }
                }
              ]
            }
          }
        }
      }
    }"#;

    #[test]
    fn maps_pull_request_search_fixture() {
        let parsed: GraphQlResponse<SearchData> =
            serde_json::from_str(PULL_REQUEST_SEARCH_FIXTURE).unwrap();
        let data = parsed.data.unwrap();
        let node = data.search.nodes[0].as_ref().unwrap();
        let item = map_pull_request_list_item(node).unwrap();
        assert_eq!(item.id.as_str(), "acme/widgets#12");
        assert_eq!(item.title, "Add widgets");
        assert_eq!(item.author_login, "alice");
        assert_eq!(item.repo_label, "acme/widgets");
        assert_eq!(item.labels.len(), 1);
        assert_eq!(item.labels[0].name, "ready");
        assert!(!item.is_draft);
        assert_eq!(item.review_state, ReviewState::Pending);
    }

    #[test]
    fn maps_pull_request_detail_fixture() {
        let parsed: GraphQlResponse<PullRequestData> =
            serde_json::from_str(DETAIL_FIXTURE).unwrap();
        let node = parsed
            .data
            .unwrap()
            .repository
            .unwrap()
            .pull_request
            .unwrap();
        let detail = map_pull_request_detail(&node).unwrap();
        assert_eq!(detail.body, "Please review.");
        assert_eq!(detail.reviewers.len(), 1);
        assert_eq!(detail.reviewers[0].login, "bob");
    }
}
