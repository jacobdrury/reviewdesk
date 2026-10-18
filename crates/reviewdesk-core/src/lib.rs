//! Domain types and Provider trait for Review Desk.
//!
//! This crate has no HTTP client and no UI dependencies.

mod codeowners;
mod credentials;
mod error;
mod id;
mod local_store;
mod metadata;
mod patch;
mod provider;
mod pull_request;
mod pull_request_list;

pub use codeowners::{codeowners_breakdown, parse_codeowners};
pub use credentials::Credentials;
pub use error::ProviderError;
pub use id::PullRequestId;
pub use local_store::{LocalStore, LocalStoreError};
pub use metadata::{
    Assignee, CodeownersRule, MergeableState, MetadataWriteCapabilities, ReviewDecision,
};
pub use patch::Patch;
pub use provider::Provider;
pub use pull_request::{empty_metadata_fields, PullRequestDetail, Reviewer};
pub use pull_request_list::{
    DraftFilter, PullRequestLabel, PullRequestListFilter, PullRequestListItem,
    PullRequestListOptions, PullRequestListQuery, PullRequestListSort, PullRequestListSortField,
    ReviewState, SortDirection,
};
