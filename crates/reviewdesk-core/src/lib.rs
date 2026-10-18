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
