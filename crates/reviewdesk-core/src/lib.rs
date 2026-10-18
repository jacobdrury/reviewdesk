//! Domain types and Provider trait for Review Desk.
//!
//! This crate has no HTTP client and no UI dependencies.

mod credentials;
mod error;
mod id;
mod local_store;
mod patch;
mod pipelines;
mod provider;
mod pull_request;
mod pull_request_list;

pub use credentials::Credentials;
pub use error::ProviderError;
pub use id::PullRequestId;
pub use local_store::{LocalStore, LocalStoreError};
pub use patch::Patch;
pub use pipelines::{
    CheckRollup, CheckRollupState, Job, JobState, Pipeline, PipelineKind, PipelinesSnapshot,
    RerunAction, RerunTarget, aggregate_pipeline_state, compute_check_rollup,
};
pub use provider::Provider;
pub use pull_request::{PullRequestDetail, Reviewer};
pub use pull_request_list::{
    DraftFilter, PullRequestLabel, PullRequestListFilter, PullRequestListItem,
    PullRequestListOptions, PullRequestListQuery, PullRequestListSort, PullRequestListSortField,
    ReviewState, SortDirection,
};
