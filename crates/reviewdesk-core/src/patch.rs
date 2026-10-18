use std::fmt;

/// Unified-diff text suitable for `DiffFile::parse`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Patch(String);

impl Patch {
    pub fn new(text: impl Into<String>) -> Self {
        Self(text.into())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }

    pub fn into_inner(self) -> String {
        self.0
    }
}

impl fmt::Display for Patch {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl AsRef<str> for Patch {
    fn as_ref(&self) -> &str {
        &self.0
    }
}
