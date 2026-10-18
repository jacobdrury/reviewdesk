//! Pending review and Thread write-through orchestration (ADR-0004).

use reviewdesk_core::{
    PendingDiffReviewComment, Provider, ProviderError, PullRequestId, PullRequestThreads,
    ReviewSubmitEvent,
};

use crate::sync::AppLocalStore;

pub async fn load_threads_from_store(
    store: &AppLocalStore,
    id: &PullRequestId,
) -> Result<Option<PullRequestThreads>, ProviderError> {
    store
        .get_pull_request_threads(id)
        .await
        .map_err(|e| ProviderError::other(e.to_string()))
}

pub async fn load_threads_via_store(
    provider: &dyn Provider,
    store: &AppLocalStore,
    id: &PullRequestId,
) -> Result<PullRequestThreads, ProviderError> {
    if let Some(cached) = load_threads_from_store(store, id).await? {
        return Ok(cached);
    }
    refresh_threads_from_provider(provider, store, id).await
}

pub async fn refresh_threads_from_provider(
    provider: &dyn Provider,
    store: &AppLocalStore,
    id: &PullRequestId,
) -> Result<PullRequestThreads, ProviderError> {
    let snapshot = provider.fetch_pull_request_threads(id).await?;
    store
        .put_pull_request_threads(id, &snapshot)
        .await
        .map_err(|e| ProviderError::other(e.to_string()))?;
    Ok(snapshot)
}

pub async fn add_pending_diff_review_comment(
    provider: &dyn Provider,
    store: &AppLocalStore,
    id: &PullRequestId,
    comment: PendingDiffReviewComment,
) -> Result<PullRequestThreads, ProviderError> {
    let snapshot = provider
        .add_pending_diff_review_comment(id, comment)
        .await?;
    store
        .put_pull_request_threads(id, &snapshot)
        .await
        .map_err(|e| ProviderError::other(e.to_string()))?;
    Ok(snapshot)
}

pub async fn submit_pending_review(
    provider: &dyn Provider,
    store: &AppLocalStore,
    id: &PullRequestId,
    event: ReviewSubmitEvent,
    summary_body: Option<String>,
) -> Result<PullRequestThreads, ProviderError> {
    let snapshot = provider
        .submit_pending_review(id, event, summary_body)
        .await?;
    store
        .put_pull_request_threads(id, &snapshot)
        .await
        .map_err(|e| ProviderError::other(e.to_string()))?;
    Ok(snapshot)
}

pub async fn discard_pending_review(
    provider: &dyn Provider,
    store: &AppLocalStore,
    id: &PullRequestId,
) -> Result<PullRequestThreads, ProviderError> {
    let snapshot = provider.discard_pending_review(id).await?;
    store
        .put_pull_request_threads(id, &snapshot)
        .await
        .map_err(|e| ProviderError::other(e.to_string()))?;
    Ok(snapshot)
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;
    use std::sync::{Arc, Mutex};

    use async_trait::async_trait;
    use reviewdesk_core::{
        DiffAnchor, DiffLineSide, NotificationListOptions, NotificationPage, NotificationThreadId,
        Patch, PendingReview, PullRequestDetail, PullRequestListItem, PullRequestListOptions,
        ReviewComment, ReviewCommentId, Thread, ThreadId, ThreadStatus, suggestion_from_body,
    };
    use reviewdesk_localstore::TursoLocalStore;
    use tempfile::tempdir;

    use super::*;

    struct MemoryProvider {
        threads: Mutex<HashMap<String, PullRequestThreads>>,
        fail_writes: Mutex<bool>,
    }

    impl MemoryProvider {
        fn new() -> Self {
            Self {
                threads: Mutex::new(HashMap::new()),
                fail_writes: Mutex::new(false),
            }
        }

        fn set_fail_writes(&self, fail: bool) {
            *self.fail_writes.lock().expect("lock") = fail;
        }
    }

    fn pr_id(raw: &str) -> PullRequestId {
        PullRequestId::new(raw).expect("id")
    }

    #[async_trait]
    impl Provider for MemoryProvider {
        async fn current_user(&self) -> Result<String, ProviderError> {
            Ok("test-user".into())
        }

        async fn list_pull_requests(
            &self,
            _options: &PullRequestListOptions,
        ) -> Result<Vec<PullRequestListItem>, ProviderError> {
            Ok(Vec::new())
        }

        async fn fetch_patch(&self, _id: &PullRequestId) -> Result<Patch, ProviderError> {
            Err(ProviderError::other("unused"))
        }

        async fn fetch_pull_request(
            &self,
            _id: &PullRequestId,
        ) -> Result<PullRequestDetail, ProviderError> {
            Err(ProviderError::other("unused"))
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
            Err(ProviderError::other("unused"))
        }

        async fn mark_notification_done(
            &self,
            _id: &NotificationThreadId,
        ) -> Result<(), ProviderError> {
            Err(ProviderError::other("unused"))
        }

        async fn fetch_pull_request_threads(
            &self,
            id: &PullRequestId,
        ) -> Result<PullRequestThreads, ProviderError> {
            Ok(self
                .threads
                .lock()
                .expect("lock")
                .get(id.as_str())
                .cloned()
                .unwrap_or_else(|| PullRequestThreads {
                    threads: Vec::new(),
                    pending_review: None,
                }))
        }

        async fn add_pending_diff_review_comment(
            &self,
            id: &PullRequestId,
            comment: PendingDiffReviewComment,
        ) -> Result<PullRequestThreads, ProviderError> {
            if *self.fail_writes.lock().expect("lock") {
                return Err(ProviderError::other("provider write failed"));
            }

            let mut map = self.threads.lock().expect("lock");
            let mut snapshot = map.get(id.as_str()).cloned().unwrap_or_else(|| {
                PullRequestThreads {
                    threads: Vec::new(),
                    pending_review: None,
                }
            });

            if snapshot.pending_review.is_none() {
                snapshot.pending_review = Some(PendingReview {
                    pull_request_id: id.clone(),
                    provider_review_id: Some("pending-1".into()),
                    summary_body: None,
                });
            }

            let suggestion = suggestion_from_body(&comment.body);
            let new_comment = ReviewComment {
                id: ReviewCommentId::new(format!(
                    "c-{}",
                    snapshot
                        .threads
                        .iter()
                        .map(|t| t.comments.len())
                        .sum::<usize>()
                        + 1
                )),
                author_login: "viewer".into(),
                body: comment.body,
                is_pending: true,
                suggestion,
            };

            if let Some(thread_id) = comment.reply_to_thread {
                let thread = snapshot
                    .threads
                    .iter_mut()
                    .find(|t| t.id.as_str() == thread_id.as_str())
                    .ok_or(ProviderError::NotFound)?;
                thread.comments.push(new_comment);
            } else {
                let thread_id = ThreadId::new(format!("t-{}", snapshot.threads.len() + 1));
                snapshot.threads.push(Thread {
                    id: thread_id,
                    anchor: Some(comment.anchor),
                    status: ThreadStatus::Open,
                    comments: vec![new_comment],
                });
            }

            map.insert(id.as_str().to_string(), snapshot.clone());
            Ok(snapshot)
        }

        async fn submit_pending_review(
            &self,
            id: &PullRequestId,
            _event: ReviewSubmitEvent,
            _summary_body: Option<String>,
        ) -> Result<PullRequestThreads, ProviderError> {
            if *self.fail_writes.lock().expect("lock") {
                return Err(ProviderError::other("provider write failed"));
            }
            let mut map = self.threads.lock().expect("lock");
            let Some(mut snapshot) = map.get(id.as_str()).cloned() else {
                return Ok(PullRequestThreads {
                    threads: Vec::new(),
                    pending_review: None,
                });
            };
            for thread in &mut snapshot.threads {
                for comment in &mut thread.comments {
                    comment.is_pending = false;
                }
            }
            snapshot.pending_review = None;
            map.insert(id.as_str().to_string(), snapshot.clone());
            Ok(snapshot)
        }

        async fn discard_pending_review(
            &self,
            id: &PullRequestId,
        ) -> Result<PullRequestThreads, ProviderError> {
            if *self.fail_writes.lock().expect("lock") {
                return Err(ProviderError::other("provider write failed"));
            }
            let mut map = self.threads.lock().expect("lock");
            let Some(mut snapshot) = map.get(id.as_str()).cloned() else {
                return Ok(PullRequestThreads {
                    threads: Vec::new(),
                    pending_review: None,
                });
            };
            for thread in &mut snapshot.threads {
                thread
                    .comments
                    .retain(|comment| !comment.is_pending);
            }
            snapshot
                .threads
                .retain(|thread| !thread.comments.is_empty());
            snapshot.pending_review = None;
            map.insert(id.as_str().to_string(), snapshot.clone());
            Ok(snapshot)
        }
    }

    async fn open_app_store() -> (tempfile::TempDir, AppLocalStore) {
        let dir = tempdir().expect("tempdir");
        let path = dir.path().join("reviewdesk.db");
        let inner = TursoLocalStore::open(&path).await.expect("open");
        (dir, AppLocalStore::new(Arc::new(inner)))
    }

    fn sample_comment() -> PendingDiffReviewComment {
        PendingDiffReviewComment {
            reply_to_thread: None,
            anchor: DiffAnchor {
                path: "src/lib.rs".into(),
                side: DiffLineSide::Right,
                start_line: 10,
                end_line: 10,
            },
            body: "Please adjust this line.".into(),
        }
    }

    #[tokio::test]
    async fn pending_review_draft_survives_store_reopen() {
        let (_dir, store) = open_app_store().await;
        let provider = MemoryProvider::new();
        let id = pr_id("acme/widgets#1");

        add_pending_diff_review_comment(&provider, &store, &id, sample_comment())
            .await
            .expect("write");

        let cached = store
            .get_pull_request_threads(&id)
            .await
            .expect("read")
            .expect("snapshot");
        assert!(cached.pending_review.is_some());
        assert_eq!(cached.threads.len(), 1);
        assert!(cached.threads[0].comments[0].is_pending);

        let reloaded = load_threads_from_store(&store, &id)
            .await
            .expect("load")
            .expect("rows");
        assert_eq!(reloaded, cached);
    }

    #[tokio::test]
    async fn submit_clears_pending_review_from_store() {
        let (_dir, store) = open_app_store().await;
        let provider = MemoryProvider::new();
        let id = pr_id("acme/widgets#2");

        add_pending_diff_review_comment(&provider, &store, &id, sample_comment())
            .await
            .expect("write");
        submit_pending_review(
            &provider,
            &store,
            &id,
            ReviewSubmitEvent::Comment,
            None,
        )
        .await
        .expect("submit");

        let cached = store
            .get_pull_request_threads(&id)
            .await
            .expect("read")
            .expect("snapshot");
        assert!(cached.pending_review.is_none());
        assert!(!cached.threads[0].comments[0].is_pending);
    }

    #[tokio::test]
    async fn discard_clears_pending_drafts_from_store() {
        let (_dir, store) = open_app_store().await;
        let provider = MemoryProvider::new();
        let id = pr_id("acme/widgets#3");

        add_pending_diff_review_comment(&provider, &store, &id, sample_comment())
            .await
            .expect("write");
        discard_pending_review(&provider, &store, &id)
            .await
            .expect("discard");

        let cached = store
            .get_pull_request_threads(&id)
            .await
            .expect("read")
            .expect("snapshot");
        assert!(cached.pending_review.is_none());
        assert!(cached.threads.is_empty());
    }

    #[tokio::test]
    async fn provider_failure_does_not_write_through_to_store() {
        let (_dir, store) = open_app_store().await;
        let provider = MemoryProvider::new();
        provider.set_fail_writes(true);
        let id = pr_id("acme/widgets#4");

        let err = add_pending_diff_review_comment(&provider, &store, &id, sample_comment())
            .await
            .expect_err("expected failure");
        assert!(matches!(err, ProviderError::Other(_)));

        let cached = store
            .get_pull_request_threads(&id)
            .await
            .expect("read");
        assert!(cached.is_none());
    }
}
