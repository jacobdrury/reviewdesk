//! Thin GPUI shell: chrome, Effect driver, and Render over [`ReviewSession`].

use std::collections::HashSet;
use std::sync::Arc;

use gpui_kit::component::{
    ActiveTheme as _, StyledExt as _, TitleBar, WindowExt as _,
    diff::{DiffMode, DiffState},
    h_flex,
    resizable::{h_resizable, resizable_panel},
    v_flex,
};
use gpui_kit::{
    AppContext as _, Context, Entity, IntoElement, ParentElement as _, Render, Styled as _,
    Subscription, Task, UniformListScrollHandle, Window, WindowAppearance, div, px,
};
use reviewdesk_core::{
    DiffAnchor, DraftFilter, LocalStoreError, NotificationListOptions, NotificationQuery,
    NotificationThreadId, PendingDiffReviewComment, Provider, ProviderError, PullRequestId,
    PullRequestListOptions, PullRequestListQuery, ReviewSubmitEvent, View,
};
use tokio::runtime::Handle;

use crate::PULL_REQUESTS_REFRESH;
use crate::review_session::{Effect, Intent, ReviewSession};
use crate::review_writeback::{
    add_pending_diff_review_comment, discard_pending_review, load_threads_via_store,
    refresh_threads_from_provider, submit_pending_review,
};
use crate::sync::{
    AppLocalStore, load_detail_via_store, load_patch_via_store,
    mark_notification_done_via_provider, mark_notification_read_via_provider, refresh_patch,
    refresh_pull_request_detail, sync_inbox_from_provider, sync_pull_requests_from_provider,
};
use crate::theme::{
    Appearance, ToastIntent, apply_sync_result, discover_macos, missing_theme_notification,
    should_sync_on_activation,
};
use crate::ui::{PullRequestsListChrome, render_inbox, render_pull_requests, render_review_pane, render_sidebar};
use crate::review_session::SidebarTab;

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
    pub(crate) session: ReviewSession,
    provider: Arc<dyn Provider>,
    store: Arc<AppLocalStore>,
    _runtime: Arc<tokio::runtime::Runtime>,
    runtime: Handle,
    diff: Entity<DiffState>,
    inbox_scroll: UniformListScrollHandle,
    pull_requests_scroll: UniformListScrollHandle,
    files_scroll: UniformListScrollHandle,
    collapsed_groups: HashSet<String>,
    _inbox_reload_task: Task<()>,
    _inbox_sync_task: Task<()>,
    _inbox_poll_timer: Task<()>,
    _pull_requests_reload_task: Task<()>,
    _pull_requests_sync_task: Task<()>,
    _notification_read_task: Task<()>,
    _notification_done_task: Task<()>,
    _views_bootstrap_task: Task<()>,
    _viewer_login_task: Task<()>,
    _views_persist_task: Task<()>,
    _detail_task: Task<()>,
    _patch_task: Task<()>,
    _threads_task: Task<()>,
    _threads_write_task: Task<()>,
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
            pull_requests_scroll: UniformListScrollHandle::new(),
            files_scroll: UniformListScrollHandle::new(),
            collapsed_groups: HashSet::new(),
            _inbox_reload_task: Task::ready(()),
            _inbox_sync_task: Task::ready(()),
            _inbox_poll_timer: Task::ready(()),
            _pull_requests_reload_task: Task::ready(()),
            _pull_requests_sync_task: Task::ready(()),
            _notification_read_task: Task::ready(()),
            _notification_done_task: Task::ready(()),
            _views_bootstrap_task: Task::ready(()),
            _viewer_login_task: Task::ready(()),
            _views_persist_task: Task::ready(()),
            _detail_task: Task::ready(()),
            _patch_task: Task::ready(()),
            _threads_task: Task::ready(()),
            _threads_write_task: Task::ready(()),
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
                Effect::QueryPullRequests => self.run_query_pull_requests(cx),
                Effect::SyncPullRequests { generation } => {
                    self.run_sync_pull_requests(generation, cx)
                }
                Effect::FetchDetail { id } => self.run_fetch_detail(id, cx),
                Effect::FetchPatch { id } => self.run_fetch_patch(id, cx),
                Effect::FetchThreads { id } => self.run_fetch_threads(id, cx),
                Effect::RefreshThreads { id } => self.run_refresh_threads(id, cx),
                Effect::SaveDiffReviewComment { id, anchor, body } => {
                    self.run_save_diff_review_comment(id, anchor, body, cx)
                }
                Effect::SubmitPendingReviewWrite {
                    id,
                    event,
                    summary_body,
                } => self.run_submit_pending_review(id, event, summary_body, cx),
                Effect::DiscardPendingReviewWrite { id } => {
                    self.run_discard_pending_review(id, cx)
                }
                Effect::ArmPullRequestsRefreshTimer => self.arm_pull_requests_refresh_timer(cx),
                Effect::BootstrapPullRequestsViews => self.run_bootstrap_views(cx),
                Effect::FetchViewerLogin => self.run_fetch_viewer_login(cx),
                Effect::PersistViews {
                    views,
                    last_used,
                    scoped_repository,
                } => self.run_persist_views(views, last_used, scoped_repository, cx),
                Effect::QueryInbox => self.run_query_inbox(cx),
                Effect::SyncInbox {
                    generation,
                    if_modified_since,
                } => self.run_sync_inbox(generation, if_modified_since, cx),
                Effect::ArmInboxPollTimer { interval } => self.arm_inbox_poll_timer(interval, cx),
                Effect::MarkNotificationRead { id } => self.run_mark_notification_read(id, cx),
                Effect::MarkNotificationDone { id } => self.run_mark_notification_done(id, cx),
            }
        }
    }

    pub(crate) fn toggle_collapsed_group(&mut self, key: String, cx: &mut Context<Self>) {
        if self.collapsed_groups.contains(&key) {
            self.collapsed_groups.remove(&key);
        } else {
            self.collapsed_groups.insert(key);
        }
        cx.notify();
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

    fn run_query_pull_requests(&mut self, cx: &mut Context<Self>) {
        let store = self.store.clone();
        let runtime = self.runtime.clone();
        self._pull_requests_reload_task = cx.spawn(async move |this, cx| {
            let result = runtime
                .spawn(async move {
                    store
                        .query_pull_requests(&PullRequestListQuery::default())
                        .await
                })
                .await
                .unwrap_or_else(|e| Err(LocalStoreError::other(e.to_string())));
            let result = result.map_err(map_store_error);
            let _ = this.update(cx, |this, cx| {
                this.dispatch(Intent::PullRequestListQueryFinished { result }, cx);
            });
        });
    }

    fn run_sync_pull_requests(&mut self, generation: u64, cx: &mut Context<Self>) {
        let provider = self.provider.clone();
        let store = self.store.clone();
        let runtime = self.runtime.clone();
        let options = PullRequestListOptions {
            include_authored: true,
            drafts: DraftFilter::All,
        };
        self._pull_requests_sync_task = cx.spawn(async move |this, cx| {
            let result = runtime
                .spawn(async move {
                    sync_pull_requests_from_provider(provider.as_ref(), &store, &options).await
                })
                .await
                .unwrap_or_else(|e| Err(ProviderError::other(e.to_string())));
            let _ = this.update(cx, |this, cx| {
                this.dispatch(
                    Intent::PullRequestListSyncFinished { generation, result },
                    cx,
                );
            });
        });
    }

    fn run_bootstrap_views(&mut self, cx: &mut Context<Self>) {
        let store = self.store.clone();
        let runtime = self.runtime.clone();
        self._views_bootstrap_task = cx.spawn(async move |this, cx| {
            let result = runtime
                .spawn(async move {
                    let views = store.list_views().await.map_err(map_store_error);
                    let last_used = store.get_last_used_view().await.map_err(map_store_error);
                    let scoped = store.get_scoped_repository().await.map_err(map_store_error);
                    (views, last_used, scoped)
                })
                .await
                .unwrap_or_else(|e| {
                    (
                        Err(map_store_error(LocalStoreError::other(e.to_string()))),
                        Err(map_store_error(LocalStoreError::other("join"))),
                        Err(map_store_error(LocalStoreError::other("join"))),
                    )
                });
            let (views, last_used, scoped_repository) = result;
            let _ = this.update(cx, |this, cx| {
                this.dispatch(
                    Intent::PullRequestsViewsBootstrapFinished {
                        views,
                        last_used,
                        scoped_repository,
                    },
                    cx,
                );
            });
        });
    }

    fn run_fetch_viewer_login(&mut self, cx: &mut Context<Self>) {
        let provider = self.provider.clone();
        let runtime = self.runtime.clone();
        self._viewer_login_task = cx.spawn(async move |this, cx| {
            let result = runtime
                .spawn(async move { provider.current_user().await })
                .await
                .unwrap_or_else(|e| Err(ProviderError::other(e.to_string())));
            let _ = this.update(cx, |this, cx| {
                this.dispatch(Intent::ViewerLoginFinished { result }, cx);
            });
        });
    }

    fn run_persist_views(
        &mut self,
        views: Vec<View>,
        last_used: Option<reviewdesk_core::ViewId>,
        scoped_repository: Option<String>,
        cx: &mut Context<Self>,
    ) {
        let store = self.store.clone();
        let runtime = self.runtime.clone();
        self._views_persist_task = cx.spawn(async move |this, cx| {
            let result = runtime
                .spawn(async move {
                    store
                        .persist_views_preferences(
                            &views,
                            last_used.as_ref(),
                            scoped_repository.as_deref(),
                        )
                        .await
                })
                .await
                .unwrap_or_else(|e| Err(LocalStoreError::other(e.to_string())));
            let result = result.map_err(map_store_error);
            let _ = this.update(cx, |this, cx| {
                this.dispatch(Intent::ViewsPersistFinished { views: result }, cx);
            });
        });
    }

    fn run_query_inbox(&mut self, cx: &mut Context<Self>) {
        let store = self.store.clone();
        let runtime = self.runtime.clone();
        self._inbox_reload_task = cx.spawn(async move |this, cx| {
            let result = runtime
                .spawn(async move {
                    store
                        .query_notifications(&NotificationQuery::default())
                        .await
                })
                .await
                .unwrap_or_else(|e| Err(LocalStoreError::other(e.to_string())));
            let result = result.map_err(map_store_error);
            let _ = this.update(cx, |this, cx| {
                this.dispatch(Intent::InboxQueryFinished { result }, cx);
            });
        });
    }

    fn run_sync_inbox(
        &mut self,
        generation: u64,
        if_modified_since: Option<String>,
        cx: &mut Context<Self>,
    ) {
        let provider = self.provider.clone();
        let store = self.store.clone();
        let runtime = self.runtime.clone();
        let options = NotificationListOptions {
            all: false,
            if_modified_since,
        };
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

    fn arm_inbox_poll_timer(&mut self, interval: std::time::Duration, cx: &mut Context<Self>) {
        self._inbox_poll_timer = cx.spawn(async move |this, cx| {
            cx.background_executor().timer(interval).await;
            let _ = this.update(cx, |this, cx| {
                this.dispatch(Intent::RefreshInbox, cx);
            });
        });
    }

    fn run_mark_notification_read(&mut self, id: NotificationThreadId, cx: &mut Context<Self>) {
        let provider = self.provider.clone();
        let store = self.store.clone();
        let runtime = self.runtime.clone();
        let finish_id = id.clone();
        self._notification_read_task = cx.spawn(async move |this, cx| {
            let result = runtime
                .spawn(async move {
                    mark_notification_read_via_provider(provider.as_ref(), &store, &id).await
                })
                .await
                .unwrap_or_else(|e| Err(ProviderError::other(e.to_string())));
            let _ = this.update(cx, |this, cx| {
                this.dispatch(
                    Intent::NotificationReadFinished {
                        id: finish_id,
                        result,
                    },
                    cx,
                );
            });
        });
    }

    fn run_mark_notification_done(&mut self, id: NotificationThreadId, cx: &mut Context<Self>) {
        let provider = self.provider.clone();
        let store = self.store.clone();
        let runtime = self.runtime.clone();
        let finish_id = id.clone();
        self._notification_done_task = cx.spawn(async move |this, cx| {
            let result = runtime
                .spawn(async move {
                    mark_notification_done_via_provider(provider.as_ref(), &store, &id).await
                })
                .await
                .unwrap_or_else(|e| Err(ProviderError::other(e.to_string())));
            let _ = this.update(cx, |this, cx| {
                this.dispatch(
                    Intent::NotificationDoneFinished {
                        id: finish_id,
                        result,
                    },
                    cx,
                );
            });
        });
    }

    fn arm_pull_requests_refresh_timer(&mut self, cx: &mut Context<Self>) {
        self._refresh_timer = cx.spawn(async move |this, cx| {
            loop {
                cx.background_executor().timer(PULL_REQUESTS_REFRESH).await;
                if this
                    .update(cx, |this, cx| {
                        this.dispatch(Intent::RefreshPullRequests, cx);
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

    fn run_fetch_threads(&mut self, id: PullRequestId, cx: &mut Context<Self>) {
        let store = self.store.clone();
        let provider = self.provider.clone();
        let runtime = self.runtime.clone();
        let request_id = id;
        self._threads_task = cx.spawn(async move |this, cx| {
            let cache_id = request_id.clone();
            let cached = runtime
                .spawn({
                    let store = store.clone();
                    async move { store.get_pull_request_threads(&cache_id).await }
                })
                .await
                .unwrap_or_else(|e| Err(LocalStoreError::other(e.to_string())));
            let local_result = cached.map_err(map_store_error);
            let had_cache = matches!(&local_result, Ok(Some(_)));

            let _ = this.update(cx, |this, cx| {
                this.dispatch(
                    Intent::ThreadsLocalFinished {
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
                        refresh_threads_from_provider(provider.as_ref(), &store, &fetch_id).await
                    } else {
                        load_threads_via_store(provider.as_ref(), &store, &fetch_id).await
                    }
                })
                .await
                .unwrap_or_else(|e| Err(ProviderError::other(e.to_string())));

            let _ = this.update(cx, |this, cx| {
                this.dispatch(
                    Intent::ThreadsNetworkFinished {
                        id: request_id,
                        had_cache,
                        result,
                    },
                    cx,
                );
            });
        });
    }

    fn run_refresh_threads(&mut self, id: PullRequestId, cx: &mut Context<Self>) {
        let provider = self.provider.clone();
        let store = self.store.clone();
        let runtime = self.runtime.clone();
        let request_id = id;
        self._threads_task = cx.spawn(async move |this, cx| {
            let fetch_id = request_id.clone();
            let result = runtime
                .spawn(async move {
                    refresh_threads_from_provider(provider.as_ref(), &store, &fetch_id).await
                })
                .await
                .unwrap_or_else(|e| Err(ProviderError::other(e.to_string())));

            let _ = this.update(cx, |this, cx| {
                this.dispatch(
                    Intent::ThreadsNetworkFinished {
                        id: request_id,
                        had_cache: true,
                        result,
                    },
                    cx,
                );
            });
        });
    }

    fn run_save_diff_review_comment(
        &mut self,
        id: PullRequestId,
        anchor: DiffAnchor,
        body: String,
        cx: &mut Context<Self>,
    ) {
        let provider = self.provider.clone();
        let store = self.store.clone();
        let runtime = self.runtime.clone();
        let request_id = id;
        self._threads_write_task = cx.spawn(async move |this, cx| {
            let comment = PendingDiffReviewComment {
                reply_to_thread: None,
                anchor,
                body,
            };
            let write_id = request_id.clone();
            let result = runtime
                .spawn(async move {
                    add_pending_diff_review_comment(
                        provider.as_ref(),
                        &store,
                        &write_id,
                        comment,
                    )
                    .await
                })
                .await
                .unwrap_or_else(|e| Err(ProviderError::other(e.to_string())));

            let _ = this.update(cx, |this, cx| {
                this.dispatch(
                    Intent::ThreadsWriteFinished {
                        id: request_id,
                        result,
                    },
                    cx,
                );
            });
        });
    }

    fn run_submit_pending_review(
        &mut self,
        id: PullRequestId,
        event: ReviewSubmitEvent,
        summary_body: Option<String>,
        cx: &mut Context<Self>,
    ) {
        let provider = self.provider.clone();
        let store = self.store.clone();
        let runtime = self.runtime.clone();
        let request_id = id;
        self._threads_write_task = cx.spawn(async move |this, cx| {
            let write_id = request_id.clone();
            let result = runtime
                .spawn(async move {
                    submit_pending_review(
                        provider.as_ref(),
                        &store,
                        &write_id,
                        event,
                        summary_body,
                    )
                    .await
                })
                .await
                .unwrap_or_else(|e| Err(ProviderError::other(e.to_string())));

            let _ = this.update(cx, |this, cx| {
                this.dispatch(
                    Intent::ThreadsWriteFinished {
                        id: request_id,
                        result,
                    },
                    cx,
                );
            });
        });
    }

    fn run_discard_pending_review(&mut self, id: PullRequestId, cx: &mut Context<Self>) {
        let provider = self.provider.clone();
        let store = self.store.clone();
        let runtime = self.runtime.clone();
        let request_id = id;
        self._threads_write_task = cx.spawn(async move |this, cx| {
            let write_id = request_id.clone();
            let result = runtime
                .spawn(async move {
                    discard_pending_review(provider.as_ref(), &store, &write_id).await
                })
                .await
                .unwrap_or_else(|e| Err(ProviderError::other(e.to_string())));

            let _ = this.update(cx, |this, cx| {
                this.dispatch(
                    Intent::ThreadsWriteFinished {
                        id: request_id,
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
        let sync_label = view.pull_requests_pane.sync.status_label();

        v_flex()
            .size_full()
            .child(
                TitleBar::new().child(
                    h_flex()
                        .w_full()
                        .justify_between()
                        .items_center()
                        .pr_2()
                        .child(div().text_sm().font_medium().child("Review Desk"))
                        .child(
                            div()
                                .text_xs()
                                .text_color(cx.theme().muted_foreground)
                                .truncate()
                                .child(sync_label),
                        ),
                ),
            )
            .child(
                div().flex_1().min_h_0().child(
                    h_resizable("shell")
                        .child(
                            resizable_panel()
                                .size(px(244.))
                                .size_range(px(200.)..px(360.))
                                .child(render_sidebar(&view, entity.clone(), cx)),
                        )
                        .child({
                            let middle = if view.sidebar_tab == SidebarTab::Inbox {
                                render_inbox(&view, &self.inbox_scroll, entity.clone(), cx).into_any_element()
                            } else {
                                render_pull_requests(
                                    &view,
                                    PullRequestsListChrome {
                                        scroll: &self.pull_requests_scroll,
                                        collapsed_groups: &self.collapsed_groups,
                                    },
                                    entity.clone(),
                                    cx,
                                )
                                .into_any_element()
                            };
                            resizable_panel()
                                .size(px(360.))
                                .size_range(px(280.)..px(720.))
                                .child(middle)
                        })
                        .child(resizable_panel().size_range(px(480.)..px(2000.)).child(
                            render_review_pane(&view, &self.diff, &self.files_scroll, entity, cx),
                        )),
                ),
            )
    }
}
