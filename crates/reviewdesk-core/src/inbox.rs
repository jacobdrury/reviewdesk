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

/// Sort direction for Inbox queries and future saved views.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SortDirection {
    Asc,
    Desc,
}

/// Fields supported for Inbox ordering in v1.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InboxSortField {
    UpdatedAt,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InboxSort {
    pub field: InboxSortField,
    pub direction: SortDirection,
}

impl Default for InboxSort {
    fn default() -> Self {
        Self {
            field: InboxSortField::UpdatedAt,
            direction: SortDirection::Desc,
        }
    }
}

/// Filter criteria for Inbox queries. Extends [`InboxOptions`] with optional view filters.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct InboxFilter {
    pub options: InboxOptions,
    pub repo_label: Option<String>,
    pub author_login: Option<String>,
    pub review_state: Option<ReviewState>,
    pub has_label: Option<String>,
}

/// Query shape for listing Inbox rows from LocalStore (baseline Inbox and future custom views).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct InboxQuery {
    pub filter: InboxFilter,
    pub sort: InboxSort,
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
