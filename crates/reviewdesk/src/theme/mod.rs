//! Zed theme sync and adapter for Review Desk.
//!
//! #22 owns Fallback One Dark activation and the Zed→gpui-kit rewrite.
//! #23 extends discovery, focus re-sync, and missing-name toast.

#![allow(unused_imports)] // Re-exports are the #22/#23 public seam.

mod adapter;
mod discovery;
mod fallback;
mod sync;

pub use adapter::{adapt_zed_family, adapt_zed_theme, AdaptError};
pub use discovery::{discover_macos, DiscoveredThemes};
pub use fallback::{fallback_one_dark, FALLBACK_THEME_NAME, ONE_FAMILY_JSON};
pub use sync::{
    apply_sync_result, missing_theme_toast_message, resolve_active_theme_name, sync_theme,
    Appearance, DiscoveryInputs, SyncInputs, SyncResult, ToastIntent,
};
