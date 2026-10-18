use crate::id::PullRequestId;
use crate::pull_request_list::PullRequestLabel;

/// Overview detail for a selected pull request.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PullRequestDetail {
    pub id: PullRequestId,
    pub title: String,
    pub body: String,
    pub author_login: String,
    pub author_avatar_url: Option<String>,
    pub is_draft: bool,
    pub labels: Vec<PullRequestLabel>,
    pub web_url: String,
    pub reviewers: Vec<Reviewer>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Reviewer {
    pub login: String,
    pub avatar_url: Option<String>,
}
