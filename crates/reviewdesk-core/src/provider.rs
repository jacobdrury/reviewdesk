use async_trait::async_trait;

use crate::error::ProviderError;
use crate::id::PullRequestId;
use crate::patch::Patch;
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
}
