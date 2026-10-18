//! PROTOTYPE (throwaway): Conversation + Diff-anchored Threads for Review Desk.
//!
//! Question: how should Threads read and compose on Conversation and the Diff
//! surface — concrete enough to grill remaining UX, without live Provider writeback?
//!
//! Baseline: clone Linear's issue comments
//! ([comment-on-issues](https://linear.app/docs/comment-on-issues)) for Review Desk
//! PRs. Sibling prototype `prototype/pull-requests-list-nav` locks the list chrome
//! to Linear Issues; this branch locks Conversation/Threads to Linear comments.
//!
//! Mapping (Linear → Review Desk):
//! - Issue comment feed → Conversation chronological comments + Thread roots
//! - Reply-to-comment threads → Thread replies (shared with Diff)
//! - Resolve thread (root or showcase reply) → Thread resolve
//! - “Leave a comment…” bottom composer → Conversation compose (timeline posts
//!   immediately; Diff-anchored drafts sit on Pending review per #54)
//! - Unsent / Drafts → Pending review drafts (visible with Draft badge)
//! - Inline comments on description → Diff-anchored Review comments / Suggestions
//!
//! Domain from #54: Review comment / Thread / Suggestion / Pending review.
//! Annotation is out of product language. Iterate in place — no A/B/C switcher.
//!
//! Run: `cargo run -p reviewdesk`
//! Branch: `prototype/conversation-diff-threads` (never merge as-is)

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ThreadStatus {
    Open,
    Resolved,
    Outdated,
}

#[derive(Debug, Clone, Copy)]
pub struct StubComment {
    pub author: &'static str,
    pub body: &'static str,
    pub relative_time: &'static str,
    pub pending: bool,
    pub suggestion: Option<&'static str>,
    /// When set, resolving from this reply “showcases” it (Linear).
    pub showcase_resolve: bool,
}

#[derive(Debug, Clone, Copy)]
pub struct StubThread {
    pub id: &'static str,
    /// Diff anchor; `None` = timeline-only Conversation comment (Linear top-level).
    pub path: Option<&'static str>,
    pub line_label: Option<&'static str>,
    pub status: ThreadStatus,
    pub comments: &'static [StubComment],
}

/// Shared stub Thread set for Conversation + Diff (no Provider).
pub const STUB_THREADS: &[StubThread] = &[
    StubThread {
        id: "t-timeline",
        path: None,
        line_label: None,
        status: ThreadStatus::Open,
        comments: &[StubComment {
            author: "carol",
            body: "Looks good overall — shipping the shell pick next.",
            relative_time: "2h",
            pending: false,
            suggestion: None,
            showcase_resolve: false,
        }],
    },
    StubThread {
        id: "t1",
        path: Some("crates/reviewdesk/src/chrome_prototype.rs"),
        line_label: Some("L42–L48"),
        status: ThreadStatus::Open,
        comments: &[
            StubComment {
                author: "alice",
                body: "Can we keep Dock Dense padding on the sidebar list?",
                relative_time: "1h",
                pending: false,
                suggestion: None,
                showcase_resolve: false,
            },
            StubComment {
                author: "you",
                body: "Yes — chrome A locked that density for the pick.",
                relative_time: "45m",
                pending: false,
                suggestion: None,
                showcase_resolve: false,
            },
        ],
    },
    StubThread {
        id: "t2",
        path: Some("crates/reviewdesk/src/ui/shell_prototype.rs"),
        line_label: Some("L120"),
        status: ThreadStatus::Open,
        comments: &[
            StubComment {
                author: "bob",
                body: "Metadata should stay open by default only on Overview.",
                relative_time: "40m",
                pending: false,
                suggestion: None,
                showcase_resolve: false,
            },
            StubComment {
                author: "you",
                body: "Agree — draft reply sits on Pending review until Submit.",
                relative_time: "Draft",
                pending: true,
                suggestion: None,
                showcase_resolve: false,
            },
        ],
    },
    StubThread {
        id: "t3",
        path: Some("crates/reviewdesk/src/app.rs"),
        line_label: Some("L88–L92"),
        status: ThreadStatus::Open,
        comments: &[StubComment {
            author: "you",
            body: "Rename the Annotation label while we are here.",
            relative_time: "Draft",
            pending: true,
            suggestion: Some(
                "window.set_window_title(\"Review Desk\");\n// Pending review drafts, not Annotations",
            ),
            showcase_resolve: false,
        }],
    },
    StubThread {
        id: "t4",
        path: Some("docs/agents/domain.md"),
        line_label: Some("file"),
        status: ThreadStatus::Resolved,
        comments: &[
            StubComment {
                author: "carol",
                body: "CODEOWNERS breakdown belongs on Overview, not Metadata.",
                relative_time: "1d",
                pending: false,
                suggestion: None,
                showcase_resolve: false,
            },
            StubComment {
                author: "you",
                body: "Resolved after #53 — this reply is the showcase.",
                relative_time: "1d",
                pending: false,
                suggestion: None,
                showcase_resolve: true,
            },
        ],
    },
    StubThread {
        id: "t5",
        path: Some("crates/reviewdesk/src/paths.rs"),
        line_label: Some("L12 · outdated"),
        status: ThreadStatus::Outdated,
        comments: &[StubComment {
            author: "dave",
            body: "This path helper moved — Thread is outdated but still replyable.",
            relative_time: "3d",
            pending: false,
            suggestion: None,
            showcase_resolve: false,
        }],
    },
];

pub const PENDING_DRAFT_COUNT: usize = 2;

pub fn thread_by_id(id: &str) -> Option<&'static StubThread> {
    STUB_THREADS.iter().find(|t| t.id == id)
}

pub fn conversation_threads(hide_resolved: bool) -> Vec<&'static StubThread> {
    STUB_THREADS
        .iter()
        .filter(|t| {
            if hide_resolved && matches!(t.status, ThreadStatus::Resolved) {
                return false;
            }
            true
        })
        .collect()
}

pub fn default_reply_thread_id() -> &'static str {
    "t2"
}
