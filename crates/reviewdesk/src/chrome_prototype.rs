//! PROTOTYPE (throwaway): primary shell chrome for Review Desk.
//!
//! Question: how should Inbox ↔ Pull requests nav, Review pane tabs
//! (Overview / Files / Pipelines / Conversation), and a foldable Metadata
//! panel (open by default) sit together, and which density wins against the
//! earlier Dock Dense notes (`prototype/zed-shell-chrome` variant A)?
//!
//! Three variants on the live app, cycled with the bottom bar (or `[` / `]`):
//! - A (Dock Dense): activity rail + list + review + right Metadata dock
//! - B (Header Metadata): title-bar nav; Metadata folds under the tab bar
//! - C (Focus Stack): list or review fills the stage; Metadata is a bottom drawer
//!
//! Run: `cargo run -p reviewdesk`
//! Branch: `prototype/primary-shell-chrome` (never merge as-is)

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
    Files,
    Pipelines,
    Conversation,
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
            Self::A => "Dock Dense",
            Self::B => "Header Metadata",
            Self::C => "Focus Stack",
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

    pub fn list_default_width(self) -> f32 {
        match self {
            Self::A => 240.,
            Self::B => 300.,
            Self::C => 320.,
        }
    }

    pub fn list_min_width(self) -> f32 {
        match self {
            Self::A => 200.,
            Self::B => 240.,
            Self::C => 260.,
        }
    }

    pub fn metadata_default_width(self) -> f32 {
        match self {
            Self::A => 260.,
            Self::B | Self::C => 280.,
        }
    }

    /// Narrow icon rail for Inbox / Pull requests (A only).
    pub fn activity_rail(self) -> bool {
        matches!(self, Self::A)
    }

    /// Inbox | Pull requests as text control in the title bar (B only).
    pub fn title_bar_nav(self) -> bool {
        matches!(self, Self::B)
    }

    /// Labeled sidebar nav instead of a rail (C only).
    pub fn labeled_sidebar_nav(self) -> bool {
        matches!(self, Self::C)
    }

    /// Opening a PR hides the list; Back restores it (C only).
    pub fn focus_stack(self) -> bool {
        matches!(self, Self::C)
    }

    /// Metadata as a right dock panel (A).
    pub fn metadata_right_dock(self) -> bool {
        matches!(self, Self::A)
    }

    /// Metadata as a band under the review tab bar (B).
    pub fn metadata_under_tabs(self) -> bool {
        matches!(self, Self::B)
    }

    /// Metadata as a bottom drawer (C).
    pub fn metadata_bottom_drawer(self) -> bool {
        matches!(self, Self::C)
    }

    pub fn dense(self) -> bool {
        matches!(self, Self::A)
    }

    pub fn uppercase_labels(self) -> bool {
        matches!(self, Self::A)
    }

    pub fn tone_split(self) -> bool {
        matches!(self, Self::C)
    }
}

impl PrototypeReviewTab {
    pub fn label(self) -> &'static str {
        match self {
            Self::Overview => "Overview",
            Self::Files => "Files",
            Self::Pipelines => "Pipelines",
            Self::Conversation => "Conversation",
        }
    }

    pub fn index(self) -> usize {
        match self {
            Self::Overview => 0,
            Self::Files => 1,
            Self::Pipelines => 2,
            Self::Conversation => 3,
        }
    }

    pub fn from_index(ix: usize) -> Self {
        match ix {
            1 => Self::Files,
            2 => Self::Pipelines,
            3 => Self::Conversation,
            _ => Self::Overview,
        }
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
