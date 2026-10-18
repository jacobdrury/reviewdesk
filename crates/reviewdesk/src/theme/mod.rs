//! Zed theme sync and adapter for Review Desk.
//!
//! #22 owns Fallback One Dark activation and the Zed→gpui-kit rewrite.
//! #23 extends discovery, focus re-sync, and missing-name toast.

mod adapter;
mod discovery;
mod fallback;
mod sync;

pub use discovery::{discover_macos, DiscoveredThemes};
pub use sync::{
    apply_sync_result, missing_theme_notification, should_sync_on_activation, Appearance,
    ToastIntent,
};
