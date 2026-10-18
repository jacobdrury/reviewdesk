use std::path::Path;

use async_trait::async_trait;
use reviewdesk_core::{
    LocalStore, LocalStoreError, Patch, PullRequestDetail, PullRequestId, PullRequestListItem,
    PullRequestListQuery,
};
use turso::{Builder, Connection, Database};

use crate::detail_patch;
use crate::error::map_open;
use crate::migrate;

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
}
