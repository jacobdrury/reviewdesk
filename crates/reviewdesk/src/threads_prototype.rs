//! PROTOTYPE (throwaway): Conversation + Diff-anchored Threads for Review Desk.
//!
//! Question: how should Threads read and compose on Conversation and the Diff
//! surface (shared Thread set), including a Pending review affordance and a
//! Suggestion stub — concrete enough to grill remaining UX, without live
//! Provider writeback?
//!
//! Domain from #54: Review comment / Thread / Suggestion / Pending review;
//! Annotation is out of product language. One Thread set shared by Conversation
//! and Diff. Suggestions are compose/display only.
//!
//! Shell chrome is locked to the #58 pick (Linear + right Metadata dock).
//! Three Thread layout variants, cycled with the bottom bar (or `[` / `]`):
//! - A (Inline): Conversation is a chronological feed; Diff shows full inline
//!   Thread cards (incl. Suggestion + reply compose stubs).
//! - B (Split rail): Conversation is Thread index + detail; Diff keeps compact
//!   markers and a right Thread rail.
//! - C (Inspector): Conversation is a filterable index; Diff markers open a
//!   shared bottom inspector (Suggestion as proposed vs current).
//!
//! Run: `cargo run -p reviewdesk`
//! Branch: `prototype/conversation-diff-threads` (never merge as-is)

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ThreadVariant {
    #[default]
    A,
    B,
    C,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ThreadFilter {
    All,
    Open,
    Resolved,
    Outdated,
    Pending,
}

impl ThreadVariant {
    pub fn key(self) -> &'static str {
        match self {
            Self::A => "A",
            Self::B => "B",
            Self::C => "C",
        }
    }

    pub fn name(self) -> &'static str {
        match self {
            Self::A => "Inline feed + Diff cards",
            Self::B => "Split index + Diff rail",
            Self::C => "Filter index + inspector",
        }
    }

    pub fn label(self) -> String {
        format!("{} ({})", self.key(), self.name())
    }

    pub fn next(self) -> Self {
        match self {
            Self::A => Self::B,
            Self::B => Self::C,
            Self::C => Self::A,
        }
    }

    pub fn prev(self) -> Self {
        match self {
            Self::A => Self::C,
            Self::B => Self::A,
            Self::C => Self::B,
        }
    }

    /// Diff paints full inline Thread cards.
    pub fn inline_diff_cards(self) -> bool {
        matches!(self, Self::A)
    }
}

impl ThreadFilter {
    pub fn label(self) -> &'static str {
        match self {
            Self::All => "All",
            Self::Open => "Open",
            Self::Resolved => "Resolved",
            Self::Outdated => "Outdated",
            Self::Pending => "Pending",
        }
    }

    pub fn all() -> &'static [Self] {
        &[
            Self::All,
            Self::Open,
            Self::Resolved,
            Self::Outdated,
            Self::Pending,
        ]
    }
}

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
    pub pending: bool,
    pub suggestion: Option<&'static str>,
}

#[derive(Debug, Clone, Copy)]
pub struct StubThread {
    pub id: &'static str,
    pub path: &'static str,
    pub line_label: &'static str,
    pub status: ThreadStatus,
    pub pending_draft: bool,
    pub comments: &'static [StubComment],
}

#[derive(Debug, Clone, Copy)]
pub struct StubTimeline {
    pub author: &'static str,
    pub body: &'static str,
}

/// Shared stub Thread set for Conversation + Diff (no Provider).
pub const STUB_THREADS: &[StubThread] = &[
    StubThread {
        id: "t1",
        path: "crates/reviewdesk/src/chrome_prototype.rs",
        line_label: "L42–L48 · RIGHT",
        status: ThreadStatus::Open,
        pending_draft: false,
        comments: &[
            StubComment {
                author: "alice",
                body: "Can we keep Dock Dense padding on the sidebar list?",
                pending: false,
                suggestion: None,
            },
            StubComment {
                author: "you",
                body: "Yes — chrome A locked that density for the pick.",
                pending: false,
                suggestion: None,
            },
        ],
    },
    StubThread {
        id: "t2",
        path: "crates/reviewdesk/src/ui/shell_prototype.rs",
        line_label: "L120 · RIGHT",
        status: ThreadStatus::Open,
        pending_draft: true,
        comments: &[
            StubComment {
                author: "bob",
                body: "Metadata should stay open by default only on Overview.",
                pending: false,
                suggestion: None,
            },
            StubComment {
                author: "you",
                body: "Agree — draft reply on Pending review until Submit.",
                pending: true,
                suggestion: None,
            },
        ],
    },
    StubThread {
        id: "t3",
        path: "crates/reviewdesk/src/app.rs",
        line_label: "L88–L92 · RIGHT",
        status: ThreadStatus::Open,
        pending_draft: true,
        comments: &[StubComment {
            author: "you",
            body: "Rename the Annotation label while we are here.",
            pending: true,
            suggestion: Some(
                "window.set_window_title(\"Review Desk\");\n// Pending review drafts, not Annotations",
            ),
        }],
    },
    StubThread {
        id: "t4",
        path: "docs/agents/domain.md",
        line_label: "file",
        status: ThreadStatus::Resolved,
        pending_draft: false,
        comments: &[
            StubComment {
                author: "carol",
                body: "CODEOWNERS breakdown belongs on Overview, not Metadata.",
                pending: false,
                suggestion: None,
            },
            StubComment {
                author: "you",
                body: "Resolved after #53.",
                pending: false,
                suggestion: None,
            },
        ],
    },
    StubThread {
        id: "t5",
        path: "crates/reviewdesk/src/paths.rs",
        line_label: "L12 · LEFT (outdated)",
        status: ThreadStatus::Outdated,
        pending_draft: false,
        comments: &[StubComment {
            author: "dave",
            body: "This path helper moved — Thread is outdated but still replyable.",
            pending: false,
            suggestion: None,
        }],
    },
];

pub const STUB_TIMELINE: &[StubTimeline] = &[StubTimeline {
    author: "carol",
    body: "Looks good overall — non-diff Conversation comment (publishes immediately).",
}];

pub const PENDING_REVIEW_SUMMARY: &str =
    "Pending review · 2 drafts (1 reply, 1 Suggestion) · not submitted";

pub fn thread_by_id(id: &str) -> Option<&'static StubThread> {
    STUB_THREADS.iter().find(|t| t.id == id)
}

pub fn filtered_threads(filter: ThreadFilter) -> Vec<&'static StubThread> {
    STUB_THREADS
        .iter()
        .filter(|t| match filter {
            ThreadFilter::All => true,
            ThreadFilter::Open => matches!(t.status, ThreadStatus::Open),
            ThreadFilter::Resolved => matches!(t.status, ThreadStatus::Resolved),
            ThreadFilter::Outdated => matches!(t.status, ThreadStatus::Outdated),
            ThreadFilter::Pending => t.pending_draft,
        })
        .collect()
}

pub fn default_selected_thread_id() -> &'static str {
    "t2"
}
