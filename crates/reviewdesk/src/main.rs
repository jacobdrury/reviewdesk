mod app;
mod auth;
mod theme;

use std::sync::Arc;
use std::time::Duration;

use gpui_kit::{component::TitleBar, px, size, AppContext as _, WindowBounds, WindowOptions};
use reviewdesk_core::Provider;
use reviewdesk_github::GitHubProvider;
use tokio::runtime::Runtime;

use app::ReviewDesk;
use auth::resolve_credentials;
use theme::{apply_sync_result, sync_theme, SyncInputs};

fn main() {
    let credentials = match resolve_credentials() {
        Ok(credentials) => credentials,
        Err(error) => {
            eprintln!("Review Desk needs a GitHub token.");
            eprintln!("Set GITHUB_TOKEN or GH_TOKEN, or run `gh auth login`.");
            eprintln!("{error}");
            std::process::exit(1);
        }
    };

    let provider: Arc<dyn Provider> = match GitHubProvider::new(credentials) {
        Ok(provider) => Arc::new(provider),
        Err(error) => {
            eprintln!("Failed to create GitHub provider: {error}");
            std::process::exit(1);
        }
    };

    let runtime = match Runtime::new() {
        Ok(runtime) => Arc::new(runtime),
        Err(error) => {
            eprintln!("Failed to start tokio runtime: {error}");
            std::process::exit(1);
        }
    };

    gpui_kit::application()
        .with_assets(gpui_kit::assets::Assets)
        .run(move |cx| {
            gpui_kit::init(cx);
            // #22: silent dark Fallback when Zed/settings are absent.
            // #23 replaces SyncInputs::absent_zed() with real discovery.
            let theme = sync_theme(SyncInputs::absent_zed());
            apply_sync_result(cx, &theme);

            let options = WindowOptions {
                window_bounds: Some(WindowBounds::centered(size(px(1280.), px(800.)), cx)),
                window_min_size: Some(size(px(960.), px(600.))),
                app_id: Some("reviewdesk".into()),
                ..TitleBar::window_options()
            };

            gpui_kit::open_window(options, cx, |window, cx| {
                cx.new(|cx| ReviewDesk::new(provider, runtime, window, cx))
            })
            .expect("Failed to open Review Desk window");

            cx.activate(true);
        });
}

/// Inbox auto-refresh interval while the app is foregrounded.
pub const INBOX_REFRESH: Duration = Duration::from_secs(5 * 60);
