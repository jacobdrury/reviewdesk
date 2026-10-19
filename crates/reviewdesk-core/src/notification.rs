use std::fmt;
use std::time::Duration;

use chrono::{DateTime, Utc};
use thiserror::Error;

use crate::id::PullRequestId;

/// Opaque Provider notification-thread identity.
///
/// The string form is Provider-owned (GitHub `thread_id`). Core does not interpret it.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct NotificationThreadId(String);

#[derive(Debug, Error, PartialEq, Eq)]
pub enum NotificationThreadIdError {
    #[error("notification thread id must not be empty")]
    Empty,
}

impl NotificationThreadId {
    pub fn new(raw: impl Into<String>) -> Result<Self, NotificationThreadIdError> {
        let raw = raw.into();
        if raw.is_empty() {
            return Err(NotificationThreadIdError::Empty);
        }
        Ok(Self(raw))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for NotificationThreadId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

/// Allowlisted notification reasons the Inbox surfaces. Other Provider reasons
/// (e.g. `subscribed`, `assign`, `approval_requested`) are filtered out upstream.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NotificationReason {
    ReviewRequested,
    Mention,
    TeamMention,
    Comment,
    Author,
    CiActivity,
    StateChange,
}

impl NotificationReason {
    /// Parses a Provider reason string, returning `None` when it is not on the
    /// Inbox allowlist.
    pub fn parse(raw: &str) -> Option<Self> {
        match raw {
            "review_requested" => Some(Self::ReviewRequested),
            "mention" => Some(Self::Mention),
            "team_mention" => Some(Self::TeamMention),
            "comment" => Some(Self::Comment),
            "author" => Some(Self::Author),
            "ci_activity" => Some(Self::CiActivity),
            "state_change" => Some(Self::StateChange),
            _ => None,
        }
    }

    /// Stable storage/display token (matches the Provider reason string).
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::ReviewRequested => "review_requested",
            Self::Mention => "mention",
            Self::TeamMention => "team_mention",
            Self::Comment => "comment",
            Self::Author => "author",
            Self::CiActivity => "ci_activity",
            Self::StateChange => "state_change",
        }
    }

    /// Short human label for Inbox row chrome.
    pub fn label(&self) -> &'static str {
        match self {
            Self::ReviewRequested => "Review requested",
            Self::Mention => "Mention",
            Self::TeamMention => "Team mention",
            Self::Comment => "Comment",
            Self::Author => "Author",
            Self::CiActivity => "CI",
            Self::StateChange => "State change",
        }
    }
}

/// One Inbox row: a Provider notification thread whose subject is a pull request.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NotificationThread {
    pub id: NotificationThreadId,
    pub pull_request_id: PullRequestId,
    pub title: String,
    pub repo_label: String,
    pub reason: NotificationReason,
    pub unread: bool,
    pub updated_at: DateTime<Utc>,
}

/// Options for a notification list poll. Defaults to unread-only, no conditional header.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct NotificationListOptions {
    /// Include read threads when `true`; Inbox default is unread-only.
    pub all: bool,
    /// Value for the conditional `If-Modified-Since` request header.
    pub if_modified_since: Option<String>,
}

/// Result of one notification poll, including polite-polling metadata.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NotificationPage {
    pub threads: Vec<NotificationThread>,
    /// Provider-suggested minimum delay before the next poll (`X-Poll-Interval`).
    pub poll_interval: Option<Duration>,
    /// `Last-Modified` to echo back as `If-Modified-Since` on the next poll.
    pub last_modified: Option<String>,
    /// `true` when the Provider answered `304 Not Modified`; `threads` is empty.
    pub not_modified: bool,
}

/// Query shape for reading Inbox rows from LocalStore.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NotificationQuery {
    pub unread_only: bool,
}

impl Default for NotificationQuery {
    fn default() -> Self {
        Self { unread_only: true }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_accepts_allowlisted_reasons() {
        assert_eq!(
            NotificationReason::parse("review_requested"),
            Some(NotificationReason::ReviewRequested)
        );
        assert_eq!(
            NotificationReason::parse("ci_activity"),
            Some(NotificationReason::CiActivity)
        );
    }

    #[test]
    fn parse_rejects_unlisted_reasons() {
        assert_eq!(NotificationReason::parse("subscribed"), None);
        assert_eq!(NotificationReason::parse("assign"), None);
        assert_eq!(NotificationReason::parse(""), None);
    }

    #[test]
    fn reason_token_round_trips() {
        for reason in [
            NotificationReason::ReviewRequested,
            NotificationReason::Mention,
            NotificationReason::TeamMention,
            NotificationReason::Comment,
            NotificationReason::Author,
            NotificationReason::CiActivity,
            NotificationReason::StateChange,
        ] {
            assert_eq!(NotificationReason::parse(reason.as_str()), Some(reason));
        }
    }

    #[test]
    fn empty_thread_id_is_rejected() {
        assert_eq!(
            NotificationThreadId::new(""),
            Err(NotificationThreadIdError::Empty)
        );
    }

    #[test]
    fn default_query_is_unread_only() {
        assert!(NotificationQuery::default().unread_only);
    }
}
