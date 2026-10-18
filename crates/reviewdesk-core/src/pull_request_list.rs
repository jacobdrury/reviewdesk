use chrono::{DateTime, Utc};

use crate::id::PullRequestId;

/// Options for listing Pull requests. Defaults match the first-slice hardcoded UI.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PullRequestListOptions {
    pub include_authored: bool,
    pub drafts: DraftFilter,
}

impl Default for PullRequestListOptions {
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
pub struct PullRequestLabel {
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

/// Sort direction for Pull requests queries and future saved views.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SortDirection {
    Asc,
    Desc,
}

/// Fields supported for Pull requests ordering in v1.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PullRequestListSortField {
    UpdatedAt,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PullRequestListSort {
    pub field: PullRequestListSortField,
    pub direction: SortDirection,
}

impl Default for PullRequestListSort {
    fn default() -> Self {
        Self {
            field: PullRequestListSortField::UpdatedAt,
            direction: SortDirection::Desc,
        }
    }
}

/// Filter criteria for Pull requests queries. Extends [`PullRequestListOptions`] with optional view filters.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct PullRequestListFilter {
    pub options: PullRequestListOptions,
    pub repo_label: Option<String>,
    pub author_login: Option<String>,
    pub review_state: Option<ReviewState>,
    pub has_label: Option<String>,
}

/// Query shape for listing Pull requests rows from LocalStore (baseline list and future custom views).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct PullRequestListQuery {
    pub filter: PullRequestListFilter,
    pub sort: PullRequestListSort,
}

/// One row in the Pull requests list.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PullRequestListItem {
    pub id: PullRequestId,
    pub title: String,
    pub web_url: String,
    pub updated_at: DateTime<Utc>,
    pub author_login: String,
    pub author_avatar_url: Option<String>,
    pub repo_label: String,
    pub labels: Vec<PullRequestLabel>,
    pub review_state: ReviewState,
    pub is_draft: bool,
}
