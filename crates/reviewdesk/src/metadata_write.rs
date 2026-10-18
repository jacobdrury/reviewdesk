//! Provider write-through orchestration for Metadata manage mutations.

use reviewdesk_core::{MergeMethod, Provider, ProviderError, PullRequestDetail, PullRequestId};

use crate::sync::AppLocalStore;

/// Metadata manage mutation the app orchestration layer executes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MetadataWrite {
    SetLabels(Vec<String>),
    SetAssignees(Vec<String>),
    RequestReviewer(String),
    RemoveReviewer(String),
    RerequestReviewer(String),
    SetDraft(bool),
    SetBaseRef(String),
    Merge(MergeMethod),
    SetAutoMerge { enabled: bool, method: MergeMethod },
}

/// Call Provider → write-through LocalStore on success → targeted refetch (refetch wins).
pub async fn apply_metadata_write(
    provider: &dyn Provider,
    store: &AppLocalStore,
    id: &PullRequestId,
    write: MetadataWrite,
) -> Result<PullRequestDetail, ProviderError> {
    run_provider_write(provider, id, &write).await?;

    let write_through = provider.fetch_pull_request(id).await?;
    store
        .put_pull_request_detail(&write_through)
        .await
        .map_err(|e| ProviderError::other(e.to_string()))?;
    sync_list_row_from_detail(store, &write_through).await?;

    let refetched = provider.fetch_pull_request(id).await?;
    store
        .put_pull_request_detail(&refetched)
        .await
        .map_err(|e| ProviderError::other(e.to_string()))?;
    sync_list_row_from_detail(store, &refetched).await?;

    Ok(refetched)
}

async fn run_provider_write(
    provider: &dyn Provider,
    id: &PullRequestId,
    write: &MetadataWrite,
) -> Result<(), ProviderError> {
    match write {
        MetadataWrite::SetLabels(labels) => provider.set_pull_request_labels(id, labels).await,
        MetadataWrite::SetAssignees(logins) => {
            provider.set_pull_request_assignees(id, logins).await
        }
        MetadataWrite::RequestReviewer(login) => {
            provider.request_pull_request_reviewer(id, login).await
        }
        MetadataWrite::RemoveReviewer(login) => {
            provider.remove_pull_request_reviewer(id, login).await
        }
        MetadataWrite::RerequestReviewer(login) => {
            provider.rerequest_pull_request_reviewer(id, login).await
        }
        MetadataWrite::SetDraft(is_draft) => provider.set_pull_request_draft(id, *is_draft).await,
        MetadataWrite::SetBaseRef(base) => provider.set_pull_request_base(id, base).await,
        MetadataWrite::Merge(method) => provider.merge_pull_request(id, *method).await,
        MetadataWrite::SetAutoMerge { enabled, method } => {
            provider
                .set_pull_request_auto_merge(id, *enabled, *method)
                .await
        }
    }
}

async fn sync_list_row_from_detail(
    store: &AppLocalStore,
    detail: &PullRequestDetail,
) -> Result<(), ProviderError> {
    let rows = store
        .query_pull_requests(&reviewdesk_core::PullRequestListQuery::default())
        .await
        .map_err(|e| ProviderError::other(e.to_string()))?;
    let Some(mut row) = rows.into_iter().find(|r| r.id == detail.id) else {
        return Ok(());
    };
    row.title = detail.title.clone();
    row.is_draft = detail.is_draft;
    row.labels = detail.labels.clone();
    row.web_url = detail.web_url.clone();
    store
        .sync_pull_requests(&[row])
        .await
        .map_err(|e| ProviderError::other(e.to_string()))?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use std::sync::{Arc, Mutex as StdMutex};

    use async_trait::async_trait;
    use reviewdesk_core::{
        LocalStore, LocalStoreError, MetadataWriteCapabilities, NotificationListOptions,
        NotificationPage, NotificationQuery, NotificationThread, NotificationThreadId, Patch,
        PullRequestLabel, PullRequestListItem, PullRequestThreads, ReviewDecision,
        ReviewSubmitEvent, View, ViewId,
    };
    use tempfile::tempdir;

    use super::*;
    use crate::sync::AppLocalStore;
    use reviewdesk_localstore::TursoLocalStore;

    struct DetailStore {
        detail: StdMutex<Option<PullRequestDetail>>,
        puts: StdMutex<Vec<PullRequestDetail>>,
    }

    #[async_trait]
    impl LocalStore for DetailStore {
        async fn schema_version(&self) -> Result<u32, LocalStoreError> {
            Ok(1)
        }

        async fn sync_pull_requests(
            &self,
            _items: &[PullRequestListItem],
        ) -> Result<(), LocalStoreError> {
            Ok(())
        }

        async fn query_pull_requests(
            &self,
            _query: &reviewdesk_core::PullRequestListQuery,
        ) -> Result<Vec<PullRequestListItem>, LocalStoreError> {
            Ok(Vec::new())
        }

        async fn put_pull_request_detail(
            &self,
            detail: &PullRequestDetail,
        ) -> Result<(), LocalStoreError> {
            self.puts.lock().expect("lock").push(detail.clone());
            *self.detail.lock().expect("lock") = Some(detail.clone());
            Ok(())
        }

        async fn get_pull_request_detail(
            &self,
            _id: &PullRequestId,
        ) -> Result<Option<PullRequestDetail>, LocalStoreError> {
            Ok(self.detail.lock().expect("lock").clone())
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
            _threads: &[NotificationThread],
        ) -> Result<(), LocalStoreError> {
            Ok(())
        }

        async fn query_notifications(
            &self,
            _query: &NotificationQuery,
        ) -> Result<Vec<NotificationThread>, LocalStoreError> {
            Ok(Vec::new())
        }

        async fn set_notification_read(
            &self,
            _id: &NotificationThreadId,
        ) -> Result<(), LocalStoreError> {
            Ok(())
        }

        async fn remove_notification(
            &self,
            _id: &NotificationThreadId,
        ) -> Result<(), LocalStoreError> {
            Ok(())
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

        async fn put_pipelines(
            &self,
            _id: &PullRequestId,
            _snapshot: &reviewdesk_core::PipelinesSnapshot,
        ) -> Result<(), LocalStoreError> {
            Ok(())
        }

        async fn get_pipelines(
            &self,
            _id: &PullRequestId,
        ) -> Result<Option<reviewdesk_core::PipelinesSnapshot>, LocalStoreError> {
            Ok(None)
        }
    }

    struct StubProvider {
        fetch_count: StdMutex<u32>,
        write_through_title: String,
        refetch_title: String,
        fail_write: bool,
        fail_auto_merge: bool,
        write_through_auto_merge: bool,
        refetch_auto_merge: bool,
    }

    #[async_trait]
    impl Provider for StubProvider {
        async fn current_user(&self) -> Result<String, ProviderError> {
            Ok("me".into())
        }

        async fn list_pull_requests(
            &self,
            _options: &reviewdesk_core::PullRequestListOptions,
        ) -> Result<Vec<PullRequestListItem>, ProviderError> {
            Ok(Vec::new())
        }

        async fn fetch_patch(&self, _id: &PullRequestId) -> Result<Patch, ProviderError> {
            Err(ProviderError::other("unused"))
        }

        async fn fetch_pull_request(
            &self,
            id: &PullRequestId,
        ) -> Result<PullRequestDetail, ProviderError> {
            let mut count = self.fetch_count.lock().expect("lock");
            *count += 1;
            let title = if *count == 1 {
                self.write_through_title.clone()
            } else {
                self.refetch_title.clone()
            };
            let auto_merge = if *count == 1 {
                self.write_through_auto_merge
            } else {
                self.refetch_auto_merge
            };
            Ok(sample_detail(id, &title, auto_merge))
        }

        async fn set_pull_request_labels(
            &self,
            _id: &PullRequestId,
            _labels: &[String],
        ) -> Result<(), ProviderError> {
            if self.fail_write {
                return Err(ProviderError::other("write failed"));
            }
            Ok(())
        }

        async fn merge_pull_request(
            &self,
            _id: &PullRequestId,
            _method: MergeMethod,
        ) -> Result<(), ProviderError> {
            if self.fail_write {
                return Err(ProviderError::other("write failed"));
            }
            Ok(())
        }

        async fn set_pull_request_auto_merge(
            &self,
            _id: &PullRequestId,
            _enabled: bool,
            _method: MergeMethod,
        ) -> Result<(), ProviderError> {
            if self.fail_auto_merge {
                return Err(ProviderError::other("auto-merge not supported"));
            }
            Ok(())
        }

        async fn list_notifications(
            &self,
            _options: &NotificationListOptions,
        ) -> Result<NotificationPage, ProviderError> {
            Err(ProviderError::other("unused"))
        }

        async fn mark_notification_read(
            &self,
            _id: &NotificationThreadId,
        ) -> Result<(), ProviderError> {
            Ok(())
        }

        async fn mark_notification_done(
            &self,
            _id: &NotificationThreadId,
        ) -> Result<(), ProviderError> {
            Ok(())
        }

        async fn fetch_pull_request_threads(
            &self,
            _id: &PullRequestId,
        ) -> Result<PullRequestThreads, ProviderError> {
            Err(ProviderError::other("unused"))
        }

        async fn add_pending_diff_review_comment(
            &self,
            _id: &PullRequestId,
            _comment: reviewdesk_core::PendingDiffReviewComment,
        ) -> Result<PullRequestThreads, ProviderError> {
            Err(ProviderError::other("unused"))
        }

        async fn submit_pending_review(
            &self,
            _id: &PullRequestId,
            _event: ReviewSubmitEvent,
            _summary_body: Option<String>,
        ) -> Result<PullRequestThreads, ProviderError> {
            Err(ProviderError::other("unused"))
        }

        async fn discard_pending_review(
            &self,
            _id: &PullRequestId,
        ) -> Result<PullRequestThreads, ProviderError> {
            Err(ProviderError::other("unused"))
        }

        async fn fetch_pipelines(
            &self,
            _id: &PullRequestId,
        ) -> Result<reviewdesk_core::PipelinesSnapshot, ProviderError> {
            Err(ProviderError::other("unused"))
        }

        async fn rerun_check(
            &self,
            _id: &PullRequestId,
            _target: &reviewdesk_core::RerunTarget,
        ) -> Result<(), ProviderError> {
            Err(ProviderError::other("unused"))
        }

        async fn set_thread_status(
            &self,
            _id: &PullRequestId,
            _thread_id: &reviewdesk_core::ThreadId,
            _status: reviewdesk_core::ThreadStatus,
        ) -> Result<reviewdesk_core::PullRequestThreads, ProviderError> {
            Err(ProviderError::other("unused"))
        }

        async fn publish_timeline_comment(
            &self,
            _id: &PullRequestId,
            _comment: reviewdesk_core::TimelineConversationComment,
        ) -> Result<reviewdesk_core::PullRequestThreads, ProviderError> {
            Err(ProviderError::other("unused"))
        }
    }

    fn pr_id(raw: &str) -> PullRequestId {
        PullRequestId::new(raw).unwrap()
    }

    fn sample_detail(
        id: &PullRequestId,
        title: &str,
        auto_merge_enabled: bool,
    ) -> PullRequestDetail {
        PullRequestDetail {
            id: id.clone(),
            title: title.into(),
            body: String::new(),
            author_login: "alice".into(),
            author_avatar_url: None,
            is_draft: false,
            labels: vec![PullRequestLabel {
                name: "bug".into(),
                color: "ff0000".into(),
            }],
            web_url: "https://example.test".into(),
            reviewers: Vec::new(),
            assignees: Vec::new(),
            head_ref: "feature".into(),
            base_ref: "main".into(),
            review_decision: ReviewDecision::ReviewRequired,
            mergeable: reviewdesk_core::MergeableState::Mergeable,
            state: reviewdesk_core::PullRequestState::Open,
            auto_merge_enabled,
            codeowners_rules: Vec::new(),
            write_capabilities: MetadataWriteCapabilities {
                labels: true,
                assignees: false,
                reviewers: false,
                draft_ready: true,
                base_ref: false,
                merge: true,
                auto_merge: true,
            },
        }
    }

    #[tokio::test]
    async fn success_write_through_then_refetch_wins_in_local_store() {
        let id = pr_id("acme/widgets#1");
        let inner = Arc::new(DetailStore {
            detail: StdMutex::new(Some(sample_detail(&id, "before", false))),
            puts: StdMutex::new(Vec::new()),
        });
        let store = AppLocalStore::new(inner.clone());
        let provider = StubProvider {
            fetch_count: StdMutex::new(0),
            write_through_title: "write-through".into(),
            refetch_title: "refetch-wins".into(),
            fail_write: false,
            fail_auto_merge: false,
            write_through_auto_merge: false,
            refetch_auto_merge: false,
        };

        let result = apply_metadata_write(
            &provider,
            &store,
            &id,
            MetadataWrite::SetLabels(vec!["bug".into()]),
        )
        .await
        .expect("write");

        assert_eq!(result.title, "refetch-wins");
        let cached = store
            .get_pull_request_detail(&id)
            .await
            .expect("get")
            .expect("row");
        assert_eq!(cached.title, "refetch-wins");
        let puts = inner.puts.lock().expect("lock");
        assert_eq!(puts.len(), 2);
        assert_eq!(puts[0].title, "write-through");
        assert_eq!(puts[1].title, "refetch-wins");
    }

    #[tokio::test]
    async fn failed_provider_write_leaves_local_store_unchanged() {
        let dir = tempdir().expect("tempdir");
        let db_path = dir.path().join("reviewdesk.db");
        let inner = Arc::new(TursoLocalStore::open(&db_path).await.expect("open store"));
        let store = AppLocalStore::new(inner);
        let id = pr_id("acme/widgets#2");
        let seed = sample_detail(&id, "seed", false);
        store.put_pull_request_detail(&seed).await.expect("seed");

        let provider = StubProvider {
            fetch_count: StdMutex::new(0),
            write_through_title: "ignored".into(),
            refetch_title: "ignored".into(),
            fail_write: true,
            fail_auto_merge: false,
            write_through_auto_merge: false,
            refetch_auto_merge: false,
        };

        let err = apply_metadata_write(
            &provider,
            &store,
            &id,
            MetadataWrite::SetLabels(vec!["bug".into()]),
        )
        .await
        .expect_err("should fail");

        assert!(matches!(err, ProviderError::Other(_)));
        let cached = store
            .get_pull_request_detail(&id)
            .await
            .expect("get")
            .expect("row");
        assert_eq!(cached.title, "seed");
    }

    #[tokio::test]
    async fn merge_success_write_through_then_refetch_wins_in_local_store() {
        let id = pr_id("acme/widgets#3");
        let inner = Arc::new(DetailStore {
            detail: StdMutex::new(Some(sample_detail(&id, "before", false))),
            puts: StdMutex::new(Vec::new()),
        });
        let store = AppLocalStore::new(inner.clone());
        let provider = StubProvider {
            fetch_count: StdMutex::new(0),
            write_through_title: "merged-write-through".into(),
            refetch_title: "merged-refetch".into(),
            fail_write: false,
            fail_auto_merge: false,
            write_through_auto_merge: false,
            refetch_auto_merge: false,
        };

        let result = apply_metadata_write(
            &provider,
            &store,
            &id,
            MetadataWrite::Merge(MergeMethod::Squash),
        )
        .await
        .expect("merge");

        assert_eq!(result.title, "merged-refetch");
        let cached = store
            .get_pull_request_detail(&id)
            .await
            .expect("get")
            .expect("row");
        assert_eq!(cached.title, "merged-refetch");
    }

    #[tokio::test]
    async fn auto_merge_enable_success_refetch_wins_in_local_store() {
        let id = pr_id("acme/widgets#4");
        let inner = Arc::new(DetailStore {
            detail: StdMutex::new(Some(sample_detail(&id, "before", false))),
            puts: StdMutex::new(Vec::new()),
        });
        let store = AppLocalStore::new(inner.clone());
        let provider = StubProvider {
            fetch_count: StdMutex::new(0),
            write_through_title: "ignored".into(),
            refetch_title: "ignored".into(),
            fail_write: false,
            fail_auto_merge: false,
            write_through_auto_merge: true,
            refetch_auto_merge: true,
        };

        let result = apply_metadata_write(
            &provider,
            &store,
            &id,
            MetadataWrite::SetAutoMerge {
                enabled: true,
                method: MergeMethod::Squash,
            },
        )
        .await
        .expect("auto-merge");

        assert!(result.auto_merge_enabled);
        let cached = store
            .get_pull_request_detail(&id)
            .await
            .expect("get")
            .expect("row");
        assert!(cached.auto_merge_enabled);
    }

    #[tokio::test]
    async fn gated_auto_merge_failure_leaves_local_store_unchanged() {
        let dir = tempdir().expect("tempdir");
        let db_path = dir.path().join("reviewdesk.db");
        let inner = Arc::new(TursoLocalStore::open(&db_path).await.expect("open store"));
        let store = AppLocalStore::new(inner);
        let id = pr_id("acme/widgets#5");
        let seed = sample_detail(&id, "seed", false);
        store.put_pull_request_detail(&seed).await.expect("seed");

        let provider = StubProvider {
            fetch_count: StdMutex::new(0),
            write_through_title: "ignored".into(),
            refetch_title: "ignored".into(),
            fail_write: false,
            fail_auto_merge: true,
            write_through_auto_merge: true,
            refetch_auto_merge: true,
        };

        let err = apply_metadata_write(
            &provider,
            &store,
            &id,
            MetadataWrite::SetAutoMerge {
                enabled: true,
                method: MergeMethod::Squash,
            },
        )
        .await
        .expect_err("should fail");

        assert!(matches!(err, ProviderError::Other(_)));
        let cached = store
            .get_pull_request_detail(&id)
            .await
            .expect("get")
            .expect("row");
        assert!(!cached.auto_merge_enabled);
        assert_eq!(cached.title, "seed");
    }
}
