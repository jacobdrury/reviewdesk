use std::path::Path;

use async_trait::async_trait;
use reviewdesk_core::{
    LocalStore, LocalStoreError, NotificationQuery, NotificationThread, NotificationThreadId,
    Patch, PullRequestDetail, PullRequestId, PullRequestListItem, PullRequestListQuery,
    PullRequestThreads, View, ViewId,
};
use turso::{Builder, Connection, Database};

use crate::detail_patch;
use crate::error::map_open;
use crate::migrate;
use crate::threads;
use crate::views;

/// Turso-backed local store. Open with [`TursoLocalStore::open`]; migrations run on open.
pub struct TursoLocalStore {
    #[allow(dead_code)]
    database: Database,
    conn: Connection,
}

impl TursoLocalStore {
    /// Opens (or creates) a database at `path`, enables WAL when possible, and runs migrations.
    pub async fn open(path: impl AsRef<Path>) -> Result<Self, LocalStoreError> {
        let path = path.as_ref();
        let path_str = path
            .to_str()
            .ok_or_else(|| LocalStoreError::open(path.display().to_string(), "invalid path"))?;

        let database = Builder::new_local(path_str)
            .build()
            .await
            .map_err(|e| map_open(path, e))?;

        let conn = database.connect().map_err(|e| map_open(path, e))?;

        migrate::prefer_wal(&conn).await?;
        migrate::run(&conn).await?;

        Ok(Self { database, conn })
    }
}

#[async_trait]
impl LocalStore for TursoLocalStore {
    async fn schema_version(&self) -> Result<u32, LocalStoreError> {
        migrate::applied_version(&self.conn).await
    }

    async fn sync_pull_requests(
        &self,
        items: &[PullRequestListItem],
    ) -> Result<(), LocalStoreError> {
        crate::pull_request_list::sync_pull_requests(&self.conn, items).await
    }

    async fn query_pull_requests(
        &self,
        query: &PullRequestListQuery,
    ) -> Result<Vec<PullRequestListItem>, LocalStoreError> {
        crate::pull_request_list::query_pull_requests(&self.conn, query).await
    }

    async fn put_pull_request_detail(
        &self,
        detail: &PullRequestDetail,
    ) -> Result<(), LocalStoreError> {
        detail_patch::put_pull_request_detail(&self.conn, detail).await
    }

    async fn get_pull_request_detail(
        &self,
        id: &PullRequestId,
    ) -> Result<Option<PullRequestDetail>, LocalStoreError> {
        detail_patch::get_pull_request_detail(&self.conn, id).await
    }

    async fn put_patch(&self, id: &PullRequestId, patch: &Patch) -> Result<(), LocalStoreError> {
        detail_patch::put_patch(&self.conn, id, patch).await
    }

    async fn get_patch(&self, id: &PullRequestId) -> Result<Option<Patch>, LocalStoreError> {
        detail_patch::get_patch(&self.conn, id).await
    }

    async fn list_views(&self) -> Result<Vec<View>, LocalStoreError> {
        views::list_views(&self.conn).await
    }

    async fn save_view(&self, view: &View) -> Result<(), LocalStoreError> {
        views::save_view(&self.conn, view).await
    }

    async fn delete_view(&self, id: &ViewId) -> Result<(), LocalStoreError> {
        views::delete_view(&self.conn, id).await
    }

    async fn reorder_views(&self, order: &[ViewId]) -> Result<(), LocalStoreError> {
        views::reorder_views(&self.conn, order).await
    }

    async fn get_last_used_view(&self) -> Result<Option<ViewId>, LocalStoreError> {
        views::get_last_used_view(&self.conn).await
    }

    async fn set_last_used_view(&self, id: &ViewId) -> Result<(), LocalStoreError> {
        views::set_last_used_view(&self.conn, id).await
    }

    async fn get_scoped_repository(&self) -> Result<Option<String>, LocalStoreError> {
        views::get_scoped_repository(&self.conn).await
    }

    async fn set_scoped_repository(&self, repo: Option<&str>) -> Result<(), LocalStoreError> {
        views::set_scoped_repository(&self.conn, repo).await
    }

    async fn sync_notifications(
        &self,
        threads: &[NotificationThread],
    ) -> Result<(), LocalStoreError> {
        crate::notification::sync_notifications(&self.conn, threads).await
    }

    async fn query_notifications(
        &self,
        query: &NotificationQuery,
    ) -> Result<Vec<NotificationThread>, LocalStoreError> {
        crate::notification::query_notifications(&self.conn, query).await
    }

    async fn set_notification_read(
        &self,
        id: &NotificationThreadId,
    ) -> Result<(), LocalStoreError> {
        crate::notification::set_notification_read(&self.conn, id).await
    }

    async fn remove_notification(&self, id: &NotificationThreadId) -> Result<(), LocalStoreError> {
        crate::notification::remove_notification(&self.conn, id).await
    }

    async fn put_pull_request_threads(
        &self,
        id: &PullRequestId,
        threads: &PullRequestThreads,
    ) -> Result<(), LocalStoreError> {
        threads::put_pull_request_threads(&self.conn, id, threads).await
    }

    async fn get_pull_request_threads(
        &self,
        id: &PullRequestId,
    ) -> Result<Option<PullRequestThreads>, LocalStoreError> {
        threads::get_pull_request_threads(&self.conn, id).await
    }
}
