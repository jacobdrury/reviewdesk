//! Pending review and Thread write-through orchestration (ADR-0004).

use reviewdesk_core::{
    PendingDiffReviewComment, Provider, ProviderError, PullRequestId, PullRequestThreads,
    ReviewSubmitEvent, ThreadId, ThreadStatus, TimelineConversationComment,
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

pub async fn set_thread_status(
    provider: &dyn Provider,
    store: &AppLocalStore,
    id: &PullRequestId,
    thread_id: &ThreadId,
    status: ThreadStatus,
) -> Result<PullRequestThreads, ProviderError> {
    let snapshot = provider
        .set_thread_status(id, thread_id, status)
        .await?;
    store
        .put_pull_request_threads(id, &snapshot)
        .await
        .map_err(|e| ProviderError::other(e.to_string()))?;
    Ok(snapshot)
}

pub async fn publish_timeline_comment(
    provider: &dyn Provider,
    store: &AppLocalStore,
    id: &PullRequestId,
    comment: TimelineConversationComment,
) -> Result<PullRequestThreads, ProviderError> {
    let snapshot = provider.publish_timeline_comment(id, comment).await?;
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
        Patch, PendingReview, PipelinesSnapshot, PullRequestDetail, PullRequestId,
        PullRequestListItem, PullRequestListOptions, RerunTarget, ReviewComment, ReviewCommentId,
        Thread, ThreadId, ThreadStatus, suggestion_from_body,
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

        async fn fetch_pipelines(
            &self,
            _id: &PullRequestId,
        ) -> Result<PipelinesSnapshot, ProviderError> {
            Err(ProviderError::other("unused"))
        }

        async fn rerun_check(
            &self,
            _id: &PullRequestId,
            _target: &RerunTarget,
        ) -> Result<(), ProviderError> {
            Err(ProviderError::other("unused"))
        }

        async fn set_thread_status(
            &self,
            id: &PullRequestId,
            thread_id: &ThreadId,
            status: ThreadStatus,
        ) -> Result<PullRequestThreads, ProviderError> {
            if *self.fail_writes.lock().expect("lock") {
                return Err(ProviderError::other("provider write failed"));
            }
            let mut map = self.threads.lock().expect("lock");
            let mut snapshot = map.get(id.as_str()).cloned().unwrap_or_default();
            let thread = snapshot
                .threads
                .iter_mut()
                .find(|t| t.id.as_str() == thread_id.as_str())
                .ok_or(ProviderError::NotFound)?;
            if matches!(thread.status, ThreadStatus::Outdated) {
                return Err(ProviderError::other("cannot resolve outdated thread"));
            }
            thread.status = status;
            map.insert(id.as_str().to_string(), snapshot.clone());
            Ok(snapshot)
        }

        async fn publish_timeline_comment(
            &self,
            id: &PullRequestId,
            comment: TimelineConversationComment,
        ) -> Result<PullRequestThreads, ProviderError> {
            if *self.fail_writes.lock().expect("lock") {
                return Err(ProviderError::other("provider write failed"));
            }
            let mut map = self.threads.lock().expect("lock");
            let mut snapshot = map.get(id.as_str()).cloned().unwrap_or_default();
            let suggestion = suggestion_from_body(&comment.body);
            let new_comment = ReviewComment {
                id: ReviewCommentId::new(format!(
                    "c-timeline-{}",
                    snapshot
                        .threads
                        .iter()
                        .map(|t| t.comments.len())
                        .sum::<usize>()
                        + 1
                )),
                author_login: "viewer".into(),
                body: comment.body,
                is_pending: false,
                suggestion,
            };
            snapshot.threads.push(Thread {
                id: ThreadId::new(format!("t-timeline-{}", snapshot.threads.len() + 1)),
                anchor: None,
                status: ThreadStatus::Open,
                comments: vec![new_comment],
            });
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

    fn sample_open_thread() -> Thread {
        Thread {
            id: ThreadId::new("t-resolve"),
            anchor: Some(DiffAnchor {
                path: "src/lib.rs".into(),
                side: DiffLineSide::Right,
                start_line: 1,
                end_line: 1,
            }),
            status: ThreadStatus::Open,
            comments: vec![ReviewComment {
                id: ReviewCommentId::new("c-1"),
                author_login: "alice".into(),
                body: "Please fix.".into(),
                is_pending: false,
                suggestion: None,
            }],
        }
    }

    async fn seed_threads(provider: &MemoryProvider, store: &AppLocalStore, id: &PullRequestId) {
        let snapshot = PullRequestThreads {
            threads: vec![sample_open_thread()],
            pending_review: None,
        };
        provider
            .threads
            .lock()
            .expect("lock")
            .insert(id.as_str().to_string(), snapshot.clone());
        store
            .put_pull_request_threads(id, &snapshot)
            .await
            .expect("seed");
    }

    #[tokio::test]
    async fn resolve_thread_write_through_updates_store() {
        let (_dir, store) = open_app_store().await;
        let provider = MemoryProvider::new();
        let id = pr_id("acme/widgets#10");
        seed_threads(&provider, &store, &id).await;

        set_thread_status(
            &provider,
            &store,
            &id,
            &ThreadId::new("t-resolve"),
            ThreadStatus::Resolved,
        )
        .await
        .expect("resolve");

        let cached = store
            .get_pull_request_threads(&id)
            .await
            .expect("read")
            .expect("snapshot");
        assert!(matches!(
            cached.threads[0].status,
            ThreadStatus::Resolved
        ));
    }

    #[tokio::test]
    async fn timeline_comment_publishes_immediately_not_pending_review() {
        let (_dir, store) = open_app_store().await;
        let provider = MemoryProvider::new();
        let id = pr_id("acme/widgets#11");

        publish_timeline_comment(
            &provider,
            &store,
            &id,
            TimelineConversationComment {
                body: "Ship when green.".into(),
            },
        )
        .await
        .expect("publish");

        let cached = store
            .get_pull_request_threads(&id)
            .await
            .expect("read")
            .expect("snapshot");
        assert!(cached.pending_review.is_none());
        assert_eq!(cached.threads.len(), 1);
        assert!(cached.threads[0].anchor.is_none());
        assert!(!cached.threads[0].comments[0].is_pending);
        assert_eq!(cached.threads[0].comments[0].body, "Ship when green.");
    }

    #[tokio::test]
    async fn resolve_failure_does_not_write_through_to_store() {
        let (_dir, store) = open_app_store().await;
        let provider = MemoryProvider::new();
        let id = pr_id("acme/widgets#12");
        seed_threads(&provider, &store, &id).await;
        provider.set_fail_writes(true);

        let err = set_thread_status(
            &provider,
            &store,
            &id,
            &ThreadId::new("t-resolve"),
            ThreadStatus::Resolved,
        )
        .await
        .expect_err("expected failure");
        assert!(matches!(err, ProviderError::Other(_)));

        let cached = store
            .get_pull_request_threads(&id)
            .await
            .expect("read")
            .expect("snapshot");
        assert!(matches!(cached.threads[0].status, ThreadStatus::Open));
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
