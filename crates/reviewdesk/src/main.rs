mod app;
mod auth;
mod chrome_prototype;
mod overview_body;
mod patch_index;
mod paths;
mod review_session;
mod sync;
mod theme;
mod threads_prototype;
mod ui;

use std::sync::Arc;
use std::time::Duration;

use gpui_kit::{AppContext as _, WindowBounds, WindowOptions, component::TitleBar, px, size};
use reviewdesk_core::Provider;
use reviewdesk_github::GitHubProvider;
use reviewdesk_localstore::TursoLocalStore;
use tokio::runtime::Runtime;

use app::{ReviewDesk, appearance_from_system};
use auth::resolve_credentials;
use paths::{ensure_db_parent, local_store_db_path};
use sync::AppLocalStore;
use theme::{apply_sync_result, discover_macos};

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

    let db_path = match local_store_db_path() {
        Ok(path) => path,
        Err(error) => {
            eprintln!("Failed to resolve local database path: {error}");
            std::process::exit(1);
        }
    };
    if let Err(error) = ensure_db_parent(&db_path) {
        eprintln!("Failed to create application data directory: {error}");
        std::process::exit(1);
    }

    let store = match runtime.block_on(async { TursoLocalStore::open(&db_path).await }) {
        Ok(store) => Arc::new(AppLocalStore::new(Arc::new(store))),
        Err(error) => {
            eprintln!(
                "Failed to open local store at {}: {error}",
                db_path.display()
            );
            std::process::exit(1);
        }
    };

    gpui_kit::application()
        .with_assets(gpui_kit::assets::Assets)
        .run(move |cx| {
            gpui_kit::init(cx);
            // Boot with discovery before the first window so paint is not kit light.
            // Use system appearance (same mapping as the window path) for mode:system.
            // ReviewDesk re-syncs on create and on later focus.
            let boot = discover_macos().sync(appearance_from_system(cx.window_appearance()));
            apply_sync_result(cx, &boot);

            let options = WindowOptions {
                window_bounds: Some(WindowBounds::centered(size(px(1280.), px(800.)), cx)),
                window_min_size: Some(size(px(960.), px(600.))),
                app_id: Some("reviewdesk".into()),
                ..TitleBar::window_options()
            };

            gpui_kit::open_window(options, cx, |window, cx| {
                cx.new(|cx| ReviewDesk::new(provider, store.clone(), runtime, window, cx))
            })
            .expect("Failed to open Review Desk window");

            cx.activate(true);
        });
}

/// Inbox auto-refresh interval while the app is foregrounded.
pub const INBOX_REFRESH: Duration = Duration::from_secs(5 * 60);
