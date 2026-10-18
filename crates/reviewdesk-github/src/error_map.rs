use reviewdesk_core::ProviderError;

/// Map an HTTP status to a shared Provider error.
pub fn from_status(status: u16, body: &str) -> ProviderError {
    match status {
        401 | 403 if body.to_lowercase().contains("rate limit") => ProviderError::RateLimit,
        401 | 403 => ProviderError::Auth,
        404 => ProviderError::NotFound,
        429 => ProviderError::RateLimit,
        _ => ProviderError::other(format!("HTTP {status}: {body}")),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unauthorized_is_auth() {
        assert_eq!(from_status(401, "Bad credentials"), ProviderError::Auth);
    }

    #[test]
    fn forbidden_without_rate_limit_is_auth() {
        assert_eq!(
            from_status(403, "Resource not accessible"),
            ProviderError::Auth
        );
    }

    #[test]
    fn forbidden_rate_limit_message_is_rate_limit() {
        assert_eq!(
            from_status(403, "API rate limit exceeded"),
            ProviderError::RateLimit
        );
    }

    #[test]
    fn not_found_maps() {
        assert_eq!(from_status(404, "Not Found"), ProviderError::NotFound);
    }

    #[test]
    fn too_many_requests_is_rate_limit() {
        assert_eq!(from_status(429, ""), ProviderError::RateLimit);
    }

    #[test]
    fn other_status_keeps_message() {
        assert_eq!(
            from_status(500, "boom"),
            ProviderError::Other("HTTP 500: boom".into())
        );
    }
}
