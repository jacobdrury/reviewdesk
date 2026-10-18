//! Private paint modules. Not a behavioral seam — they read [`SessionView`] only.

mod files;
mod inbox;
mod overview;
mod review_pane;
mod shell_prototype;
mod threads_prototype;

pub(crate) use files::render_files;
#[allow(unused_imports)] // kept for non-prototype shell restore
pub(crate) use inbox::render_inbox;
pub(crate) use overview::render_overview;
#[allow(unused_imports)] // kept for non-prototype shell restore
pub(crate) use review_pane::render_review_pane;
pub(crate) use shell_prototype::{PrototypeChrome, render_prototype_shell, sync_session_tab};
pub(crate) use threads_prototype::{
    render_changes_with_threads, render_conversation, render_diff_thread_annotation,
};
