//! Domain types and Provider trait for Review Desk.
//!
//! This crate has no HTTP client and no UI dependencies.

mod codeowners;
mod credentials;
mod error;
mod id;
mod local_store;
mod metadata;
mod notification;
mod patch;
mod provider;
mod pull_request;
mod pull_request_list;
mod review_threads;
mod view_list;
mod views;

pub use codeowners::{codeowners_breakdown, parse_codeowners};
pub use credentials::Credentials;
pub use error::ProviderError;
pub use id::PullRequestId;
pub use local_store::{LocalStore, LocalStoreError};
pub use metadata::{
    Assignee, CodeownersRule, MergeableState, MetadataWriteCapabilities, ReviewDecision,
};
pub use notification::{
    NotificationListOptions, NotificationPage, NotificationQuery, NotificationReason,
    NotificationThread, NotificationThreadId, NotificationThreadIdError,
};
pub use patch::Patch;
pub use provider::Provider;
pub use pull_request::{empty_metadata_fields, PullRequestDetail, Reviewer};
pub use pull_request_list::{
    DraftFilter, PullRequestLabel, PullRequestListFilter, PullRequestListItem,
    PullRequestListOptions, PullRequestListQuery, PullRequestListSort, PullRequestListSortField,
    ReviewState, SortDirection,
};
pub use review_threads::{
    DiffAnchor, DiffLineSide, PendingDiffReviewComment, PendingReview, PullRequestThreads,
    ReviewComment, ReviewCommentId, ReviewSubmitEvent, Thread, ThreadId, ThreadStatus,
    suggestion_from_body,
};
pub use view_list::{
    PullRequestListRow, PullRequestsEmptyKind, build_pull_request_list_rows, distinct_repo_labels,
    group_by_display_label, resolve_launch_view_id, review_state_label, visible_list_rows,
};
pub use views::{
    CiStatusFilter, UpdatedWithin, VIEW_ID_ALL_PRS, VIEW_ID_BY_PROJECT, VIEW_ID_DRAFTS,
    VIEW_ID_MINE, VIEW_ID_NEEDS_MY_REVIEW, View, ViewFilter, ViewGroupBy, ViewId, ViewKind,
};
