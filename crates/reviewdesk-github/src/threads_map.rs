use reviewdesk_core::{
    DiffAnchor, DiffLineSide, PendingReview, ProviderError, PullRequestId, PullRequestThreads,
    ReviewComment, ReviewCommentId, ReviewSubmitEvent, Thread, ThreadId, ThreadStatus,
    suggestion_from_body,
};
use serde::Deserialize;

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ReviewThreadsData {
    pub repository: Option<ReviewThreadsRepository>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ReviewThreadsRepository {
    pub pull_request: Option<ReviewThreadsPullRequest>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ReviewThreadsPullRequest {
    pub head_ref_oid: Option<String>,
    pub review_threads: ReviewThreadConnection,
    pub reviews: ReviewConnection,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ReviewThreadConnection {
    pub page_info: PageInfo,
    pub nodes: Vec<Option<ReviewThreadNode>>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ReviewConnection {
    pub nodes: Vec<Option<PendingReviewNode>>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PendingReviewNode {
    pub id: String,
    pub database_id: Option<u64>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct PageInfo {
    pub has_next_page: bool,
    pub end_cursor: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ReviewThreadNode {
    pub id: String,
    pub is_resolved: bool,
    pub is_outdated: bool,
    pub path: Option<String>,
    pub line: Option<i64>,
    pub start_line: Option<i64>,
    pub diff_side: Option<String>,
    #[allow(dead_code)]
    pub start_diff_side: Option<String>,
    pub subject_type: Option<String>,
    pub comments: ReviewCommentConnection,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ReviewCommentConnection {
    pub nodes: Vec<Option<ReviewCommentNode>>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ReviewCommentNode {
    pub id: String,
    pub database_id: Option<u64>,
    pub body: String,
    pub state: Option<String>,
    pub author: Option<CommentAuthor>,
}

#[derive(Debug, Deserialize)]
pub struct CommentAuthor {
    pub login: Option<String>,
}

pub fn map_pull_request_threads(
    id: &PullRequestId,
    pull_request: &ReviewThreadsPullRequest,
) -> PullRequestThreads {
    let pending_review = pull_request
        .reviews
        .nodes
        .iter()
        .filter_map(|node| node.as_ref())
        .next()
        .map(|review| PendingReview {
            pull_request_id: id.clone(),
            provider_review_id: review
                .database_id
                .map(|n| n.to_string())
                .or_else(|| Some(review.id.clone())),
            summary_body: None,
        });

    let threads = pull_request
        .review_threads
        .nodes
        .iter()
        .filter_map(|node| node.as_ref())
        .map(map_review_thread)
        .collect();

    PullRequestThreads {
        threads,
        pending_review,
    }
}

fn map_review_thread(node: &ReviewThreadNode) -> Thread {
    let status = if node.is_outdated {
        ThreadStatus::Outdated
    } else if node.is_resolved {
        ThreadStatus::Resolved
    } else {
        ThreadStatus::Open
    };

    let anchor = map_thread_anchor(node);

    let comments = node
        .comments
        .nodes
        .iter()
        .filter_map(|c| c.as_ref())
        .map(map_review_comment)
        .collect();

    Thread {
        id: ThreadId::new(node.id.clone()),
        anchor,
        status,
        comments,
    }
}

fn map_thread_anchor(node: &ReviewThreadNode) -> Option<DiffAnchor> {
    let path = node.path.clone()?;
    if node.subject_type.as_deref() == Some("FILE") {
        return Some(DiffAnchor {
            path,
            side: map_diff_side(node.diff_side.as_deref()),
            start_line: node.start_line.unwrap_or(0).max(0) as u32,
            end_line: node.line.unwrap_or(0).max(0) as u32,
        });
    }
    let line = node.line.unwrap_or(0).max(0) as u32;
    let start = node.start_line.unwrap_or(line as i64).max(0) as u32;
    Some(DiffAnchor {
        path,
        side: map_diff_side(node.diff_side.as_deref()),
        start_line: start.min(line),
        end_line: line.max(start),
    })
}

fn map_review_comment(node: &ReviewCommentNode) -> ReviewComment {
    let is_pending = node.state.as_deref() == Some("PENDING");
    let author_login = node
        .author
        .as_ref()
        .and_then(|a| a.login.clone())
        .unwrap_or_else(|| "ghost".into());
    let suggestion = suggestion_from_body(&node.body);
    let id = node
        .database_id
        .map(|n| n.to_string())
        .unwrap_or_else(|| node.id.clone());
    ReviewComment {
        id: ReviewCommentId::new(id),
        author_login,
        body: node.body.clone(),
        is_pending,
        suggestion,
    }
}

fn map_diff_side(raw: Option<&str>) -> DiffLineSide {
    match raw {
        Some("LEFT") => DiffLineSide::Left,
        _ => DiffLineSide::Right,
    }
}

pub fn review_submit_event_to_rest(event: ReviewSubmitEvent) -> &'static str {
    match event {
        ReviewSubmitEvent::Approve => "APPROVE",
        ReviewSubmitEvent::RequestChanges => "REQUEST_CHANGES",
        ReviewSubmitEvent::Comment => "COMMENT",
    }
}

pub fn diff_side_to_rest(side: DiffLineSide) -> &'static str {
    match side {
        DiffLineSide::Left => "LEFT",
        DiffLineSide::Right => "RIGHT",
    }
}

#[derive(Debug, Deserialize)]
struct RestReviewList {
    #[serde(default)]
    id: u64,
    state: String,
    user: Option<RestUser>,
}

#[derive(Debug, Deserialize)]
struct RestUser {
    login: String,
}

#[derive(Debug, Deserialize)]
struct RestReviewCreate {
    id: u64,
}

#[derive(Debug, Deserialize)]
struct RestReviewComment {
    id: u64,
}

pub fn find_viewer_pending_review_id(
    body: &str,
    viewer_login: &str,
) -> Result<Option<u64>, ProviderError> {
    let reviews: Vec<RestReviewList> = serde_json::from_str(body)
        .map_err(|e| ProviderError::other(format!("reviews list decode: {e}")))?;
    Ok(reviews.into_iter().find_map(|review| {
        if review.state != "PENDING" {
            return None;
        }
        let login = review.user.as_ref()?.login.as_str();
        if login != viewer_login {
            return None;
        }
        Some(review.id)
    }))
}

pub fn parse_review_create_id(body: &str) -> Result<u64, ProviderError> {
    let created: RestReviewCreate = serde_json::from_str(body)
        .map_err(|e| ProviderError::other(format!("review create decode: {e}")))?;
    Ok(created.id)
}

pub fn parse_review_comment_id(body: &str) -> Result<u64, ProviderError> {
    let created: RestReviewComment = serde_json::from_str(body)
        .map_err(|e| ProviderError::other(format!("review comment decode: {e}")))?;
    Ok(created.id)
}

#[cfg(test)]
mod tests {
    use super::*;
    use reviewdesk_core::PullRequestId;

    fn pr_id() -> PullRequestId {
        PullRequestId::new("acme/widgets#42").expect("id")
    }

    #[test]
    fn maps_review_thread_with_pending_comment_and_suggestion() {
        let json = include_str!("../testdata/review_threads_page.json");
        let data: ReviewThreadsData = serde_json::from_str(json).expect("decode");
        let pull_request = data.repository.unwrap().pull_request.unwrap();
        let snapshot = map_pull_request_threads(&pr_id(), &pull_request);

        assert_eq!(snapshot.threads.len(), 1);
        let thread = &snapshot.threads[0];
        assert_eq!(thread.status, ThreadStatus::Open);
        assert_eq!(thread.comments.len(), 2);
        assert!(thread.comments[1].is_pending);
        assert_eq!(thread.comments[0].suggestion.as_deref(), Some("let x = 1;"));
        assert!(snapshot.pending_review.is_some());
        assert_eq!(
            snapshot.pending_review.as_ref().unwrap().provider_review_id,
            Some("991".into())
        );
    }

    #[test]
    fn find_viewer_pending_review_id_matches_login() {
        let body = r#"[
          {"id": 1, "state": "APPROVED", "user": {"login": "alice"}},
          {"id": 991, "state": "PENDING", "user": {"login": "alice"}}
        ]"#;
        assert_eq!(
            find_viewer_pending_review_id(body, "alice").unwrap(),
            Some(991)
        );
        assert_eq!(find_viewer_pending_review_id(body, "bob").unwrap(), None);
    }
}
