//! PROTOTYPE (throwaway): Pull requests list in Views + left nav refinement.
//!
//! Question: what should the PR list inside a View look like (row density,
//! grouping, filter/sort chrome, empty vs zero-match), and how should the
//! Linear-style left nav place Inbox / Pull requests / Views?
//!
//! Builds on #58 HITL pick: Linear sidebar, full-screen Review with Back,
//! Overview · Pipelines · Conversation · Changes, Metadata right dock.
//! Variants only change Views placement + list structure — not Review chrome.
//!
//! Three variants, cycled with the bottom bar (or `[` / `]`):
//! - A (Linear Issues): flat sidebar (Inbox + Pull requests); full-bleed list;
//!   View tabs like Linear All/Active/Backlog; Filter + Display; dense single-line
//!   rows (CI · owner/repo#n · review · title · updated · author); group headers.
//! - B (View chips): flat sidebar; Views as chips; roomier multi-line rows;
//!   side-by-side list/Review (no focus stack).
//! - C (Views rail): Views index rail beside a narrower list; table-ish columns;
//!   focus Review on open.
//!
//! Stub Views from #52: All PRs (builtin) + Needs my review / Mine / Drafts.
//! Demo mode cycles populated ↔ empty store ↔ zero-match.
//!
//! Run: `cargo run -p reviewdesk`
//! Branch: `prototype/pull-requests-list-nav` (never merge as-is)

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ChromeVariant {
    #[default]
    A,
    B,
    C,
}

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

/// Built-in + seed Views from #52 grilling.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum StubViewId {
    AllPrs,
    NeedsMyReview,
    Mine,
    Drafts,
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

impl ChromeVariant {
    pub fn key(self) -> &'static str {
        match self {
            Self::A => "A",
            Self::B => "B",
            Self::C => "C",
        }
    }

    pub fn name(self) -> &'static str {
        match self {
            Self::A => "Linear Issues",
            Self::B => "View chips",
            Self::C => "Views rail",
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

    pub fn sidebar_width(self) -> f32 {
        match self {
            Self::A => 220.,
            Self::B => 200.,
            Self::C => 200.,
        }
    }

    /// Width of the PR list column when not full-bleed (not the Views rail).
    pub fn list_default_width(self) -> f32 {
        match self {
            Self::A => 320.,
            Self::B => 360.,
            Self::C => 380.,
        }
    }

    /// List owns the main pane (Linear Issues); Review only after open + Back.
    pub fn full_bleed_list(self) -> bool {
        matches!(self, Self::A)
    }

    /// View tabs across the top of the list (Linear All / Active / Backlog).
    pub fn views_as_tabs(self) -> bool {
        matches!(self, Self::A)
    }

    pub fn views_rail_width(self) -> f32 {
        168.
    }

    pub fn metadata_default_width(self) -> f32 {
        260.
    }

    /// Views live as indented children under Pull requests in the sidebar.
    pub fn views_in_sidebar(self) -> bool {
        false
    }

    /// Views as a horizontal chip strip above the PR list.
    pub fn views_as_chips(self) -> bool {
        matches!(self, Self::B)
    }

    /// Views as a narrow index rail beside the PR list.
    pub fn views_as_rail(self) -> bool {
        matches!(self, Self::C)
    }

    /// Opening a PR hides the list; Back restores it (A + C).
    pub fn focus_stack(self) -> bool {
        matches!(self, Self::A | Self::C)
    }

    /// Dense Dock-style row padding (A).
    pub fn dense(self) -> bool {
        matches!(self, Self::A)
    }

    /// Roomier multi-line PR rows (B).
    pub fn roomy_rows(self) -> bool {
        matches!(self, Self::B)
    }

    /// Table-ish columns in the PR list (C).
    pub fn table_rows(self) -> bool {
        matches!(self, Self::C)
    }

    pub fn uppercase_labels(self) -> bool {
        matches!(self, Self::A)
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

impl ShellNav {
    pub fn label(self) -> &'static str {
        match self {
            Self::Inbox => "Inbox",
            Self::PullRequests => "Pull requests",
        }
    }
}

impl StubViewId {
    pub const ALL: &[Self] = &[
        Self::AllPrs,
        Self::NeedsMyReview,
        Self::Mine,
        Self::Drafts,
    ];

    pub fn label(self) -> &'static str {
        match self {
            Self::AllPrs => "All PRs",
            Self::NeedsMyReview => "Needs my review",
            Self::Mine => "Mine",
            Self::Drafts => "Drafts",
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
        }
    }

    /// Default group-by for the prototype list (None = flat).
    pub fn group_by(self) -> Option<&'static str> {
        match self {
            Self::AllPrs => Some("review state"),
            Self::NeedsMyReview => Some("review state"),
            Self::Mine => Some("repo"),
            Self::Drafts => Some("repo"),
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
