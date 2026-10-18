use std::fmt;

use thiserror::Error;

/// Opaque pull-request identity across Providers.
///
/// GitHub encodes this as `owner/repo#number`. Callers must not assume a format
/// beyond what the active Provider documents.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct PullRequestId(String);

#[derive(Debug, Error, PartialEq, Eq)]
pub enum PullRequestIdError {
    #[error("pull request id must not be empty")]
    Empty,
    #[error("invalid GitHub pull request id: {0}")]
    InvalidGitHub(String),
}

impl PullRequestId {
    pub fn new(raw: impl Into<String>) -> Result<Self, PullRequestIdError> {
        let raw = raw.into();
        if raw.is_empty() {
            return Err(PullRequestIdError::Empty);
        }
        Ok(Self(raw))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// Format a GitHub pull-request id as `owner/repo#number`.
    pub fn github(owner: &str, repo: &str, number: u64) -> Result<Self, PullRequestIdError> {
        if owner.is_empty() || repo.is_empty() || number == 0 {
            return Err(PullRequestIdError::InvalidGitHub(format!(
                "{owner}/{repo}#{number}"
            )));
        }
        Self::new(format!("{owner}/{repo}#{number}"))
    }

    /// Parse a GitHub `owner/repo#number` id.
    pub fn parse_github(&self) -> Result<(String, String, u64), PullRequestIdError> {
        let Some((repo_path, number_str)) = self.0.rsplit_once('#') else {
            return Err(PullRequestIdError::InvalidGitHub(self.0.clone()));
        };
        let Some((owner, repo)) = repo_path.split_once('/') else {
            return Err(PullRequestIdError::InvalidGitHub(self.0.clone()));
        };
        if owner.is_empty() || repo.is_empty() || repo.contains('/') {
            return Err(PullRequestIdError::InvalidGitHub(self.0.clone()));
        }
        let Ok(number) = number_str.parse::<u64>() else {
            return Err(PullRequestIdError::InvalidGitHub(self.0.clone()));
        };
        if number == 0 {
            return Err(PullRequestIdError::InvalidGitHub(self.0.clone()));
        }
        Ok((owner.to_string(), repo.to_string(), number))
    }
}

impl fmt::Display for PullRequestId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn github_formats_owner_repo_number() {
        let id = PullRequestId::github("acme", "widgets", 42).unwrap();
        assert_eq!(id.as_str(), "acme/widgets#42");
    }

    #[test]
    fn parse_github_round_trips() {
        let id = PullRequestId::github("acme", "widgets", 7).unwrap();
        let (owner, repo, number) = id.parse_github().unwrap();
        assert_eq!(owner, "acme");
        assert_eq!(repo, "widgets");
        assert_eq!(number, 7);
    }

    #[test]
    fn parse_github_rejects_missing_number() {
        let id = PullRequestId::new("acme/widgets").unwrap();
        assert!(matches!(
            id.parse_github(),
            Err(PullRequestIdError::InvalidGitHub(_))
        ));
    }

    #[test]
    fn empty_id_is_rejected() {
        assert_eq!(PullRequestId::new(""), Err(PullRequestIdError::Empty));
    }

    #[test]
    fn github_rejects_zero_number() {
        assert!(PullRequestId::github("acme", "widgets", 0).is_err());
    }
}
