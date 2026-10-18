//! Background Provider → LocalStore sync with serialized writes.

use std::sync::Arc;

use reviewdesk_core::{
    LocalStore, LocalStoreError, Patch, Provider, ProviderError, PullRequestDetail, PullRequestId,
    PullRequestListItem, PullRequestListOptions, PullRequestListQuery, View, ViewId,
};
use tokio::sync::Mutex;

/// Wraps a [`LocalStore`] so all access is serialized (Turso same-connection rule).
pub struct AppLocalStore {
    inner: Arc<dyn LocalStore>,
    store_lock: Mutex<()>,
}

impl AppLocalStore {
    pub fn new(inner: Arc<dyn LocalStore>) -> Self {
        Self {
            inner,
            store_lock: Mutex::new(()),
        }
    }

    pub async fn query_pull_requests(
        &self,
        query: &PullRequestListQuery,
    ) -> Result<Vec<PullRequestListItem>, LocalStoreError> {
        let _guard = self.store_lock.lock().await;
        self.inner.query_pull_requests(query).await
    }

    pub async fn get_pull_request_detail(
        &self,
        id: &PullRequestId,
    ) -> Result<Option<PullRequestDetail>, LocalStoreError> {
        let _guard = self.store_lock.lock().await;
        self.inner.get_pull_request_detail(id).await
    }

    pub async fn get_patch(&self, id: &PullRequestId) -> Result<Option<Patch>, LocalStoreError> {
        let _guard = self.store_lock.lock().await;
        self.inner.get_patch(id).await
    }

    pub async fn list_views(&self) -> Result<Vec<View>, LocalStoreError> {
        let _guard = self.store_lock.lock().await;
        self.inner.list_views().await
    }

    pub async fn save_view(&self, view: &View) -> Result<(), LocalStoreError> {
        let _guard = self.store_lock.lock().await;
        self.inner.save_view(view).await
    }

    pub async fn delete_view(&self, id: &ViewId) -> Result<(), LocalStoreError> {
        let _guard = self.store_lock.lock().await;
        self.inner.delete_view(id).await
    }

    pub async fn reorder_views(&self, order: &[ViewId]) -> Result<(), LocalStoreError> {
        let _guard = self.store_lock.lock().await;
        self.inner.reorder_views(order).await
    }

    pub async fn get_last_used_view(&self) -> Result<Option<ViewId>, LocalStoreError> {
        let _guard = self.store_lock.lock().await;
        self.inner.get_last_used_view().await
    }

    pub async fn set_last_used_view(&self, id: &ViewId) -> Result<(), LocalStoreError> {
        let _guard = self.store_lock.lock().await;
        self.inner.set_last_used_view(id).await
    }

    pub async fn get_scoped_repository(&self) -> Result<Option<String>, LocalStoreError> {
        let _guard = self.store_lock.lock().await;
        self.inner.get_scoped_repository().await
    }

    pub async fn set_scoped_repository(&self, repo: Option<&str>) -> Result<(), LocalStoreError> {
        let _guard = self.store_lock.lock().await;
        self.inner.set_scoped_repository(repo).await
    }

    pub async fn persist_views_preferences(
        &self,
        views: &[View],
        last_used: Option<&ViewId>,
        scoped_repository: Option<&str>,
    ) -> Result<Vec<View>, LocalStoreError> {
        let existing = self.list_views().await?;
        for old in existing {
            if !views.iter().any(|v| v.id == old.id) {
                self.delete_view(&old.id).await?;
            }
        }
        for view in views {
            self.save_view(view).await?;
        }
        let order: Vec<ViewId> = views.iter().map(|v| v.id.clone()).collect();
        self.reorder_views(&order).await?;
        if let Some(id) = last_used {
            self.set_last_used_view(id).await?;
        }
        self.set_scoped_repository(scoped_repository).await?;
        self.list_views().await
    }

    async fn sync_pull_requests_locked(
        &self,
        items: &[PullRequestListItem],
    ) -> Result<(), LocalStoreError> {
        let _guard = self.store_lock.lock().await;
        self.inner.sync_pull_requests(items).await
    }

    async fn put_pull_request_detail_locked(
        &self,
        detail: &PullRequestDetail,
    ) -> Result<(), LocalStoreError> {
        let _guard = self.store_lock.lock().await;
        self.inner.put_pull_request_detail(detail).await
    }

    async fn put_patch_locked(
        &self,
        id: &PullRequestId,
        patch: &Patch,
    ) -> Result<(), LocalStoreError> {
        let _guard = self.store_lock.lock().await;
        self.inner.put_patch(id, patch).await
    }
}

/// Fetches the Pull requests list from the Provider and reconciles into LocalStore.
pub async fn sync_pull_requests_from_provider(
    provider: &dyn Provider,
    store: &AppLocalStore,
    options: &PullRequestListOptions,
) -> Result<(), ProviderError> {
    let items = provider.list_pull_requests(options).await?;
    store
        .sync_pull_requests_locked(&items)
        .await
        .map_err(|e| ProviderError::other(e.to_string()))?;
    Ok(())
}

/// Loads detail from cache, or fetches from Provider, stores, and returns it.
pub async fn load_detail_via_store(
    provider: &dyn Provider,
    store: &AppLocalStore,
    id: &PullRequestId,
) -> Result<PullRequestDetail, ProviderError> {
    if let Some(detail) = store
        .get_pull_request_detail(id)
        .await
        .map_err(|e| ProviderError::other(e.to_string()))?
    {
        return Ok(detail);
    }

    let detail = provider.fetch_pull_request(id).await?;
    store
        .put_pull_request_detail_locked(&detail)
        .await
        .map_err(|e| ProviderError::other(e.to_string()))?;
    Ok(detail)
}

/// Loads patch from cache, or fetches from Provider, stores, and returns it.
pub async fn load_patch_via_store(
    provider: &dyn Provider,
    store: &AppLocalStore,
    id: &PullRequestId,
) -> Result<Patch, ProviderError> {
    if let Some(patch) = store
        .get_patch(id)
        .await
        .map_err(|e| ProviderError::other(e.to_string()))?
    {
        return Ok(patch);
    }

    let patch = provider.fetch_patch(id).await?;
    store
        .put_patch_locked(id, &patch)
        .await
        .map_err(|e| ProviderError::other(e.to_string()))?;
    Ok(patch)
}

/// Fetches detail from the Provider, updates LocalStore, and returns it.
pub async fn refresh_pull_request_detail(
    provider: &dyn Provider,
    store: &AppLocalStore,
    id: &PullRequestId,
) -> Result<PullRequestDetail, ProviderError> {
    let detail = provider.fetch_pull_request(id).await?;
    store
        .put_pull_request_detail_locked(&detail)
        .await
        .map_err(|e| ProviderError::other(e.to_string()))?;
    Ok(detail)
}

/// Fetches patch from the Provider, updates LocalStore, and returns it.
pub async fn refresh_patch(
    provider: &dyn Provider,
    store: &AppLocalStore,
    id: &PullRequestId,
) -> Result<Patch, ProviderError> {
    let patch = provider.fetch_patch(id).await?;
    store
        .put_patch_locked(id, &patch)
        .await
        .map_err(|e| ProviderError::other(e.to_string()))?;
    Ok(patch)
}

#[cfg(test)]
mod tests {
    use std::sync::Mutex as StdMutex;

    use async_trait::async_trait;
    use chrono::Utc;
    use reviewdesk_core::{PullRequestLabel, ReviewState};

    use super::*;

    struct RecordingStore {
        pull_requests: StdMutex<Vec<PullRequestListItem>>,
    }

    #[async_trait]
    impl LocalStore for RecordingStore {
        async fn schema_version(&self) -> Result<u32, LocalStoreError> {
            Ok(1)
        }

        async fn sync_pull_requests(
            &self,
            items: &[PullRequestListItem],
        ) -> Result<(), LocalStoreError> {
            *self.pull_requests.lock().expect("lock") = items.to_vec();
            Ok(())
        }

        async fn query_pull_requests(
            &self,
            _query: &PullRequestListQuery,
        ) -> Result<Vec<PullRequestListItem>, LocalStoreError> {
            Ok(self.pull_requests.lock().expect("lock").clone())
        }

        async fn put_pull_request_detail(
            &self,
            _detail: &PullRequestDetail,
        ) -> Result<(), LocalStoreError> {
            Ok(())
        }

        async fn get_pull_request_detail(
            &self,
            _id: &PullRequestId,
        ) -> Result<Option<PullRequestDetail>, LocalStoreError> {
            Ok(None)
        }

        async fn put_patch(
            &self,
            _id: &PullRequestId,
            _patch: &Patch,
        ) -> Result<(), LocalStoreError> {
            Ok(())
        }

        async fn get_patch(&self, _id: &PullRequestId) -> Result<Option<Patch>, LocalStoreError> {
            Ok(None)
        }

        async fn list_views(&self) -> Result<Vec<View>, LocalStoreError> {
            Ok(Vec::new())
        }

        async fn save_view(&self, _view: &View) -> Result<(), LocalStoreError> {
            Ok(())
        }

        async fn delete_view(&self, _id: &ViewId) -> Result<(), LocalStoreError> {
            Ok(())
        }

        async fn reorder_views(&self, _order: &[ViewId]) -> Result<(), LocalStoreError> {
            Ok(())
        }

        async fn get_last_used_view(&self) -> Result<Option<ViewId>, LocalStoreError> {
            Ok(None)
        }

        async fn set_last_used_view(&self, _id: &ViewId) -> Result<(), LocalStoreError> {
            Ok(())
        }

        async fn get_scoped_repository(&self) -> Result<Option<String>, LocalStoreError> {
            Ok(None)
        }

        async fn set_scoped_repository(&self, _repo: Option<&str>) -> Result<(), LocalStoreError> {
            Ok(())
        }

        async fn sync_notifications(
            &self,
            _threads: &[reviewdesk_core::NotificationThread],
        ) -> Result<(), LocalStoreError> {
            Ok(())
        }

        async fn query_notifications(
            &self,
            _query: &reviewdesk_core::NotificationQuery,
        ) -> Result<Vec<reviewdesk_core::NotificationThread>, LocalStoreError> {
            Ok(Vec::new())
        }

        async fn set_notification_read(
            &self,
            _id: &reviewdesk_core::NotificationThreadId,
        ) -> Result<(), LocalStoreError> {
            Ok(())
        }

        async fn remove_notification(
            &self,
            _id: &reviewdesk_core::NotificationThreadId,
        ) -> Result<(), LocalStoreError> {
            Ok(())
        }
    }

    struct StubProvider {
        pull_requests: Vec<PullRequestListItem>,
    }

    #[async_trait]
    impl Provider for StubProvider {
        async fn current_user(&self) -> Result<String, ProviderError> {
            Ok("me".into())
        }

        async fn list_pull_requests(
            &self,
            _options: &PullRequestListOptions,
        ) -> Result<Vec<PullRequestListItem>, ProviderError> {
            Ok(self.pull_requests.clone())
        }

        async fn fetch_patch(&self, _id: &PullRequestId) -> Result<Patch, ProviderError> {
            Err(ProviderError::other("not used"))
        }

        async fn fetch_pull_request(
            &self,
            _id: &PullRequestId,
        ) -> Result<PullRequestDetail, ProviderError> {
            Err(ProviderError::other("not used"))
        }

        async fn list_notifications(
            &self,
            _options: &reviewdesk_core::NotificationListOptions,
        ) -> Result<reviewdesk_core::NotificationPage, ProviderError> {
            Ok(reviewdesk_core::NotificationPage {
                threads: Vec::new(),
                poll_interval: None,
                last_modified: None,
                not_modified: false,
            })
        }

        async fn mark_notification_read(
            &self,
            _id: &reviewdesk_core::NotificationThreadId,
        ) -> Result<(), ProviderError> {
            Ok(())
        }

        async fn mark_notification_done(
            &self,
            _id: &reviewdesk_core::NotificationThreadId,
        ) -> Result<(), ProviderError> {
            Ok(())
        }
    }

    fn sample_item(id: &str) -> PullRequestListItem {
        PullRequestListItem {
            id: PullRequestId::new(id).expect("id"),
            title: "Title".to_string(),
            web_url: "https://example.com".to_string(),
            updated_at: Utc::now(),
            author_login: "alice".to_string(),
            author_avatar_url: None,
            repo_label: "acme/widgets".to_string(),
            labels: vec![PullRequestLabel {
                name: "bug".to_string(),
                color: "ff0000".to_string(),
            }],
            review_state: ReviewState::Pending,
            is_draft: false,
        }
    }

    #[tokio::test]
    async fn sync_pull_requests_from_provider_writes_into_store() {
        let recording = Arc::new(RecordingStore {
            pull_requests: StdMutex::new(Vec::new()),
        });
        let store = AppLocalStore::new(recording.clone());
        let item = sample_item("acme/widgets#1");
        let provider = StubProvider {
            pull_requests: vec![item.clone()],
        };

        sync_pull_requests_from_provider(&provider, &store, &PullRequestListOptions::default())
            .await
            .expect("sync");

        let rows = store
            .query_pull_requests(&PullRequestListQuery::default())
            .await
            .expect("query");
        assert_eq!(rows, vec![item]);
    }
}
