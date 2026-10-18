use reviewdesk_core::{ProviderError, PullRequestId};
use thiserror::Error;

#[derive(Debug, Error, PartialEq, Eq)]
pub enum GitHubIdError {
    #[error("invalid GitHub pull request id: {0}")]
    Invalid(String),
}

impl From<GitHubIdError> for ProviderError {
    fn from(value: GitHubIdError) -> Self {
        ProviderError::other(value.to_string())
    }
}

/// Format a GitHub pull-request id as `owner/repo#number`.
pub fn pull_request_id(
    owner: &str,
    repo: &str,
    number: u64,
) -> Result<PullRequestId, GitHubIdError> {
    if owner.is_empty() || repo.is_empty() || number == 0 {
        return Err(GitHubIdError::Invalid(format!("{owner}/{repo}#{number}")));
    }
    PullRequestId::new(format!("{owner}/{repo}#{number}"))
        .map_err(|_| GitHubIdError::Invalid(format!("{owner}/{repo}#{number}")))
}

/// Parse a GitHub `owner/repo#number` id.
pub fn parse_pull_request_id(id: &PullRequestId) -> Result<(String, String, u64), GitHubIdError> {
    let raw = id.as_str();
    let Some((repo_path, number_str)) = raw.rsplit_once('#') else {
        return Err(GitHubIdError::Invalid(raw.to_string()));
    };
    let Some((owner, repo)) = repo_path.split_once('/') else {
        return Err(GitHubIdError::Invalid(raw.to_string()));
    };
    if owner.is_empty() || repo.is_empty() || repo.contains('/') {
        return Err(GitHubIdError::Invalid(raw.to_string()));
    }
    let Ok(number) = number_str.parse::<u64>() else {
        return Err(GitHubIdError::Invalid(raw.to_string()));
    };
    if number == 0 {
        return Err(GitHubIdError::Invalid(raw.to_string()));
    }
    Ok((owner.to_string(), repo.to_string(), number))
}

#[cfg(test)]
mod tests {
    use super::*;
    use reviewdesk_core::PullRequestId;

    #[test]
    fn formats_owner_repo_number() {
        let id = pull_request_id("acme", "widgets", 42).unwrap();
        assert_eq!(id.as_str(), "acme/widgets#42");
    }

    #[test]
    fn parse_round_trips() {
        let id = pull_request_id("acme", "widgets", 7).unwrap();
        let (owner, repo, number) = parse_pull_request_id(&id).unwrap();
        assert_eq!(owner, "acme");
        assert_eq!(repo, "widgets");
        assert_eq!(number, 7);
    }

    #[test]
    fn parse_rejects_missing_number() {
        let id = PullRequestId::new("acme/widgets").unwrap();
        assert!(matches!(
            parse_pull_request_id(&id),
            Err(GitHubIdError::Invalid(_))
        ));
    }

    #[test]
    fn rejects_zero_number() {
        assert!(pull_request_id("acme", "widgets", 0).is_err());
    }
}
