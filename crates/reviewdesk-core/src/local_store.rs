use async_trait::async_trait;
use thiserror::Error;

use crate::id::PullRequestId;
use crate::notification::{NotificationQuery, NotificationThread, NotificationThreadId};
use crate::patch::Patch;
use crate::pull_request::PullRequestDetail;
use crate::pull_request_list::{PullRequestListItem, PullRequestListQuery};
use crate::review_threads::PullRequestThreads;
use crate::views::{View, ViewId};

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

/// Local-first cache for the Pull requests list, pull-request metadata, and patches.
///
/// Turso and SQL stay in the implementation crate; UI and sync depend on this trait only.
#[async_trait]
pub trait LocalStore: Send + Sync {
    /// Schema version after the latest successful migration.
    async fn schema_version(&self) -> Result<u32, LocalStoreError>;

    /// Upserts Pull requests rows from a Provider sync snapshot and removes rows not in `items`.
    async fn sync_pull_requests(
        &self,
        items: &[PullRequestListItem],
    ) -> Result<(), LocalStoreError>;

    /// Lists Pull requests rows matching `query` (filters, sort). Does not load Patch blobs.
    async fn query_pull_requests(
        &self,
        query: &PullRequestListQuery,
    ) -> Result<Vec<PullRequestListItem>, LocalStoreError>;

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

    /// Lists locally-saved Views ordered by `position` ascending.
    async fn list_views(&self) -> Result<Vec<View>, LocalStoreError>;

    /// Inserts or replaces a View by id (filters, group-by, sort, name, position, kind).
    async fn save_view(&self, view: &View) -> Result<(), LocalStoreError>;

    /// Deletes a View by id. No-op when the id is absent.
    async fn delete_view(&self, id: &ViewId) -> Result<(), LocalStoreError>;

    /// Rewrites `position` for the given ids in the supplied order (0-based).
    async fn reorder_views(&self, order: &[ViewId]) -> Result<(), LocalStoreError>;

    /// Returns the last-used View id preference, or `None` if unset.
    async fn get_last_used_view(&self) -> Result<Option<ViewId>, LocalStoreError>;

    /// Persists the last-used View id preference.
    async fn set_last_used_view(&self, id: &ViewId) -> Result<(), LocalStoreError>;

    /// Returns the sidebar repo scope for Pull requests, or `None` for all repositories.
    async fn get_scoped_repository(&self) -> Result<Option<String>, LocalStoreError>;

    /// Persists sidebar repo scope (`None` clears scope).
    async fn set_scoped_repository(&self, repo: Option<&str>) -> Result<(), LocalStoreError>;

    /// Upserts Inbox notification threads from a poll snapshot and removes rows not in `threads`.
    async fn sync_notifications(
        &self,
        threads: &[NotificationThread],
    ) -> Result<(), LocalStoreError>;

    /// Lists Inbox notification threads matching `query`, `updated_at` descending.
    async fn query_notifications(
        &self,
        query: &NotificationQuery,
    ) -> Result<Vec<NotificationThread>, LocalStoreError>;

    /// Clears the unread flag on a notification thread after Provider write-through.
    async fn set_notification_read(
        &self,
        id: &NotificationThreadId,
    ) -> Result<(), LocalStoreError>;

    /// Removes a notification thread after a Done Provider write-through.
    async fn remove_notification(&self, id: &NotificationThreadId) -> Result<(), LocalStoreError>;

    /// Persists the shared Thread set and optional Pending review for a pull request.
    async fn put_pull_request_threads(
        &self,
        id: &PullRequestId,
        threads: &PullRequestThreads,
    ) -> Result<(), LocalStoreError>;

    /// Returns cached threads, or `None` on cache miss.
    async fn get_pull_request_threads(
        &self,
        id: &PullRequestId,
    ) -> Result<Option<PullRequestThreads>, LocalStoreError>;
}
