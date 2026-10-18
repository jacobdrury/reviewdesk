use async_trait::async_trait;
use thiserror::Error;

use crate::id::PullRequestId;
use crate::inbox::{InboxItem, InboxQuery};
use crate::patch::Patch;
use crate::pull_request::PullRequestDetail;

/// Errors from the local persistence layer. Distinct from [`ProviderError`](crate::ProviderError).
#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum LocalStoreError {
    #[error("failed to open local store at {path}: {message}")]
    Open { path: String, message: String },
    #[error("migration failed: {message}")]
    Migration { message: String },
    #[error("{message}")]
    Other { message: String },
}

impl LocalStoreError {
    pub fn open(path: impl Into<String>, message: impl Into<String>) -> Self {
        Self::Open {
            path: path.into(),
            message: message.into(),
        }
    }

    pub fn migration(message: impl Into<String>) -> Self {
        Self::Migration {
            message: message.into(),
        }
    }

    pub fn other(message: impl Into<String>) -> Self {
        Self::Other {
            message: message.into(),
        }
    }
}

/// Local-first cache for Inbox, pull-request metadata, and patches.
///
/// Turso and SQL stay in the implementation crate; UI and sync depend on this trait only.
#[async_trait]
pub trait LocalStore: Send + Sync {
    /// Schema version after the latest successful migration.
    async fn schema_version(&self) -> Result<u32, LocalStoreError>;

    /// Upserts Inbox rows from a Provider sync snapshot and removes rows not in `items`.
    async fn sync_inbox(&self, items: &[InboxItem]) -> Result<(), LocalStoreError>;

    /// Lists Inbox rows matching `query` (filters, sort). Does not load Patch blobs.
    async fn query_inbox(&self, query: &InboxQuery) -> Result<Vec<InboxItem>, LocalStoreError>;

    /// Persists overview detail for a pull request, replacing any prior row for the same id.
    async fn put_pull_request_detail(
        &self,
        detail: &PullRequestDetail,
    ) -> Result<(), LocalStoreError>;

    /// Returns cached overview detail, or `None` on cache miss.
    async fn get_pull_request_detail(
        &self,
        id: &PullRequestId,
    ) -> Result<Option<PullRequestDetail>, LocalStoreError>;

    /// Persists unified-diff text for a pull request, replacing any prior row for the same id.
    async fn put_patch(&self, id: &PullRequestId, patch: &Patch) -> Result<(), LocalStoreError>;

    /// Returns cached patch text, or `None` on cache miss.
    async fn get_patch(&self, id: &PullRequestId) -> Result<Option<Patch>, LocalStoreError>;
}
