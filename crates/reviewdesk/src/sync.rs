//! Background Provider → LocalStore sync with serialized writes.

use std::sync::Arc;

use reviewdesk_core::{
    LocalStore, LocalStoreError, Patch, Provider, ProviderError, PullRequestDetail, PullRequestId,
    PullRequestListItem, PullRequestListOptions, PullRequestListQuery, PullRequestThreads,
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

    pub async fn get_pull_request_threads(
        &self,
        id: &PullRequestId,
    ) -> Result<Option<PullRequestThreads>, LocalStoreError> {
        let _guard = self.store_lock.lock().await;
        self.inner.get_pull_request_threads(id).await
    }

    pub async fn put_pull_request_threads(
        &self,
        id: &PullRequestId,
        threads: &PullRequestThreads,
    ) -> Result<(), LocalStoreError> {
        let _guard = self.store_lock.lock().await;
        self.inner.put_pull_request_threads(id, threads).await
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

        async fn put_pull_request_threads(
            &self,
            _id: &PullRequestId,
            _threads: &PullRequestThreads,
        ) -> Result<(), LocalStoreError> {
            Ok(())
        }

        async fn get_pull_request_threads(
            &self,
            _id: &PullRequestId,
        ) -> Result<Option<PullRequestThreads>, LocalStoreError> {
            Ok(None)
        }
    }

    struct StubProvider {
        pull_requests: Vec<PullRequestListItem>,
    }

    #[async_trait]
    impl Provider for StubProvider {
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

        async fn fetch_pull_request_threads(
            &self,
            _id: &PullRequestId,
        ) -> Result<PullRequestThreads, ProviderError> {
            Err(ProviderError::other("not used"))
        }

        async fn add_pending_diff_review_comment(
            &self,
            _id: &PullRequestId,
            _comment: reviewdesk_core::PendingDiffReviewComment,
        ) -> Result<PullRequestThreads, ProviderError> {
            Err(ProviderError::other("not used"))
        }

        async fn submit_pending_review(
            &self,
            _id: &PullRequestId,
            _event: reviewdesk_core::ReviewSubmitEvent,
            _summary_body: Option<String>,
        ) -> Result<PullRequestThreads, ProviderError> {
            Err(ProviderError::other("not used"))
        }

        async fn discard_pending_review(
            &self,
            _id: &PullRequestId,
        ) -> Result<PullRequestThreads, ProviderError> {
            Err(ProviderError::other("not used"))
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
