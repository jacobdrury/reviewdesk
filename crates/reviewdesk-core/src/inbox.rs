use chrono::{DateTime, Utc};

use crate::id::PullRequestId;

/// Options for listing the Inbox. Defaults match the first-slice hardcoded UI.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InboxOptions {
    pub include_authored: bool,
    pub drafts: DraftFilter,
}

impl Default for InboxOptions {
    fn default() -> Self {
        Self {
            include_authored: false,
            drafts: DraftFilter::ReadyOnly,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DraftFilter {
    ReadyOnly,
    DraftsOnly,
    All,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InboxLabel {
    pub name: String,
    pub color: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReviewState {
    Pending,
    Approved,
    ChangesRequested,
    Commented,
    Dismissed,
    Unknown,
}

/// One row in the Inbox list.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InboxItem {
    pub id: PullRequestId,
    pub title: String,
    pub web_url: String,
    pub updated_at: DateTime<Utc>,
    pub author_login: String,
    pub author_avatar_url: Option<String>,
    pub repo_label: String,
    pub labels: Vec<InboxLabel>,
    pub review_state: ReviewState,
    pub is_draft: bool,
}
