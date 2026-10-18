//! PROTOTYPE (throwaway): Conversation + Diff-anchored Threads for Review Desk.
//!
//! HITL pick (locked): Linear comments baseline for Conversation + Diff Threads.
//! See [comment-on-issues](https://linear.app/docs/comment-on-issues). Sibling
//! `prototype/pull-requests-list-nav` locks list chrome to Linear Issues.
//!
//! Locked display:
//! - Chronological Conversation feed; Diff-anchored Threads show path + code snippet
//! - Thread cards: surface ladder (lifted feed → hairline card); nested replies
//!   indent + left rail only (no box-in-a-box)
//! - Bottom “Leave a comment…” composer (⌘ Enter); Reply opens under the Thread
//! - Pending review drafts as Draft badges + Submit/Discard chip
//! - Suggestion compose/display only; Resolve / showcase reply affordances
//! - High-contrast CTAs (avoid theme `.primary()` light-on-light)
//!
//! Domain from #54: Review comment / Thread / Suggestion / Pending review.
//! Annotation is out of product language.
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
pub struct CodeLine {
    pub number: u32,
    pub text: &'static str,
    /// Anchored / called-out line (highlighted in the snippet).
    pub highlighted: bool,
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
    /// Anchored Patch lines shown above Diff-anchored comments in Conversation.
    pub code_context: Option<&'static [CodeLine]>,
    pub status: ThreadStatus,
    pub comments: &'static [StubComment],
}

/// Shared stub Thread set for Conversation + Diff (no Provider).
pub const STUB_THREADS: &[StubThread] = &[
    StubThread {
        id: "t-timeline",
        path: None,
        line_label: None,
        code_context: None,
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
        line_label: Some("L16–L21"),
        code_context: Some(&[
            CodeLine {
                number: 14,
                text: "",
                highlighted: false,
            },
            CodeLine {
                number: 15,
                text: "#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]",
                highlighted: false,
            },
            CodeLine {
                number: 16,
                text: "pub enum ShellNav {",
                highlighted: true,
            },
            CodeLine {
                number: 17,
                text: "    Inbox,",
                highlighted: true,
            },
            CodeLine {
                number: 18,
                text: "    #[default]",
                highlighted: true,
            },
            CodeLine {
                number: 19,
                text: "    PullRequests,",
                highlighted: true,
            },
            CodeLine {
                number: 20,
                text: "}",
                highlighted: true,
            },
            CodeLine {
                number: 21,
                text: "",
                highlighted: false,
            },
        ]),
        status: ThreadStatus::Open,
        comments: &[
            StubComment {
                author: "alice",
                body: "Can we keep Dock Dense padding on the sidebar list? Calling out `ShellNav` — Inbox vs Pull requests density should match the Linear Issues row height.",
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
        line_label: Some("L115–L118"),
        code_context: Some(&[
            CodeLine {
                number: 114,
                text: "                    .child(",
                highlighted: false,
            },
            CodeLine {
                number: 115,
                text: "                        div()",
                highlighted: true,
            },
            CodeLine {
                number: 116,
                text: "                            .text_xs()",
                highlighted: true,
            },
            CodeLine {
                number: 117,
                text: "                            .text_color(cx.theme().muted_foreground)",
                highlighted: true,
            },
            CodeLine {
                number: 118,
                text: "                            .child(\"prototype · Linear comments\"),",
                highlighted: true,
            },
            CodeLine {
                number: 119,
                text: "                    ),",
                highlighted: false,
            },
        ]),
        status: ThreadStatus::Open,
        comments: &[
            StubComment {
                author: "bob",
                body: "Metadata should stay open by default only on Overview — this title chrome is fine as muted.",
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
        line_label: Some("L95–L100"),
        code_context: Some(&[
            CodeLine {
                number: 94,
                text: "            notif_scroll: UniformListScrollHandle::new(),",
                highlighted: false,
            },
            CodeLine {
                number: 95,
                text: "            chrome: ChromeVariant::A,",
                highlighted: true,
            },
            CodeLine {
                number: 96,
                text: "            shell_nav: ShellNav::PullRequests,",
                highlighted: true,
            },
            CodeLine {
                number: 97,
                text: "            prototype_tab: PrototypeReviewTab::Conversation,",
                highlighted: true,
            },
            CodeLine {
                number: 98,
                text: "            metadata_open: false,",
                highlighted: true,
            },
            CodeLine {
                number: 99,
                text: "            hide_resolved_threads: false,",
                highlighted: true,
            },
            CodeLine {
                number: 100,
                text: "            reply_thread_id: Some(default_reply_thread_id()),",
                highlighted: true,
            },
        ]),
        status: ThreadStatus::Open,
        comments: &[StubComment {
            author: "you",
            body: "Boot straight into Conversation for this prototype — Suggestion below swaps the Annotation-era wording if we still had it here.",
            relative_time: "Draft",
            pending: true,
            suggestion: Some(
                "            prototype_tab: PrototypeReviewTab::Conversation,\n            // Pending review drafts, not Annotations",
            ),
            showcase_resolve: false,
        }],
    },
    StubThread {
        id: "t4",
        path: Some("docs/agents/domain.md"),
        line_label: Some("file"),
        code_context: None,
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
        code_context: Some(&[
            CodeLine {
                number: 10,
                text: "pub fn local_store_db_path() -> Result<PathBuf, …> {",
                highlighted: false,
            },
            CodeLine {
                number: 11,
                text: "    // outdated: helper moved — Thread still replyable",
                highlighted: true,
            },
            CodeLine {
                number: 12,
                text: "    Ok(app_data_dir()?.join(\"reviewdesk.db\"))",
                highlighted: true,
            },
            CodeLine {
                number: 13,
                text: "}",
                highlighted: false,
            },
        ]),
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
