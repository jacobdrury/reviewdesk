use async_trait::async_trait;

use crate::error::ProviderError;
use crate::id::PullRequestId;
use crate::patch::Patch;
use crate::pull_request::PullRequestDetail;
use crate::pull_request_list::{PullRequestListItem, PullRequestListOptions};
use crate::review_threads::{
    PendingDiffReviewComment, PullRequestThreads, ReviewSubmitEvent,
};

/// Host that exposes pull requests and their patches.
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
}
