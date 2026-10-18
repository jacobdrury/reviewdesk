use async_trait::async_trait;

use crate::error::ProviderError;
use crate::id::PullRequestId;
use crate::notification::{NotificationListOptions, NotificationPage, NotificationThreadId};
use crate::patch::Patch;
use crate::pull_request::PullRequestDetail;
use crate::pull_request_list::{PullRequestListItem, PullRequestListOptions};
use crate::review_threads::{
    PendingDiffReviewComment, PullRequestThreads, ReviewSubmitEvent, ThreadId, ThreadStatus,
    TimelineConversationComment,
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

    /// Resolve or reopen a Thread when the Provider allows.
    async fn set_thread_status(
        &self,
        id: &PullRequestId,
        thread_id: &ThreadId,
        status: ThreadStatus,
    ) -> Result<PullRequestThreads, ProviderError>;

    /// Publish a timeline Conversation comment immediately (not Pending review).
    async fn publish_timeline_comment(
        &self,
        id: &PullRequestId,
        comment: TimelineConversationComment,
    ) -> Result<PullRequestThreads, ProviderError>;
}
