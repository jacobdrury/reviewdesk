use std::fmt;

/// Provider credentials. The app resolves the token; core never shells out.
#[derive(Clone)]
pub struct Credentials {
    token: String,
}

impl Credentials {
    pub fn new(token: impl Into<String>) -> Self {
        Self {
            token: token.into(),
        }
    }

    pub fn token(&self) -> &str {
        &self.token
    }
}

impl fmt::Debug for Credentials {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Credentials")
            .field("token", &"[redacted]")
            .finish()
    }
}
