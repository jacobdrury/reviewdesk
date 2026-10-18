use std::fmt;

use serde::{Deserialize, Serialize};
use thiserror::Error;

/// Opaque pull-request identity across Providers.
///
/// The string form is Provider-owned. Core does not interpret it.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct PullRequestId(String);

#[derive(Debug, Error, PartialEq, Eq)]
pub enum PullRequestIdError {
    #[error("pull request id must not be empty")]
    Empty,
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
    fn accepts_non_empty_id() {
        let id = PullRequestId::new("opaque").unwrap();
        assert_eq!(id.as_str(), "opaque");
    }

    #[test]
    fn empty_id_is_rejected() {
        assert_eq!(PullRequestId::new(""), Err(PullRequestIdError::Empty));
    }
}
