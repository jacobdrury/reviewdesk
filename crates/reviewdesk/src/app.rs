//! Thin GPUI shell: chrome, Effect driver, and Render over [`ReviewSession`].

use std::sync::Arc;

use gpui_kit::component::{
    ActiveTheme as _, StyledExt as _, TitleBar, WindowExt as _,
    diff::{DiffMode, DiffState},
    h_flex, v_flex,
};
use gpui_kit::{
    AppContext as _, Context, Entity, IntoElement, ParentElement as _, Render, Styled as _,
    Subscription, Task, UniformListScrollHandle, Window, WindowAppearance, div,
};
use reviewdesk_core::{
    LocalStoreError, NotificationListOptions, NotificationQuery, NotificationThreadId, Provider,
    ProviderError, PullRequestId, PullRequestListOptions, PullRequestListQuery,
};
use tokio::runtime::Handle;

use crate::PULL_REQUESTS_REFRESH;
use crate::review_session::{Effect, Intent, ReviewSession};
use crate::sync::{
    AppLocalStore, load_detail_via_store, load_patch_via_store, load_pipelines_via_store,
    mark_notification_done_via_provider, mark_notification_read_via_provider, refresh_patch,
    refresh_pipelines, refresh_pull_request_detail, rerun_check_via_store,
    sync_inbox_from_provider, sync_pull_requests_from_provider,
};
use crate::theme::{
    Appearance, ToastIntent, apply_sync_result, discover_macos, missing_theme_notification,
    should_sync_on_activation,
};
use crate::ui::{render_review_pane, render_sidebar};

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
    pull_requests_scroll: UniformListScrollHandle,
    files_scroll: UniformListScrollHandle,
    pipelines_scroll: UniformListScrollHandle,
    _inbox_reload_task: Task<()>,
    _inbox_sync_task: Task<()>,
    _inbox_poll_timer: Task<()>,
    _pull_requests_reload_task: Task<()>,
    _pull_requests_sync_task: Task<()>,
    _notification_read_task: Task<()>,
    _notification_done_task: Task<()>,
    _detail_task: Task<()>,
    _patch_task: Task<()>,
    _pipelines_task: Task<()>,
    _rerun_task: Task<()>,
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
            pipelines_scroll: UniformListScrollHandle::new(),
            _inbox_reload_task: Task::ready(()),
            _inbox_sync_task: Task::ready(()),
            _inbox_poll_timer: Task::ready(()),
            _pull_requests_reload_task: Task::ready(()),
            _pull_requests_sync_task: Task::ready(()),
            _notification_read_task: Task::ready(()),
            _notification_done_task: Task::ready(()),
            _detail_task: Task::ready(()),
            _patch_task: Task::ready(()),
            _pipelines_task: Task::ready(()),
            _rerun_task: Task::ready(()),
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
                Effect::FetchPipelines { id } => self.run_fetch_pipelines(id, cx),
                Effect::RerunPipeline { id, target } => self.run_rerun_pipeline(id, target, cx),
                Effect::ArmPullRequestsRefreshTimer => self.arm_pull_requests_refresh_timer(cx),
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
        let options = PullRequestListOptions::default();
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

    fn run_fetch_pipelines(&mut self, id: PullRequestId, cx: &mut Context<Self>) {
        let provider = self.provider.clone();
        let store = self.store.clone();
        let runtime = self.runtime.clone();
        let request_id = id;
        self._pipelines_task = cx.spawn(async move |this, cx| {
            let cache_id = request_id.clone();
            let cached = runtime
                .spawn({
                    let store = store.clone();
                    async move { store.get_pipelines(&cache_id).await }
                })
                .await
                .unwrap_or_else(|e| Err(LocalStoreError::other(e.to_string())));
            let local_result = cached.map_err(map_store_error);
            let had_cache = matches!(&local_result, Ok(Some(_)));

            let _ = this.update(cx, |this, cx| {
                this.dispatch(
                    Intent::PipelinesLocalFinished {
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
                        refresh_pipelines(provider.as_ref(), &store, &fetch_id).await
                    } else {
                        load_pipelines_via_store(provider.as_ref(), &store, &fetch_id).await
                    }
                })
                .await
                .unwrap_or_else(|e| Err(ProviderError::other(e.to_string())));

            let _ = this.update(cx, |this, cx| {
                this.dispatch(
                    Intent::PipelinesNetworkFinished {
                        id: request_id,
                        had_cache,
                        result,
                    },
                    cx,
                );
            });
        });
    }

    fn run_rerun_pipeline(
        &mut self,
        id: PullRequestId,
        target: reviewdesk_core::RerunTarget,
        cx: &mut Context<Self>,
    ) {
        let provider = self.provider.clone();
        let store = self.store.clone();
        let runtime = self.runtime.clone();
        let finish_id = id.clone();
        self._rerun_task = cx.spawn(async move |this, cx| {
            let result = runtime
                .spawn(async move {
                    rerun_check_via_store(provider.as_ref(), &store, &id, &target).await
                })
                .await
                .unwrap_or_else(|e| Err(ProviderError::other(e.to_string())));

            let _ = this.update(cx, |this, cx| {
                this.dispatch(
                    Intent::RerunPipelineFinished {
                        id: finish_id,
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
        let sync_label = view.pull_requests_sync.status_label();

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
                div()
                    .flex_1()
                    .min_h_0()
                    .child(if view.selected_id.is_some() {
                        render_review_pane(
                            &view,
                            &self.diff,
                            &self.files_scroll,
                            &self.pipelines_scroll,
                            entity,
                            cx,
                        )
                        .into_any_element()
                    } else {
                        render_sidebar(
                            &view,
                            &self.inbox_scroll,
                            &self.pull_requests_scroll,
                            entity,
                            cx,
                        )
                        .into_any_element()
                    }),
            )
    }
}
