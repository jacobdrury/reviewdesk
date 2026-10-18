//! PROTOTYPE (throwaway): Zed-inspired shell chrome variants for Review Desk.
//!
//! Question: what density, panel chrome, borders, title bar, and typography feel
//! right on the existing Inbox | Review pane shell?
//!
//! Three variants on the live app, cycled with the bottom bar (or `[` / `]`):
//! - A (Dock Dense): Zed dock density, uppercase panel labels, hairline borders
//! - B (Tabbed Workspace): global Overview/Files tabs under the title bar, status strip
//! - C (Tone Split): panels separated by background tone, no vertical borders, single-line Inbox rows
//!
//! Run: `cargo run -p reviewdesk`
//! Branch: `prototype/zed-shell-chrome` (never merge as-is)

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ChromeVariant {
    #[default]
    A,
    B,
    C,
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
            Self::B => "Tabbed Workspace",
            Self::C => "Tone Split",
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

    pub fn inbox_default_width(self) -> f32 {
        match self {
            Self::A => 240.,
            Self::B => 280.,
            Self::C => 300.,
        }
    }

    pub fn inbox_min_width(self) -> f32 {
        match self {
            Self::A => 200.,
            Self::B => 220.,
            Self::C => 240.,
        }
    }

    /// Global Overview/Files tabs under the title bar (Variant B only).
    pub fn global_tabs(self) -> bool {
        matches!(self, Self::B)
    }

    pub fn tone_split_panels(self) -> bool {
        matches!(self, Self::C)
    }

    pub fn dense_headers(self) -> bool {
        matches!(self, Self::A)
    }

    pub fn uppercase_panel_labels(self) -> bool {
        matches!(self, Self::A)
    }

    pub fn hide_inbox_title(self) -> bool {
        matches!(self, Self::B)
    }

    pub fn single_line_inbox_rows(self) -> bool {
        matches!(self, Self::C)
    }

    pub fn overview_as_card(self) -> bool {
        !matches!(self, Self::C)
    }

    pub fn show_status_strip(self) -> bool {
        matches!(self, Self::B | Self::C)
    }
}
