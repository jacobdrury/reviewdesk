//! Thin GPUI shell: chrome, Effect driver, and Render over [`ReviewSession`].

use std::collections::HashSet;
use std::sync::Arc;

use gpui_kit::component::{
    WindowExt as _,
    diff::{DiffMode, DiffState},
};
use gpui_kit::{
    AppContext as _, Context, Entity, IntoElement, Render, Subscription, Task,
    UniformListScrollHandle, Window, WindowAppearance,
};
use reviewdesk_core::{
    InboxOptions, InboxQuery, LocalStoreError, Provider, ProviderError, PullRequestId,
};
use tokio::runtime::Handle;

use crate::INBOX_REFRESH;
use crate::chrome_prototype::{
    ChromeVariant, ListDemoMode, PrototypeReviewTab, ShellNav, StubViewId,
};
use crate::review_session::{Effect, Intent, ReviewSession};
use crate::sync::{
    AppLocalStore, load_detail_via_store, load_patch_via_store, refresh_patch,
    refresh_pull_request_detail, sync_inbox_from_provider,
};
use crate::theme::{
    Appearance, ToastIntent, apply_sync_result, discover_macos, missing_theme_notification,
    should_sync_on_activation,
};
use crate::ui::{PrototypeChrome, render_prototype_shell, sync_session_tab};

fn appearance_from_window(window: &Window) -> Appearance {
    appearance_from_system(window.appearance())
}

pub(crate) fn appearance_from_system(appearance: WindowAppearance) -> Appearance {
    match appearance {
        WindowAppearance::Dark | WindowAppearance::VibrantDark => Appearance::Dark,
        WindowAppearance::Light | WindowAppearance::VibrantLight => Appearance::Light,
    }
}

fn map_store_error(error: LocalStoreError) -> ProviderError {
    ProviderError::other(error.to_string())
}

/// Window Entity: session kernel + GPUI-only chrome and Effect execution.
pub struct ReviewDesk {
    session: ReviewSession,
    provider: Arc<dyn Provider>,
    store: Arc<AppLocalStore>,
    _runtime: Arc<tokio::runtime::Runtime>,
    runtime: Handle,
    diff: Entity<DiffState>,
    inbox_scroll: UniformListScrollHandle,
    files_scroll: UniformListScrollHandle,
    notif_scroll: UniformListScrollHandle,
    /// PROTOTYPE chrome state (throwaway with `prototype/pull-requests-list-nav`).
    chrome: ChromeVariant,
    shell_nav: ShellNav,
    prototype_tab: PrototypeReviewTab,
    metadata_open: bool,
    selected_view: StubViewId,
    list_demo: ListDemoMode,
    selected_stub_id: Option<&'static str>,
    /// `None` = global Pull requests; `Some(repo)` filters the list to that repo.
    selected_repo: Option<&'static str>,
    /// Collapsed group keys (headers + nested sub-headers).
    collapsed_groups: HashSet<String>,
    _inbox_reload_task: Task<()>,
    _inbox_sync_task: Task<()>,
    _detail_task: Task<()>,
    _patch_task: Task<()>,
    _refresh_timer: Task<()>,
    _window_activation: Option<Subscription>,
    skip_next_activation_sync: bool,
    last_diff_revision: u64,
}

impl ReviewDesk {
    pub fn new(
        provider: Arc<dyn Provider>,
        store: Arc<AppLocalStore>,
        runtime: Arc<tokio::runtime::Runtime>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Self {
        window.set_window_title("Review Desk");
        let diff = cx.new(|cx| DiffState::new([], cx).with_mode(DiffMode::Split));
        let mut this = Self {
            session: ReviewSession::new(),
            provider,
            store,
            runtime: runtime.handle().clone(),
            _runtime: runtime,
            diff,
            inbox_scroll: UniformListScrollHandle::new(),
            files_scroll: UniformListScrollHandle::new(),
            notif_scroll: UniformListScrollHandle::new(),
            chrome: ChromeVariant::A,
            shell_nav: ShellNav::PullRequests,
            prototype_tab: PrototypeReviewTab::Overview,
            metadata_open: true,
            selected_view: StubViewId::NeedsMyReview,
            list_demo: ListDemoMode::Populated,
            selected_stub_id: None,
            selected_repo: None,
            collapsed_groups: HashSet::new(),
            _inbox_reload_task: Task::ready(()),
            _inbox_sync_task: Task::ready(()),
            _detail_task: Task::ready(()),
            _patch_task: Task::ready(()),
            _refresh_timer: Task::ready(()),
            _window_activation: None,
            skip_next_activation_sync: false,
            last_diff_revision: 0,
        };
        this.sync_theme(window, cx);
        this.skip_next_activation_sync = true;
        this._window_activation = Some(cx.observe_window_activation(window, |this, window, cx| {
            if !window.is_window_active() {
                return;
            }
            if !should_sync_on_activation(&mut this.skip_next_activation_sync) {
                return;
            }
            this.sync_theme(window, cx);
        }));
        this.dispatch(Intent::Boot, cx);
        this
    }

    pub(crate) fn dispatch(&mut self, intent: Intent, cx: &mut Context<Self>) {
        let effects = self.session.apply(intent);
        self.run_effects(effects, cx);
        self.sync_diff_widget(cx);
        cx.notify();
    }

    fn run_effects(&mut self, effects: Vec<Effect>, cx: &mut Context<Self>) {
        for effect in effects {
            match effect {
                Effect::QueryInbox { generation } => self.run_query_inbox(generation, cx),
                Effect::SyncInbox { generation } => self.run_sync_inbox(generation, cx),
                Effect::FetchDetail { id } => self.run_fetch_detail(id, cx),
                Effect::FetchPatch { id } => self.run_fetch_patch(id, cx),
                Effect::ArmInboxRefreshTimer => self.arm_inbox_refresh_timer(cx),
            }
        }
    }

    fn sync_diff_widget(&mut self, cx: &mut Context<Self>) {
        let view = self.session.view();
        if view.diff_revision == self.last_diff_revision {
            return;
        }
        self.last_diff_revision = view.diff_revision;
        let mode = view.diff_mode;
        let files = match (&view.selected_file_error, &view.selected_diff_file) {
            (Some(_), _) | (None, None) => Vec::new(),
            (None, Some(file)) => vec![file.clone()],
        };
        self.diff.update(cx, |state, cx| {
            if state.mode() != mode {
                state.set_mode(mode, cx);
            }
            state.set_files(files, cx);
        });
    }

    fn sync_theme(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let appearance = appearance_from_window(window);
        let result = discover_macos().sync(appearance);
        apply_sync_result(cx, &result);
        if let ToastIntent::MissingTheme { name } = &result.toast {
            window.push_notification(missing_theme_notification(name), cx);
        }
        cx.notify();
    }

    fn run_query_inbox(&mut self, generation: u64, cx: &mut Context<Self>) {
        let store = self.store.clone();
        let runtime = self.runtime.clone();
        self._inbox_reload_task = cx.spawn(async move |this, cx| {
            let result = runtime
                .spawn(async move { store.query_inbox(&InboxQuery::default()).await })
                .await
                .unwrap_or_else(|e| Err(LocalStoreError::other(e.to_string())));
            let result = result.map_err(map_store_error);
            let _ = this.update(cx, |this, cx| {
                this.dispatch(
                    Intent::InboxQueryFinished { generation, result },
                    cx,
                );
            });
        });
    }

    fn run_sync_inbox(&mut self, generation: u64, cx: &mut Context<Self>) {
        let provider = self.provider.clone();
        let store = self.store.clone();
        let runtime = self.runtime.clone();
        let options = InboxOptions::default();
        self._inbox_sync_task = cx.spawn(async move |this, cx| {
            let result = runtime
                .spawn(async move {
                    sync_inbox_from_provider(provider.as_ref(), &store, &options).await
                })
                .await
                .unwrap_or_else(|e| Err(ProviderError::other(e.to_string())));
            let _ = this.update(cx, |this, cx| {
                this.dispatch(Intent::InboxSyncFinished { generation, result }, cx);
            });
        });
    }

    fn arm_inbox_refresh_timer(&mut self, cx: &mut Context<Self>) {
        self._refresh_timer = cx.spawn(async move |this, cx| {
            loop {
                cx.background_executor().timer(INBOX_REFRESH).await;
                if this
                    .update(cx, |this, cx| {
                        this.dispatch(Intent::RefreshInbox, cx);
                    })
                    .is_err()
                {
                    break;
                }
            }
        });
    }

    fn run_fetch_detail(&mut self, id: PullRequestId, cx: &mut Context<Self>) {
        let provider = self.provider.clone();
        let store = self.store.clone();
        let runtime = self.runtime.clone();
        let request_id = id;
        self._detail_task = cx.spawn(async move |this, cx| {
            let cache_id = request_id.clone();
            let cached = runtime
                .spawn({
                    let store = store.clone();
                    async move { store.get_pull_request_detail(&cache_id).await }
                })
                .await
                .unwrap_or_else(|e| Err(LocalStoreError::other(e.to_string())));
            let local_result = cached.map_err(map_store_error);
            let had_cache = matches!(&local_result, Ok(Some(_)));

            let _ = this.update(cx, |this, cx| {
                this.dispatch(
                    Intent::DetailLocalFinished {
                        id: request_id.clone(),
                        result: local_result,
                    },
                    cx,
                );
            });

            let fetch_id = request_id.clone();
            let result = runtime
                .spawn(async move {
                    if had_cache {
                        refresh_pull_request_detail(provider.as_ref(), &store, &fetch_id).await
                    } else {
                        load_detail_via_store(provider.as_ref(), &store, &fetch_id).await
                    }
                })
                .await
                .unwrap_or_else(|e| Err(ProviderError::other(e.to_string())));

            let _ = this.update(cx, |this, cx| {
                this.dispatch(
                    Intent::DetailNetworkFinished {
                        id: request_id,
                        had_cache,
                        result,
                    },
                    cx,
                );
            });
        });
    }

    pub(crate) fn cycle_chrome(&mut self, next: bool, cx: &mut Context<Self>) {
        self.chrome = if next {
            self.chrome.next()
        } else {
            self.chrome.prev()
        };
        self.selected_stub_id = None;
        cx.notify();
    }

    pub(crate) fn set_shell_nav(&mut self, nav: ShellNav, cx: &mut Context<Self>) {
        self.shell_nav = nav;
        if nav == ShellNav::Inbox {
            self.selected_stub_id = None;
        }
        if nav == ShellNav::PullRequests {
            // Global Pull requests entry clears a repo scope.
            self.selected_repo = None;
        }
        cx.notify();
    }

    pub(crate) fn set_selected_view(&mut self, view: StubViewId, cx: &mut Context<Self>) {
        self.selected_view = view;
        self.selected_stub_id = None;
        self.shell_nav = ShellNav::PullRequests;
        cx.notify();
    }

    pub(crate) fn set_selected_repo(&mut self, repo: Option<&'static str>, cx: &mut Context<Self>) {
        self.selected_repo = repo;
        self.selected_stub_id = None;
        self.shell_nav = ShellNav::PullRequests;
        cx.notify();
    }

    pub(crate) fn toggle_collapsed_group(&mut self, key: String, cx: &mut Context<Self>) {
        if !self.collapsed_groups.remove(&key) {
            self.collapsed_groups.insert(key);
        }
        cx.notify();
    }

    pub(crate) fn cycle_list_demo(&mut self, cx: &mut Context<Self>) {
        self.list_demo = self.list_demo.next();
        self.selected_stub_id = None;
        cx.notify();
    }

    pub(crate) fn set_list_demo(&mut self, demo: ListDemoMode, cx: &mut Context<Self>) {
        self.list_demo = demo;
        self.selected_stub_id = None;
        cx.notify();
    }

    pub(crate) fn set_selected_stub(&mut self, id: &'static str, cx: &mut Context<Self>) {
        self.selected_stub_id = Some(id);
        self.shell_nav = ShellNav::PullRequests;
        cx.notify();
    }

    pub(crate) fn clear_selected_stub(&mut self, cx: &mut Context<Self>) {
        self.selected_stub_id = None;
        cx.notify();
    }

    pub(crate) fn set_prototype_tab(&mut self, tab: PrototypeReviewTab, cx: &mut Context<Self>) {
        self.prototype_tab = tab;
        // Overview opens Metadata by default; other tabs collapse it (user can re-open).
        self.metadata_open = tab.metadata_open_by_default();
        if let Some(session_tab) = sync_session_tab(tab) {
            self.dispatch(Intent::SetReviewTab(session_tab), cx);
            return;
        }
        cx.notify();
    }

    pub(crate) fn toggle_metadata(&mut self, cx: &mut Context<Self>) {
        self.metadata_open = !self.metadata_open;
        cx.notify();
    }

    fn run_fetch_patch(&mut self, id: PullRequestId, cx: &mut Context<Self>) {
        let provider = self.provider.clone();
        let store = self.store.clone();
        let runtime = self.runtime.clone();
        let request_id = id;
        self._patch_task = cx.spawn(async move |this, cx| {
            let cache_id = request_id.clone();
            let cached = runtime
                .spawn({
                    let store = store.clone();
                    async move { store.get_patch(&cache_id).await }
                })
                .await
                .unwrap_or_else(|e| Err(LocalStoreError::other(e.to_string())));
            let local_result = cached.map_err(map_store_error);
            let had_cache = matches!(&local_result, Ok(Some(_)));

            let _ = this.update(cx, |this, cx| {
                this.dispatch(
                    Intent::PatchLocalFinished {
                        id: request_id.clone(),
                        result: local_result,
                    },
                    cx,
                );
            });

            let fetch_id = request_id.clone();
            let result = runtime
                .spawn(async move {
                    if had_cache {
                        refresh_patch(provider.as_ref(), &store, &fetch_id).await
                    } else {
                        load_patch_via_store(provider.as_ref(), &store, &fetch_id).await
                    }
                })
                .await
                .unwrap_or_else(|e| Err(ProviderError::other(e.to_string())));

            let _ = this.update(cx, |this, cx| {
                this.dispatch(
                    Intent::PatchNetworkFinished {
                        id: request_id,
                        had_cache,
                        result,
                    },
                    cx,
                );
            });
        });
    }
}

impl Render for ReviewDesk {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let view = self.session.view();
        let entity = cx.entity().downgrade();
        render_prototype_shell(
            PrototypeChrome {
                chrome: self.chrome,
                shell_nav: self.shell_nav,
                review_tab: self.prototype_tab,
                metadata_open: self.metadata_open,
                selected_view: self.selected_view,
                list_demo: self.list_demo,
                selected_stub_id: self.selected_stub_id,
                selected_repo: self.selected_repo,
                collapsed_groups: self.collapsed_groups.clone(),
                view,
                diff: self.diff.clone(),
                inbox_scroll: self.inbox_scroll.clone(),
                files_scroll: self.files_scroll.clone(),
                notif_scroll: self.notif_scroll.clone(),
                entity,
            },
            cx,
        )
    }
}
