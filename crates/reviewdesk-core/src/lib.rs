//! Domain types and Provider trait for Review Desk.
//!
//! This crate has no HTTP client and no UI dependencies.

mod credentials;
mod error;
mod id;
mod local_store;
mod patch;
mod provider;
mod pull_request;
mod pull_request_list;
mod views;

pub use credentials::Credentials;
pub use error::ProviderError;
pub use id::PullRequestId;
pub use local_store::{LocalStore, LocalStoreError};
pub use patch::Patch;
pub use provider::Provider;
pub use pull_request::{PullRequestDetail, Reviewer};
pub use pull_request_list::{
    DraftFilter, PullRequestLabel, PullRequestListFilter, PullRequestListItem,
    PullRequestListOptions, PullRequestListQuery, PullRequestListSort, PullRequestListSortField,
    ReviewState, SortDirection,
};
pub use views::{
    CiStatusFilter, UpdatedWithin, View, ViewFilter, ViewGroupBy, ViewId, ViewKind,
    VIEW_ID_ALL_PRS, VIEW_ID_BY_PROJECT, VIEW_ID_DRAFTS, VIEW_ID_MINE, VIEW_ID_NEEDS_MY_REVIEW,
};
