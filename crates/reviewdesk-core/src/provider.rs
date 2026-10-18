use async_trait::async_trait;

use crate::error::ProviderError;
use crate::id::PullRequestId;
use crate::patch::Patch;
use crate::pull_request::PullRequestDetail;
use crate::pull_request_list::{PullRequestListItem, PullRequestListOptions};

/// Host that exposes pull requests and their patches.
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
}
