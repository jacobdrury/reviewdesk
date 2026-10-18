use thiserror::Error;

/// Shared Provider errors. No raw HTTP types cross the crate boundary.
#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum ProviderError {
    #[error("authentication failed")]
    Auth,
    #[error("rate limited")]
    RateLimit,
    #[error("not found")]
    NotFound,
    #[error("{0}")]
    Other(String),
}

impl ProviderError {
    pub fn other(message: impl Into<String>) -> Self {
        Self::Other(message.into())
    }
}
