use serde::{Deserialize, Serialize};

use crate::id::PullRequestId;

/// Side of the unified diff for a line anchor.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum DiffLineSide {
    Left,
    Right,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DiffAnchor {
    pub path: String,
    pub side: DiffLineSide,
    pub start_line: u32,
    pub end_line: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ThreadId(pub String);

impl ThreadId {
    pub fn new(raw: impl Into<String>) -> Self {
        Self(raw.into())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReviewCommentId(pub String);

impl ReviewCommentId {
    pub fn new(raw: impl Into<String>) -> Self {
        Self(raw.into())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ThreadStatus {
    Open,
    Resolved,
    Outdated,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReviewComment {
    pub id: ReviewCommentId,
    pub author_login: String,
    pub body: String,
    pub is_pending: bool,
    /// Replacement text from a suggestion block, if present.
    pub suggestion: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Thread {
    pub id: ThreadId,
    pub anchor: Option<DiffAnchor>,
    pub status: ThreadStatus,
    pub comments: Vec<ReviewComment>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PendingReview {
    pub pull_request_id: PullRequestId,
    pub provider_review_id: Option<String>,
    pub summary_body: Option<String>,
}

/// Shared Thread set for a pull request (Conversation + Diff surfaces).
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct PullRequestThreads {
    pub threads: Vec<Thread>,
    pub pending_review: Option<PendingReview>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ReviewSubmitEvent {
    Comment,
    Approve,
    RequestChanges,
}

/// Timeline Conversation remark (no diff anchor; publishes immediately).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TimelineConversationComment {
    pub body: String,
}

/// New diff-anchored remark on the viewer's Pending review.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PendingDiffReviewComment {
    pub reply_to_thread: Option<ThreadId>,
    pub anchor: DiffAnchor,
    pub body: String,
}

/// Extracts suggestion replacement text from a Review comment body, if fenced.
pub fn suggestion_from_body(body: &str) -> Option<String> {
    const OPEN: &str = "```suggestion";
    let start = body.find(OPEN)?;
    let after_open = &body[start + OPEN.len()..];
    let content_start = after_open.find('\n')? + 1;
    let rest = &after_open[content_start..];
    let close = rest.find("```")?;
    Some(rest[..close].trim_end().to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn suggestion_from_body_parses_fenced_block() {
        let body = "Please fix:\n```suggestion\nlet x = 1;\n```\n";
        assert_eq!(suggestion_from_body(body), Some("let x = 1;".to_string()));
    }

    #[test]
    fn suggestion_from_body_absent_when_no_fence() {
        assert_eq!(suggestion_from_body("plain comment"), None);
    }
}
