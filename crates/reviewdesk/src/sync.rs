//! Background Provider → LocalStore sync with serialized writes.

use std::sync::Arc;

use reviewdesk_core::{
    LocalStore, LocalStoreError, NotificationListOptions, NotificationQuery, NotificationThread,
    NotificationThreadId, Patch, PipelinesSnapshot, Provider, ProviderError, PullRequestDetail,
    PullRequestId, PullRequestListItem, PullRequestListOptions, PullRequestListQuery, RerunTarget,
};

use crate::review_session::InboxSyncOutcome;
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

    pub async fn get_pipelines(
        &self,
        id: &PullRequestId,
    ) -> Result<Option<PipelinesSnapshot>, LocalStoreError> {
        let _guard = self.store_lock.lock().await;
        self.inner.get_pipelines(id).await
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

    async fn put_pipelines_locked(
        &self,
        id: &PullRequestId,
        snapshot: &PipelinesSnapshot,
    ) -> Result<(), LocalStoreError> {
        let _guard = self.store_lock.lock().await;
        self.inner.put_pipelines(id, snapshot).await
    }

    pub async fn query_notifications(
        &self,
        query: &NotificationQuery,
    ) -> Result<Vec<NotificationThread>, LocalStoreError> {
        let _guard = self.store_lock.lock().await;
        self.inner.query_notifications(query).await
    }

    async fn sync_notifications_locked(
        &self,
        threads: &[NotificationThread],
    ) -> Result<(), LocalStoreError> {
        let _guard = self.store_lock.lock().await;
        self.inner.sync_notifications(threads).await
    }

    async fn set_notification_read_locked(
        &self,
        id: &NotificationThreadId,
    ) -> Result<(), LocalStoreError> {
        let _guard = self.store_lock.lock().await;
        self.inner.set_notification_read(id).await
    }

    async fn remove_notification_locked(
        &self,
        id: &NotificationThreadId,
    ) -> Result<(), LocalStoreError> {
        let _guard = self.store_lock.lock().await;
        self.inner.remove_notification(id).await
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

pub async fn load_pipelines_via_store(
    provider: &dyn Provider,
    store: &AppLocalStore,
    id: &PullRequestId,
) -> Result<PipelinesSnapshot, ProviderError> {
    if let Some(snapshot) = store
        .get_pipelines(id)
        .await
        .map_err(|e| ProviderError::other(e.to_string()))?
    {
        return Ok(snapshot);
    }

    let snapshot = provider.fetch_pipelines(id).await?;
    store
        .put_pipelines_locked(id, &snapshot)
        .await
        .map_err(|e| ProviderError::other(e.to_string()))?;
    Ok(snapshot)
}

pub async fn refresh_pipelines(
    provider: &dyn Provider,
    store: &AppLocalStore,
    id: &PullRequestId,
) -> Result<PipelinesSnapshot, ProviderError> {
    let snapshot = provider.fetch_pipelines(id).await?;
    store
        .put_pipelines_locked(id, &snapshot)
        .await
        .map_err(|e| ProviderError::other(e.to_string()))?;
    Ok(snapshot)
}

pub async fn rerun_check_via_store(
    provider: &dyn Provider,
    store: &AppLocalStore,
    id: &PullRequestId,
    target: &RerunTarget,
) -> Result<PipelinesSnapshot, ProviderError> {
    provider.rerun_check(id, target).await?;
    refresh_pipelines(provider, store, id).await
}

/// Polls the Inbox from the Provider and reconciles into LocalStore when modified.
pub async fn sync_inbox_from_provider(
    provider: &dyn Provider,
    store: &AppLocalStore,
    options: &NotificationListOptions,
) -> Result<InboxSyncOutcome, ProviderError> {
    let page = provider.list_notifications(options).await?;
    if !page.not_modified {
        store
            .sync_notifications_locked(&page.threads)
            .await
            .map_err(|e| ProviderError::other(e.to_string()))?;
    }
    Ok(InboxSyncOutcome {
        poll_interval: page.poll_interval,
        last_modified: page.last_modified,
        not_modified: page.not_modified,
    })
}

/// Marks a notification thread read on the Provider, then write-through to LocalStore.
pub async fn mark_notification_read_via_provider(
    provider: &dyn Provider,
    store: &AppLocalStore,
    id: &NotificationThreadId,
) -> Result<(), ProviderError> {
    provider.mark_notification_read(id).await?;
    store
        .set_notification_read_locked(id)
        .await
        .map_err(|e| ProviderError::other(e.to_string()))?;
    Ok(())
}

/// Removes a notification thread on the Provider (Done), then write-through to LocalStore.
pub async fn mark_notification_done_via_provider(
    provider: &dyn Provider,
    store: &AppLocalStore,
    id: &NotificationThreadId,
) -> Result<(), ProviderError> {
    provider.mark_notification_done(id).await?;
    store
        .remove_notification_locked(id)
        .await
        .map_err(|e| ProviderError::other(e.to_string()))?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::{Arc, Mutex as StdMutex};

    use async_trait::async_trait;
    use chrono::TimeZone;
    use chrono::Utc;
    use reviewdesk_core::{
        CheckRollup, CheckRollupState, Job, JobState, NotificationListOptions, NotificationPage,
        NotificationReason, NotificationThread, NotificationThreadId, Pipeline, PipelineKind,
        PipelinesSnapshot, PullRequestLabel, RerunAction, RerunTarget, ReviewState,
    };
    use reviewdesk_localstore::TursoLocalStore;
    use tempfile::tempdir;

    use super::*;

    struct RecordingStore {
        pull_requests: StdMutex<Vec<PullRequestListItem>>,
        notifications: StdMutex<Vec<NotificationThread>>,
        read_ids: StdMutex<Vec<String>>,
        removed_ids: StdMutex<Vec<String>>,
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

        async fn sync_notifications(
            &self,
            threads: &[NotificationThread],
        ) -> Result<(), LocalStoreError> {
            *self.notifications.lock().expect("lock") = threads.to_vec();
            Ok(())
        }

        async fn query_notifications(
            &self,
            _query: &NotificationQuery,
        ) -> Result<Vec<NotificationThread>, LocalStoreError> {
            Ok(self.notifications.lock().expect("lock").clone())
        }

        async fn set_notification_read(
            &self,
            id: &NotificationThreadId,
        ) -> Result<(), LocalStoreError> {
            self.read_ids
                .lock()
                .expect("lock")
                .push(id.as_str().to_string());
            Ok(())
        }

        async fn remove_notification(
            &self,
            id: &NotificationThreadId,
        ) -> Result<(), LocalStoreError> {
            self.removed_ids
                .lock()
                .expect("lock")
                .push(id.as_str().to_string());
            Ok(())
        }

        async fn put_pipelines(
            &self,
            _id: &PullRequestId,
            _snapshot: &PipelinesSnapshot,
        ) -> Result<(), LocalStoreError> {
            Ok(())
        }

        async fn get_pipelines(
            &self,
            _id: &PullRequestId,
        ) -> Result<Option<PipelinesSnapshot>, LocalStoreError> {
            Ok(None)
        }
    }

    struct StubProvider {
        pull_requests: Vec<PullRequestListItem>,
        notifications: NotificationPage,
        read_calls: StdMutex<Vec<String>>,
        done_calls: StdMutex<Vec<String>>,
        pipelines: Option<PipelinesSnapshot>,
        rerun_calls: Arc<AtomicUsize>,
        rerun_allowed: bool,
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

        async fn list_notifications(
            &self,
            _options: &NotificationListOptions,
        ) -> Result<NotificationPage, ProviderError> {
            Ok(self.notifications.clone())
        }

        async fn mark_notification_read(
            &self,
            id: &NotificationThreadId,
        ) -> Result<(), ProviderError> {
            self.read_calls
                .lock()
                .expect("lock")
                .push(id.as_str().to_string());
            Ok(())
        }

        async fn mark_notification_done(
            &self,
            id: &NotificationThreadId,
        ) -> Result<(), ProviderError> {
            self.done_calls
                .lock()
                .expect("lock")
                .push(id.as_str().to_string());
            Ok(())
        }

        async fn fetch_pipelines(
            &self,
            _id: &PullRequestId,
        ) -> Result<PipelinesSnapshot, ProviderError> {
            self.pipelines
                .clone()
                .ok_or_else(|| ProviderError::other("no pipelines"))
        }

        async fn rerun_check(
            &self,
            _id: &PullRequestId,
            _target: &RerunTarget,
        ) -> Result<(), ProviderError> {
            self.rerun_calls.fetch_add(1, Ordering::SeqCst);
            if self.rerun_allowed {
                Ok(())
            } else {
                Err(ProviderError::other("rerun not allowed"))
            }
        }
    }

    fn sample_notification(id: &str) -> NotificationThread {
        NotificationThread {
            id: NotificationThreadId::new(id).expect("id"),
            pull_request_id: PullRequestId::new("acme/widgets#1").expect("pr"),
            title: "Title".to_string(),
            repo_label: "acme/widgets".to_string(),
            reason: NotificationReason::ReviewRequested,
            unread: true,
            updated_at: Utc.with_ymd_and_hms(2026, 1, 1, 0, 0, 0).unwrap(),
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
            notifications: StdMutex::new(Vec::new()),
            read_ids: StdMutex::new(Vec::new()),
            removed_ids: StdMutex::new(Vec::new()),
        });
        let store = AppLocalStore::new(recording.clone());
        let item = sample_item("acme/widgets#1");
        let provider = StubProvider {
            pull_requests: vec![item.clone()],
            notifications: NotificationPage {
                threads: Vec::new(),
                poll_interval: None,
                last_modified: None,
                not_modified: false,
            },
            read_calls: StdMutex::new(Vec::new()),
            done_calls: StdMutex::new(Vec::new()),
            pipelines: None,
            rerun_calls: Arc::new(AtomicUsize::new(0)),
            rerun_allowed: false,
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

    #[tokio::test]
    async fn sync_inbox_from_provider_writes_into_store() {
        let recording = Arc::new(RecordingStore {
            pull_requests: StdMutex::new(Vec::new()),
            notifications: StdMutex::new(Vec::new()),
            read_ids: StdMutex::new(Vec::new()),
            removed_ids: StdMutex::new(Vec::new()),
        });
        let store = AppLocalStore::new(recording.clone());
        let thread = sample_notification("thread-1");
        let provider = StubProvider {
            pull_requests: Vec::new(),
            notifications: NotificationPage {
                threads: vec![thread.clone()],
                poll_interval: None,
                last_modified: Some("Wed, 01 Jan 2026 00:00:00 GMT".into()),
                not_modified: false,
            },
            read_calls: StdMutex::new(Vec::new()),
            done_calls: StdMutex::new(Vec::new()),
            pipelines: None,
            rerun_calls: Arc::new(AtomicUsize::new(0)),
            rerun_allowed: false,
        };

        let outcome =
            sync_inbox_from_provider(&provider, &store, &NotificationListOptions::default())
                .await
                .expect("sync");

        assert_eq!(
            outcome.last_modified.as_deref(),
            Some("Wed, 01 Jan 2026 00:00:00 GMT")
        );
        let rows = store
            .query_notifications(&NotificationQuery::default())
            .await
            .expect("query");
        assert_eq!(rows, vec![thread]);
    }

    #[tokio::test]
    async fn mark_notification_read_write_through() {
        let recording = Arc::new(RecordingStore {
            pull_requests: StdMutex::new(Vec::new()),
            notifications: StdMutex::new(Vec::new()),
            read_ids: StdMutex::new(Vec::new()),
            removed_ids: StdMutex::new(Vec::new()),
        });
        let store = AppLocalStore::new(recording.clone());
        let id = NotificationThreadId::new("thread-1").expect("id");
        let provider = StubProvider {
            pull_requests: Vec::new(),
            notifications: NotificationPage {
                threads: Vec::new(),
                poll_interval: None,
                last_modified: None,
                not_modified: false,
            },
            read_calls: StdMutex::new(Vec::new()),
            done_calls: StdMutex::new(Vec::new()),
            pipelines: None,
            rerun_calls: Arc::new(AtomicUsize::new(0)),
            rerun_allowed: false,
        };

        mark_notification_read_via_provider(&provider, &store, &id)
            .await
            .expect("read");

        assert_eq!(
            *provider.read_calls.lock().expect("lock"),
            vec!["thread-1".to_string()]
        );
        assert_eq!(
            *recording.read_ids.lock().expect("lock"),
            vec!["thread-1".to_string()]
        );
    }

    #[tokio::test]
    async fn mark_notification_done_write_through() {
        let recording = Arc::new(RecordingStore {
            pull_requests: StdMutex::new(Vec::new()),
            notifications: StdMutex::new(Vec::new()),
            read_ids: StdMutex::new(Vec::new()),
            removed_ids: StdMutex::new(Vec::new()),
        });
        let store = AppLocalStore::new(recording.clone());
        let id = NotificationThreadId::new("thread-9").expect("id");
        let provider = StubProvider {
            pull_requests: Vec::new(),
            notifications: NotificationPage {
                threads: Vec::new(),
                poll_interval: None,
                last_modified: None,
                not_modified: false,
            },
            read_calls: StdMutex::new(Vec::new()),
            done_calls: StdMutex::new(Vec::new()),
            pipelines: None,
            rerun_calls: Arc::new(AtomicUsize::new(0)),
            rerun_allowed: false,
        };

        mark_notification_done_via_provider(&provider, &store, &id)
            .await
            .expect("done");

        assert_eq!(
            *provider.done_calls.lock().expect("lock"),
            vec!["thread-9".to_string()]
        );
        assert_eq!(
            *recording.removed_ids.lock().expect("lock"),
            vec!["thread-9".to_string()]
        );
    }

    fn sample_pipelines(state: CheckRollupState) -> PipelinesSnapshot {
        PipelinesSnapshot {
            rollup: CheckRollup {
                state,
                label: "stub".into(),
            },
            pipelines: vec![Pipeline {
                id: "p1".into(),
                name: "CI".into(),
                run_label: "#1".into(),
                kind: PipelineKind::WorkflowRun,
                created_at: Utc::now(),
                jobs: vec![Job {
                    id: "j1".into(),
                    name: "test".into(),
                    state: JobState::Success,
                    required: true,
                    rerun: RerunAction::Allowed(RerunTarget::WorkflowRun { run_id: 99 }),
                }],
                rerun: RerunAction::Allowed(RerunTarget::WorkflowRun { run_id: 99 }),
            }],
        }
    }

    async fn temp_store() -> AppLocalStore {
        let dir = tempdir().expect("tempdir");
        let path = dir.path().join("reviewdesk.db");
        let inner = Arc::new(TursoLocalStore::open(&path).await.expect("open"));
        AppLocalStore::new(inner)
    }

    #[tokio::test]
    async fn load_pipelines_paints_localstore_first_then_refreshes() {
        let store = temp_store().await;
        let id = PullRequestId::new("acme/widgets#1").expect("id");
        let cached = sample_pipelines(CheckRollupState::Pending);
        store
            .put_pipelines_locked(&id, &cached)
            .await
            .expect("seed cache");

        let provider = StubProvider {
            pull_requests: vec![],
            notifications: NotificationPage {
                threads: Vec::new(),
                poll_interval: None,
                last_modified: None,
                not_modified: false,
            },
            read_calls: StdMutex::new(Vec::new()),
            done_calls: StdMutex::new(Vec::new()),
            pipelines: Some(sample_pipelines(CheckRollupState::Success)),
            rerun_calls: Arc::new(AtomicUsize::new(0)),
            rerun_allowed: true,
        };

        let loaded = load_pipelines_via_store(&provider, &store, &id)
            .await
            .expect("load");
        assert_eq!(loaded.rollup.state, CheckRollupState::Pending);

        let refreshed = refresh_pipelines(&provider, &store, &id)
            .await
            .expect("refresh");
        assert_eq!(refreshed.rollup.state, CheckRollupState::Success);
    }

    #[tokio::test]
    async fn rerun_check_write_through_refetches_after_provider_success() {
        let store = temp_store().await;
        let id = PullRequestId::new("acme/widgets#2").expect("id");
        let rerun_calls = Arc::new(AtomicUsize::new(0));
        let provider = StubProvider {
            pull_requests: vec![],
            notifications: NotificationPage {
                threads: Vec::new(),
                poll_interval: None,
                last_modified: None,
                not_modified: false,
            },
            read_calls: StdMutex::new(Vec::new()),
            done_calls: StdMutex::new(Vec::new()),
            pipelines: Some(sample_pipelines(CheckRollupState::Pending)),
            rerun_calls: rerun_calls.clone(),
            rerun_allowed: true,
        };

        let snapshot = rerun_check_via_store(
            &provider,
            &store,
            &id,
            &RerunTarget::WorkflowRun { run_id: 99 },
        )
        .await
        .expect("rerun");
        assert_eq!(snapshot.rollup.state, CheckRollupState::Pending);
        assert_eq!(rerun_calls.load(Ordering::SeqCst), 1);
    }

    #[tokio::test]
    async fn gated_rerun_does_not_update_localstore() {
        let store = temp_store().await;
        let id = PullRequestId::new("acme/widgets#3").expect("id");
        let cached = sample_pipelines(CheckRollupState::Failure);
        store
            .put_pipelines_locked(&id, &cached)
            .await
            .expect("seed");

        let provider = StubProvider {
            pull_requests: vec![],
            notifications: NotificationPage {
                threads: Vec::new(),
                poll_interval: None,
                last_modified: None,
                not_modified: false,
            },
            read_calls: StdMutex::new(Vec::new()),
            done_calls: StdMutex::new(Vec::new()),
            pipelines: Some(sample_pipelines(CheckRollupState::Success)),
            rerun_calls: Arc::new(AtomicUsize::new(0)),
            rerun_allowed: false,
        };

        let err = rerun_check_via_store(
            &provider,
            &store,
            &id,
            &RerunTarget::WorkflowRun { run_id: 1 },
        )
        .await
        .expect_err("gated");

        assert!(err.to_string().contains("not allowed"));

        let still = store
            .get_pipelines(&id)
            .await
            .expect("get")
            .expect("cached");
        assert_eq!(still.rollup.state, CheckRollupState::Failure);
    }
}
