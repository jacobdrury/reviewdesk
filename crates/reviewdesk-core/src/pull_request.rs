use crate::id::PullRequestId;
use crate::metadata::{
    Assignee, CodeownersRule, MergeableState, MetadataWriteCapabilities, PullRequestState,
    ReviewDecision,
};
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
    pub assignees: Vec<Assignee>,
    pub head_ref: String,
    pub base_ref: String,
    pub review_decision: ReviewDecision,
    pub mergeable: MergeableState,
    pub state: PullRequestState,
    pub auto_merge_enabled: bool,
    pub codeowners_rules: Vec<CodeownersRule>,
    pub write_capabilities: MetadataWriteCapabilities,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Reviewer {
    pub login: String,
    pub avatar_url: Option<String>,
}

/// Empty metadata-panel fields for tests and partial construction.
pub fn empty_metadata_fields() -> (
    Vec<Assignee>,
    String,
    String,
    ReviewDecision,
    MergeableState,
    PullRequestState,
    bool,
    Vec<CodeownersRule>,
    MetadataWriteCapabilities,
) {
    (
        Vec::new(),
        String::new(),
        String::new(),
        ReviewDecision::Unknown,
        MergeableState::Unknown,
        PullRequestState::Open,
        false,
        Vec::new(),
        MetadataWriteCapabilities::default(),
    )
}
