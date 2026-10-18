//! PROTOTYPE shell chrome helpers (locked pick from #58).
//!
//! HitL pick: Linear sidebar + right Metadata dock + focus Review on open.
//! Tab order: Overview, Pipelines, Conversation, Changes. Metadata opens by
//! default on Overview only. Thread layout variants live in
//! [`crate::threads_prototype`] — this module only holds chrome enums.

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
    /// Last tab; paints the existing Files/Diff surface.
    Changes,
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
            Self::A => "Picked (Dock + Focus)",
            Self::B => "Linear + Header Meta",
            Self::C => "Linear + Drawer",
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
            Self::A => 200.,
            Self::B | Self::C => 220.,
        }
    }

    pub fn list_default_width(self) -> f32 {
        match self {
            Self::A => 280.,
            Self::B => 320.,
            Self::C => 340.,
        }
    }

    pub fn metadata_default_width(self) -> f32 {
        match self {
            Self::A => 260.,
            Self::B | Self::C => 280.,
        }
    }

    /// Opening a PR hides the list; Back restores it (A pick + C).
    pub fn focus_stack(self) -> bool {
        matches!(self, Self::A | Self::C)
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
