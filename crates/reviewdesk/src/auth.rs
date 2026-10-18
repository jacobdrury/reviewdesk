use std::process::Command;

use reviewdesk_core::{Credentials, ProviderError};

/// Resolve a GitHub token from the environment or `gh auth token`.
pub fn resolve_credentials() -> Result<Credentials, ProviderError> {
    if let Ok(token) = std::env::var("GITHUB_TOKEN") {
        let token = token.trim();
        if !token.is_empty() {
            return Ok(Credentials::new(token));
        }
    }
    if let Ok(token) = std::env::var("GH_TOKEN") {
        let token = token.trim();
        if !token.is_empty() {
            return Ok(Credentials::new(token));
        }
    }
    gh_auth_token()
}

fn gh_auth_token() -> Result<Credentials, ProviderError> {
    let output = Command::new("gh")
        .args(["auth", "token"])
        .output()
        .map_err(|e| ProviderError::other(format!("failed to run gh auth token: {e}")))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(ProviderError::other(format!(
            "gh auth token failed: {}",
            stderr.trim()
        )));
    }

    let token = String::from_utf8_lossy(&output.stdout);
    let token = token.trim();
    if token.is_empty() {
        return Err(ProviderError::Auth);
    }
    Ok(Credentials::new(token))
}
