//! Domain types and Provider trait for Review Desk.
//!
//! This crate has no HTTP client and no UI dependencies.

mod credentials;
mod error;
mod id;
mod inbox;
mod local_store;
mod patch;
mod provider;
mod pull_request;

pub use credentials::Credentials;
pub use error::ProviderError;
pub use id::PullRequestId;
pub use inbox::{DraftFilter, InboxItem, InboxLabel, InboxOptions, ReviewState};
pub use local_store::{LocalStore, LocalStoreError};
pub use patch::Patch;
pub use provider::Provider;
pub use pull_request::{PullRequestDetail, Reviewer};
