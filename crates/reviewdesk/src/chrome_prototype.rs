//! PROTOTYPE (throwaway): primary shell chrome host for Pipelines tab work.
//!
//! Baked HITL pick from `prototype/primary-shell-chrome`: Linear sidebar,
//! right Metadata dock, focus Review on open, tabs Overview · Pipelines ·
//! Conversation · Changes. This branch only varies the Pipelines tab body
//! (see `pipelines_prototype`).
//!
//! Run: `cargo run -p reviewdesk`
//! Branch: `prototype/pipelines-tab` (never merge as-is)

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ChromeVariant {
    #[default]
    A,
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
    /// Last tab; paints the existing Files/Diff surface.
    Changes,
}

impl ChromeVariant {
    pub fn sidebar_width(self) -> f32 {
        200.
    }

    pub fn list_default_width(self) -> f32 {
        280.
    }

    pub fn metadata_default_width(self) -> f32 {
        260.
    }

    pub fn focus_stack(self) -> bool {
        true
    }

    pub fn metadata_right_dock(self) -> bool {
        true
    }

    pub fn metadata_under_tabs(self) -> bool {
        false
    }

    pub fn metadata_bottom_drawer(self) -> bool {
        false
    }

    pub fn dense(self) -> bool {
        true
    }

    pub fn uppercase_labels(self) -> bool {
        true
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

    /// Metadata starts open on Overview; other tabs collapse it by default.
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
