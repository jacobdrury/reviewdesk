//! PROTOTYPE (throwaway): Pull requests list in Views + left nav.
//!
//! Locked to the Linear Issues direction (former variant A). Iterating in place —
//! B/C switcher removed.
//!
//! Layout: Linear sidebar (Inbox + global Pull requests + Your repositories);
//! full-bleed PR list; View tabs; Filter chips + Display; dense single-line rows;
//! focus Review with Back on open; Metadata right dock.
//!
//! Stub Views from #52: All PRs + Needs review / Mine / Drafts, plus By project
//! (repo → status nesting). Demo mode cycles populated ↔ empty store ↔ zero-match.
//!
//! Run: `cargo run -p reviewdesk`
//! Branch: `prototype/pull-requests-list-nav` (never merge as-is)

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ShellNav {
    Inbox,
    #[default]
    PullRequests,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum PrototypeReviewTab {
    #[default]
    Overview,
    Pipelines,
    Conversation,
    /// Last tab; paints the existing Files/Diff surface when a real PR is open.
    Changes,
}

/// Built-in + seed Views from #52 grilling, plus a nested group-by prototype.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum StubViewId {
    AllPrs,
    NeedsMyReview,
    Mine,
    Drafts,
    /// Prototype: group by repository, then by review status within each repo.
    ByProject,
}

/// Force empty / zero-match surfaces without real LocalStore gymnastics.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ListDemoMode {
    #[default]
    Populated,
    /// No cached pull requests at all.
    EmptyStore,
    /// Cache has PRs; this View's filters match none.
    ZeroMatch,
}

pub const SIDEBAR_WIDTH: f32 = 244.;
pub const METADATA_DEFAULT_WIDTH: f32 = 260.;

impl ShellNav {
    pub fn label(self) -> &'static str {
        match self {
            Self::Inbox => "Inbox",
            Self::PullRequests => "Pull requests",
        }
    }
}

impl PrototypeReviewTab {
    pub fn label(self) -> &'static str {
        match self {
            Self::Overview => "Overview",
            Self::Pipelines => "Pipelines",
            Self::Conversation => "Conversation",
            Self::Changes => "Changes",
        }
    }

    pub fn index(self) -> usize {
        match self {
            Self::Overview => 0,
            Self::Pipelines => 1,
            Self::Conversation => 2,
            Self::Changes => 3,
        }
    }

    pub fn from_index(ix: usize) -> Self {
        match ix {
            1 => Self::Pipelines,
            2 => Self::Conversation,
            3 => Self::Changes,
            _ => Self::Overview,
        }
    }

    pub fn metadata_open_by_default(self) -> bool {
        matches!(self, Self::Overview)
    }
}

impl StubViewId {
    pub const ALL: &[Self] = &[
        Self::AllPrs,
        Self::NeedsMyReview,
        Self::Mine,
        Self::Drafts,
        Self::ByProject,
    ];

    pub fn label(self) -> &'static str {
        match self {
            Self::AllPrs => "All PRs",
            Self::NeedsMyReview => "Needs review",
            Self::Mine => "Mine",
            Self::Drafts => "Drafts",
            Self::ByProject => "By project",
        }
    }

    pub fn builtin(self) -> bool {
        matches!(self, Self::AllPrs)
    }

    pub fn filter_summary(self) -> &'static str {
        match self {
            Self::AllPrs => "No filters · updated ↓",
            Self::NeedsMyReview => "I am reviewer · ready · open",
            Self::Mine => "I am author · open",
            Self::Drafts => "I am author · draft",
            Self::ByProject => "Grouped by repo → status",
        }
    }

    pub fn group_by(self) -> Option<&'static str> {
        match self {
            Self::AllPrs => Some("review state"),
            Self::NeedsMyReview => Some("review state"),
            Self::Mine => Some("repo"),
            Self::Drafts => Some("repo"),
            Self::ByProject => Some("repo→status"),
        }
    }

    pub fn sort_label(self) -> &'static str {
        "Updated ↓"
    }
}

impl ListDemoMode {
    pub fn label(self) -> &'static str {
        match self {
            Self::Populated => "Populated",
            Self::EmptyStore => "Empty store",
            Self::ZeroMatch => "Zero-match",
        }
    }

    pub fn next(self) -> Self {
        match self {
            Self::Populated => Self::EmptyStore,
            Self::EmptyStore => Self::ZeroMatch,
            Self::ZeroMatch => Self::Populated,
        }
    }
}

/// Stub repos shown under “Your repositories”.
pub const STUB_REPOS: &[&str] = &[
    "jacobdrury/reviewdesk",
    "acme/widgets",
    "acme/kit",
];
