use async_trait::async_trait;

use crate::error::ProviderError;
use crate::id::PullRequestId;
use crate::inbox::{InboxItem, InboxOptions};
use crate::patch::Patch;
use crate::pull_request::PullRequestDetail;

/// Host that exposes pull requests and their patches.
#[async_trait]
pub trait Provider: Send + Sync {
    async fn list_inbox(&self, options: &InboxOptions) -> Result<Vec<InboxItem>, ProviderError>;

    async fn fetch_patch(&self, id: &PullRequestId) -> Result<Patch, ProviderError>;

    async fn fetch_pull_request(
        &self,
        id: &PullRequestId,
    ) -> Result<PullRequestDetail, ProviderError>;
}
