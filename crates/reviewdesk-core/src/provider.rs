use async_trait::async_trait;

use crate::error::ProviderError;
use crate::id::PullRequestId;
use crate::notification::{NotificationListOptions, NotificationPage, NotificationThreadId};
use crate::patch::Patch;
use crate::pipelines::{PipelinesSnapshot, RerunTarget};
use crate::pull_request::PullRequestDetail;
use crate::pull_request_list::{PullRequestListItem, PullRequestListOptions};
use crate::review_threads::{
    PendingDiffReviewComment, PullRequestThreads, ReviewSubmitEvent,
};

/// Host that exposes pull requests, their patches, and the notification Inbox.
#[async_trait]
pub trait Provider: Send + Sync {
    /// Login of the single signed-in identity, used to bind "I am author / reviewer" Views.
    async fn current_user(&self) -> Result<String, ProviderError>;

    async fn list_pull_requests(
        &self,
        options: &PullRequestListOptions,
    ) -> Result<Vec<PullRequestListItem>, ProviderError>;

    async fn fetch_patch(&self, id: &PullRequestId) -> Result<Patch, ProviderError>;

    async fn fetch_pull_request(
        &self,
        id: &PullRequestId,
    ) -> Result<PullRequestDetail, ProviderError>;

    /// Polls the notification Inbox, honoring conditional-request metadata.
    async fn list_notifications(
        &self,
        options: &NotificationListOptions,
    ) -> Result<NotificationPage, ProviderError>;

    /// Clears unread on a notification thread (Mark read).
    async fn mark_notification_read(&self, id: &NotificationThreadId) -> Result<(), ProviderError>;

    /// Removes a notification thread from the Inbox (Done).
    async fn mark_notification_done(&self, id: &NotificationThreadId) -> Result<(), ProviderError>;

    async fn fetch_pull_request_threads(
        &self,
        id: &PullRequestId,
    ) -> Result<PullRequestThreads, ProviderError>;

    async fn add_pending_diff_review_comment(
        &self,
        id: &PullRequestId,
        comment: PendingDiffReviewComment,
    ) -> Result<PullRequestThreads, ProviderError>;

    async fn submit_pending_review(
        &self,
        id: &PullRequestId,
        event: ReviewSubmitEvent,
        summary_body: Option<String>,
    ) -> Result<PullRequestThreads, ProviderError>;

    async fn discard_pending_review(
        &self,
        id: &PullRequestId,
    ) -> Result<PullRequestThreads, ProviderError>;

    async fn set_pull_request_labels(
        &self,
        id: &PullRequestId,
        labels: &[String],
    ) -> Result<(), ProviderError> {
        let _ = (id, labels);
        Err(ProviderError::other("labels write not supported"))
    }

    async fn set_pull_request_assignees(
        &self,
        id: &PullRequestId,
        assignees: &[String],
    ) -> Result<(), ProviderError> {
        let _ = (id, assignees);
        Err(ProviderError::other("assignees write not supported"))
    }

    async fn request_pull_request_reviewer(
        &self,
        id: &PullRequestId,
        login: &str,
    ) -> Result<(), ProviderError> {
        let _ = (id, login);
        Err(ProviderError::other("reviewer request not supported"))
    }

    async fn remove_pull_request_reviewer(
        &self,
        id: &PullRequestId,
        login: &str,
    ) -> Result<(), ProviderError> {
        let _ = (id, login);
        Err(ProviderError::other("reviewer remove not supported"))
    }

    async fn rerequest_pull_request_reviewer(
        &self,
        id: &PullRequestId,
        login: &str,
    ) -> Result<(), ProviderError> {
        let _ = (id, login);
        Err(ProviderError::other("reviewer re-request not supported"))
    }

    async fn set_pull_request_draft(
        &self,
        id: &PullRequestId,
        is_draft: bool,
    ) -> Result<(), ProviderError> {
        let _ = (id, is_draft);
        Err(ProviderError::other("draft toggle not supported"))
    }

    async fn set_pull_request_base(
        &self,
        id: &PullRequestId,
        base_ref: &str,
    ) -> Result<(), ProviderError> {
        let _ = (id, base_ref);
        Err(ProviderError::other("base branch write not supported"))
    }

    async fn fetch_pipelines(&self, id: &PullRequestId)
    -> Result<PipelinesSnapshot, ProviderError>;

    async fn rerun_check(
        &self,
        id: &PullRequestId,
        target: &RerunTarget,
    ) -> Result<(), ProviderError>;
}
