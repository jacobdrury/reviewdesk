use async_trait::async_trait;

use crate::error::ProviderError;
use crate::id::PullRequestId;
use crate::notification::{NotificationListOptions, NotificationPage, NotificationThreadId};
use crate::patch::Patch;
use crate::pipelines::{PipelinesSnapshot, RerunTarget};
use crate::pull_request::PullRequestDetail;
use crate::pull_request_list::{PullRequestListItem, PullRequestListOptions};

/// Host that exposes pull requests, their patches, and the notification Inbox.
#[async_trait]
pub trait Provider: Send + Sync {
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

    async fn fetch_pipelines(&self, id: &PullRequestId)
    -> Result<PipelinesSnapshot, ProviderError>;

    async fn rerun_check(
        &self,
        id: &PullRequestId,
        target: &RerunTarget,
    ) -> Result<(), ProviderError>;
}
