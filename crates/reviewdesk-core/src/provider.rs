use async_trait::async_trait;

use crate::error::ProviderError;
use crate::id::PullRequestId;
use crate::patch::Patch;
use crate::pipelines::{PipelinesSnapshot, RerunTarget};
use crate::pull_request::PullRequestDetail;
use crate::pull_request_list::{PullRequestListItem, PullRequestListOptions};

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

    async fn fetch_pipelines(&self, id: &PullRequestId)
    -> Result<PipelinesSnapshot, ProviderError>;

    async fn rerun_check(
        &self,
        id: &PullRequestId,
        target: &RerunTarget,
    ) -> Result<(), ProviderError>;
}
