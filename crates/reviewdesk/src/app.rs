use std::collections::HashMap;
use std::rc::Rc;
use std::sync::Arc;

use gpui_kit::component::{
    button::{Button, ButtonVariants as _},
    diff::{Diff, DiffAnnotation, DiffFile, DiffLinePosition, DiffLineRange, DiffMode, DiffState},
    h_flex,
    list::ListItem,
    resizable::{h_resizable, resizable_panel},
    scroll::Scrollbar,
    spinner::Spinner,
    tab::{Tab, TabBar},
    text::{markdown_ast, MarkdownNode, MarkdownParseContext, MarkdownPlugin, TextView},
    v_flex, ActiveTheme as _, Icon, IconName, Sizable as _, StyledExt as _, TitleBar,
    WindowExt as _,
};
use gpui_kit::{
    div, percentage, px, uniform_list, App, AppContext as _, Context, ElementId, Entity,
    InteractiveElement as _, IntoElement, ParentElement as _, Render, SharedString,
    StatefulInteractiveElement as _, Styled as _, Subscription, Task, UniformListScrollHandle,
    WeakEntity, Window, WindowAppearance,
};
use reviewdesk_core::{
    InboxItem, InboxOptions, Patch, Provider, ProviderError, PullRequestDetail, PullRequestId,
};
use tokio::runtime::Handle;

use crate::overview_body::{
    compose_overview_markdown, parse_body_segments, parse_details_fence, BodySegment, DetailsBlock,
    DetailsFence,
};
use crate::theme::{
    apply_sync_result, discover_macos, missing_theme_notification, should_sync_on_activation,
    Appearance, ToastIntent,
};
use crate::INBOX_REFRESH;

fn appearance_from_window(window: &Window) -> Appearance {
    appearance_from_system(window.appearance())
}

pub(crate) fn appearance_from_system(appearance: WindowAppearance) -> Appearance {
    match appearance {
        WindowAppearance::Dark | WindowAppearance::VibrantDark => Appearance::Dark,
        WindowAppearance::Light | WindowAppearance::VibrantLight => Appearance::Light,
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ReviewTab {
    Overview,
    Files,
}

#[derive(Clone)]
enum LoadState<T> {
    Idle,
    Loading,
    Ready(T),
    Error(ProviderError),
}

struct Annotation {
    id: ElementId,
    range: DiffLineRange,
    body: SharedString,
}

pub struct ReviewDesk {
    provider: Arc<dyn Provider>,
    _runtime: Arc<tokio::runtime::Runtime>,
    runtime: Handle,
    inbox: LoadState<Vec<InboxItem>>,
    selected_id: Option<PullRequestId>,
    review_tab: ReviewTab,
    detail: LoadState<PullRequestDetail>,
    patch_files: LoadState<Vec<DiffFile>>,
    selected_file: Option<SharedString>,
    diff: Entity<DiffState>,
    annotations: HashMap<String, Vec<Annotation>>,
    annotation_seq: usize,
    inbox_scroll: UniformListScrollHandle,
    files_scroll: UniformListScrollHandle,
    overview_segments: Vec<BodySegment>,
    details_open: HashMap<usize, bool>,
    _inbox_task: Task<()>,
    _detail_task: Task<()>,
    _patch_task: Task<()>,
    _refresh_timer: Task<()>,
    _window_activation: Option<Subscription>,
    skip_next_activation_sync: bool,
}

impl ReviewDesk {
    pub fn new(
        provider: Arc<dyn Provider>,
        runtime: Arc<tokio::runtime::Runtime>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Self {
        window.set_window_title("Review Desk");
        let diff = cx.new(|cx| DiffState::new([], cx).with_mode(DiffMode::Split));
        let mut this = Self {
            provider,
            runtime: runtime.handle().clone(),
            _runtime: runtime,
            inbox: LoadState::Idle,
            selected_id: None,
            review_tab: ReviewTab::Overview,
            detail: LoadState::Idle,
            patch_files: LoadState::Idle,
            selected_file: None,
            diff,
            annotations: HashMap::new(),
            annotation_seq: 0,
            inbox_scroll: UniformListScrollHandle::new(),
            files_scroll: UniformListScrollHandle::new(),
            overview_segments: Vec::new(),
            details_open: HashMap::new(),
            _inbox_task: Task::ready(()),
            _detail_task: Task::ready(()),
            _patch_task: Task::ready(()),
            _refresh_timer: Task::ready(()),
            _window_activation: None,
            skip_next_activation_sync: false,
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
        this.refresh_inbox(cx);
        this.start_inbox_timer(cx);
        this
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

    fn start_inbox_timer(&mut self, cx: &mut Context<Self>) {
        self._refresh_timer = cx.spawn(async move |this, cx| loop {
            cx.background_executor().timer(INBOX_REFRESH).await;
            if this
                .update(cx, |this, cx| {
                    this.refresh_inbox(cx);
                })
                .is_err()
            {
                break;
            }
        });
    }

    fn refresh_inbox(&mut self, cx: &mut Context<Self>) {
        self.inbox = LoadState::Loading;
        cx.notify();

        let provider = self.provider.clone();
        let runtime = self.runtime.clone();
        let options = InboxOptions::default();
        let previous = self.selected_id.clone();

        self._inbox_task = cx.spawn(async move |this, cx| {
            let result = runtime
                .spawn(async move { provider.list_inbox(&options).await })
                .await
                .unwrap_or_else(|e| Err(ProviderError::other(e.to_string())));

            let _ = this.update(cx, |this, cx| {
                match result {
                    Ok(items) => {
                        if let Some(selected) = &previous {
                            if !items.iter().any(|item| &item.id == selected) {
                                this.clear_selection(cx);
                            }
                        }
                        this.inbox = LoadState::Ready(items);
                    }
                    Err(error) => {
                        this.inbox = LoadState::Error(error);
                    }
                }
                cx.notify();
            });
        });
    }

    fn clear_selection(&mut self, cx: &mut Context<Self>) {
        self.selected_id = None;
        self.detail = LoadState::Idle;
        self.patch_files = LoadState::Idle;
        self.selected_file = None;
        self.overview_segments.clear();
        self.details_open.clear();
        self.diff.update(cx, |state, cx| state.set_files([], cx));
    }

    fn select_item(&mut self, id: PullRequestId, cx: &mut Context<Self>) {
        if self.selected_id.as_ref() == Some(&id) {
            return;
        }
        self.selected_id = Some(id.clone());
        self.review_tab = ReviewTab::Overview;
        self.detail = LoadState::Loading;
        self.patch_files = LoadState::Loading;
        self.selected_file = None;
        self.overview_segments.clear();
        self.details_open.clear();
        self.diff.update(cx, |state, cx| state.set_files([], cx));
        cx.notify();

        self.load_detail(id.clone(), cx);
        self.load_patch(id, cx);
    }

    fn details_is_open(&self, block: &DetailsBlock) -> bool {
        self.details_open
            .get(&block.id)
            .copied()
            .unwrap_or(block.default_open)
    }

    fn toggle_details(&mut self, id: usize, default_open: bool, cx: &mut Context<Self>) {
        let open = self.details_open.get(&id).copied().unwrap_or(default_open);
        self.details_open.insert(id, !open);
        cx.notify();
    }

    fn load_detail(&mut self, id: PullRequestId, cx: &mut Context<Self>) {
        let provider = self.provider.clone();
        let runtime = self.runtime.clone();
        let request_id = id.clone();
        self._detail_task = cx.spawn(async move |this, cx| {
            let fetch_id = request_id.clone();
            let result = runtime
                .spawn(async move { provider.fetch_pull_request(&fetch_id).await })
                .await
                .unwrap_or_else(|e| Err(ProviderError::other(e.to_string())));

            let _ = this.update(cx, |this, cx| {
                if this.selected_id.as_ref() != Some(&request_id) {
                    return;
                }
                this.details_open.clear();
                this.detail = match result {
                    Ok(detail) => {
                        this.overview_segments = parse_body_segments(&detail.body);
                        LoadState::Ready(detail)
                    }
                    Err(error) => {
                        this.overview_segments.clear();
                        LoadState::Error(error)
                    }
                };
                cx.notify();
            });
        });
    }

    fn load_patch(&mut self, id: PullRequestId, cx: &mut Context<Self>) {
        let provider = self.provider.clone();
        let runtime = self.runtime.clone();
        let request_id = id.clone();
        self._patch_task = cx.spawn(async move |this, cx| {
            let fetch_id = request_id.clone();
            let result = runtime
                .spawn(async move { provider.fetch_patch(&fetch_id).await })
                .await
                .unwrap_or_else(|e| Err(ProviderError::other(e.to_string())));

            let _ = this.update(cx, |this, cx| {
                if this.selected_id.as_ref() != Some(&request_id) {
                    return;
                }
                this.patch_files = match result {
                    Ok(patch) => match parse_patch(&patch) {
                        Ok(files) => {
                            if let Some(first) = files.first() {
                                let path = first.path().clone();
                                this.selected_file = Some(path.clone());
                                this.diff.update(cx, |state, cx| {
                                    state.set_files([first.clone()], cx);
                                });
                            }
                            LoadState::Ready(files)
                        }
                        Err(error) => LoadState::Error(error),
                    },
                    Err(error) => LoadState::Error(error),
                };
                cx.notify();
            });
        });
    }

    fn select_file(&mut self, path: SharedString, cx: &mut Context<Self>) {
        let LoadState::Ready(files) = &self.patch_files else {
            return;
        };
        let Some(file) = files.iter().find(|file| file.path() == &path).cloned() else {
            return;
        };
        self.selected_file = Some(path);
        self.diff
            .update(cx, |state, cx| state.set_files([file], cx));
        cx.notify();
    }

    fn set_review_tab(&mut self, tab: ReviewTab, cx: &mut Context<Self>) {
        self.review_tab = tab;
        cx.notify();
    }

    fn toggle_diff_mode(&mut self, cx: &mut Context<Self>) {
        self.diff.update(cx, |state, cx| {
            let next = match state.mode() {
                DiffMode::Split => DiffMode::Unified,
                DiffMode::Unified => DiffMode::Split,
                other => other,
            };
            state.set_mode(next, cx);
        });
        cx.notify();
    }

    fn add_annotation(&mut self, range: &DiffLineRange, cx: &mut Context<Self>) {
        let Some(pr_id) = self.selected_id.as_ref().map(|id| id.as_str().to_string()) else {
            return;
        };
        let (start, end) = (
            range.start().min(range.end()),
            range.start().max(range.end()),
        );
        let lines = if start == end {
            format!("line {end}")
        } else {
            format!("lines {start}-{end}")
        };
        self.annotation_seq += 1;
        let id = ElementId::from(SharedString::from(format!(
            "annotation-{}",
            self.annotation_seq
        )));
        self.annotations.entry(pr_id).or_default().push(Annotation {
            id,
            range: range.clone(),
            body: format!("Annotation on {lines}.").into(),
        });
        cx.notify();
    }

    fn current_annotations(&self) -> &[Annotation] {
        let Some(pr_id) = self.selected_id.as_ref() else {
            return &[];
        };
        self.annotations
            .get(pr_id.as_str())
            .map(|items| items.as_slice())
            .unwrap_or(&[])
    }

    fn render_inbox(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let entity = cx.entity().downgrade();
        v_flex()
            .size_full()
            .min_h_0()
            .border_r_1()
            .border_color(cx.theme().border)
            .child(
                h_flex()
                    .w_full()
                    .px_3()
                    .py(px(4.))
                    .justify_between()
                    .border_b_1()
                    .border_color(cx.theme().border)
                    .child(
                        div()
                            .text_xs()
                            .text_color(cx.theme().muted_foreground)
                            .font_medium()
                            .child("INBOX"),
                    )
                    .child(
                        Button::new("inbox-refresh")
                            .ghost()
                            .xsmall()
                            .icon(IconName::RefreshCw)
                            .on_click(cx.listener(|this, _, _, cx| this.refresh_inbox(cx))),
                    ),
            )
            .child(match &self.inbox {
                LoadState::Idle | LoadState::Loading => v_flex()
                    .flex_1()
                    .min_h_0()
                    .items_center()
                    .justify_center()
                    .gap_2()
                    .child(Spinner::new().small())
                    .child(
                        div()
                            .text_sm()
                            .text_color(cx.theme().muted_foreground)
                            .child("Loading Inbox…"),
                    )
                    .into_any_element(),
                LoadState::Error(error) => {
                    let message = error.to_string();
                    v_flex()
                        .flex_1()
                        .min_h_0()
                        .items_center()
                        .justify_center()
                        .gap_3()
                        .px_4()
                        .child(
                            div()
                                .text_sm()
                                .text_center()
                                .text_color(cx.theme().danger)
                                .child(message),
                        )
                        .child(
                            Button::new("inbox-retry")
                                .outline()
                                .small()
                                .label("Retry")
                                .on_click(cx.listener(|this, _, _, cx| {
                                    this.refresh_inbox(cx);
                                })),
                        )
                        .into_any_element()
                }
                LoadState::Ready(items) if items.is_empty() => v_flex()
                    .flex_1()
                    .min_h_0()
                    .items_center()
                    .justify_center()
                    .px_4()
                    .child(
                        div()
                            .text_sm()
                            .text_color(cx.theme().muted_foreground)
                            .text_center()
                            .child("No pull requests waiting for your review."),
                    )
                    .into_any_element(),
                LoadState::Ready(items) => {
                    let items = items.clone();
                    let selected = self.selected_id.clone();
                    div()
                        .flex_1()
                        .min_h_0()
                        .relative()
                        .child(
                            uniform_list("inbox-list", items.len(), move |range, _, cx| {
                                range
                                    .map(|ix| {
                                        let item = &items[ix];
                                        let id = item.id.clone();
                                        let selected = selected.as_ref() == Some(&item.id);
                                        let owner = entity.clone();
                                        ListItem::new(SharedString::from(item.id.as_str()))
                                            .selected(selected)
                                            .px_3()
                                            .py(px(4.))
                                            .child(
                                                v_flex()
                                                    .w_full()
                                                    .gap_1()
                                                    .child(
                                                        div()
                                                            .text_sm()
                                                            .font_medium()
                                                            .truncate()
                                                            .child(item.title.clone()),
                                                    )
                                                    .child(
                                                        div()
                                                            .text_xs()
                                                            .text_color(cx.theme().muted_foreground)
                                                            .truncate()
                                                            .child(format!(
                                                                "{} · {}",
                                                                item.repo_label, item.author_login
                                                            )),
                                                    ),
                                            )
                                            .on_click(move |_, _, cx| {
                                                let id = id.clone();
                                                let _ = owner.update(cx, |this, cx| {
                                                    this.select_item(id, cx);
                                                });
                                            })
                                            .into_any_element()
                                    })
                                    .collect()
                            })
                            .size_full()
                            .track_scroll(&self.inbox_scroll),
                        )
                        .child(Scrollbar::vertical(&self.inbox_scroll))
                        .into_any_element()
                }
            })
    }

    fn render_review_pane(&self, cx: &mut Context<Self>) -> impl IntoElement {
        if self.selected_id.is_none() {
            return v_flex()
                .size_full()
                .items_center()
                .justify_center()
                .child(
                    div()
                        .text_sm()
                        .text_color(cx.theme().muted_foreground)
                        .child("Select a pull request from the Inbox."),
                )
                .into_any_element();
        }

        let tab_ix = match self.review_tab {
            ReviewTab::Overview => 0,
            ReviewTab::Files => 1,
        };

        v_flex()
            .size_full()
            .min_h_0()
            .child(
                TabBar::new("review-tabs")
                    .w_full()
                    .px_2()
                    .border_b_1()
                    .border_color(cx.theme().border)
                    .selected_index(tab_ix)
                    .on_click(cx.listener(|this, ix: &usize, _, cx| {
                        let tab = if *ix == 0 {
                            ReviewTab::Overview
                        } else {
                            ReviewTab::Files
                        };
                        this.set_review_tab(tab, cx);
                    }))
                    .child(Tab::new().label("Overview"))
                    .child(Tab::new().label("Files")),
            )
            .child(
                div()
                    .flex_1()
                    .min_h_0()
                    .child(match self.review_tab {
                        ReviewTab::Overview => self.render_overview(cx).into_any_element(),
                        ReviewTab::Files => self.render_files(cx).into_any_element(),
                    }),
            )
            .into_any_element()
    }

    fn render_overview(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let entity = cx.entity().downgrade();
        match &self.detail {
            LoadState::Idle | LoadState::Loading => v_flex()
                .size_full()
                .min_h_0()
                .items_center()
                .justify_center()
                .gap_2()
                .child(Spinner::new().small())
                .child(
                    div()
                        .text_sm()
                        .text_color(cx.theme().muted_foreground)
                        .child("Loading overview…"),
                ),
            LoadState::Error(error) => {
                let id = self.selected_id.clone();
                v_flex()
                    .size_full()
                    .min_h_0()
                    .items_center()
                    .justify_center()
                    .gap_3()
                    .px_4()
                    .child(
                        div()
                            .text_sm()
                            .text_center()
                            .text_color(cx.theme().danger)
                            .child(error.to_string()),
                    )
                    .child(
                        Button::new("overview-retry")
                            .outline()
                            .small()
                            .label("Retry")
                            .on_click(move |_, _, cx| {
                                let Some(id) = id.clone() else {
                                    return;
                                };
                                let _ = entity.update(cx, |this, cx| {
                                    this.detail = LoadState::Loading;
                                    cx.notify();
                                    this.load_detail(id, cx);
                                });
                            }),
                    )
            }
            LoadState::Ready(detail) => {
                let reviewers = if detail.reviewers.is_empty() {
                    "None".to_string()
                } else {
                    detail
                        .reviewers
                        .iter()
                        .map(|r| r.login.as_str())
                        .collect::<Vec<_>>()
                        .join(", ")
                };
                let labels = if detail.labels.is_empty() {
                    "None".to_string()
                } else {
                    detail
                        .labels
                        .iter()
                        .map(|l| l.name.as_str())
                        .collect::<Vec<_>>()
                        .join(", ")
                };
                let is_open = |block: &DetailsBlock| self.details_is_open(block);
                let markdown = compose_overview_markdown(
                    &detail.title,
                    &detail.author_login,
                    detail.is_draft,
                    &detail.web_url,
                    &labels,
                    &reviewers,
                    &detail.body,
                    &self.overview_segments,
                    &is_open,
                );
                let owner = cx.entity().downgrade();
                div()
                    .size_full()
                    .min_h_0()
                    .p_4()
                    .child(
                        TextView::markdown("pr-overview", markdown)
                            .scrollable(true)
                            .text_sm()
                            .size_full()
                            .plugin(DetailsTogglePlugin { owner }),
                    )
            }
        }
    }

    fn render_files(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let entity = cx.entity().downgrade();
        match &self.patch_files {
            LoadState::Idle | LoadState::Loading => {
                return v_flex()
                    .size_full()
                    .items_center()
                    .justify_center()
                    .gap_2()
                    .child(Spinner::new().small())
                    .child(
                        div()
                            .text_sm()
                            .text_color(cx.theme().muted_foreground)
                            .child("Loading patch…"),
                    )
                    .into_any_element();
            }
            LoadState::Error(error) => {
                let id = self.selected_id.clone();
                return v_flex()
                    .size_full()
                    .items_center()
                    .justify_center()
                    .gap_3()
                    .px_4()
                    .child(
                        div()
                            .text_sm()
                            .text_center()
                            .text_color(cx.theme().danger)
                            .child(error.to_string()),
                    )
                    .child(
                        Button::new("patch-retry")
                            .outline()
                            .small()
                            .label("Retry")
                            .on_click(move |_, _, cx| {
                                let Some(id) = id.clone() else {
                                    return;
                                };
                                let _ = entity.update(cx, |this, cx| {
                                    this.patch_files = LoadState::Loading;
                                    cx.notify();
                                    this.load_patch(id, cx);
                                });
                            }),
                    )
                    .into_any_element();
            }
            LoadState::Ready(_) => {}
        }

        let files = match &self.patch_files {
            LoadState::Ready(files) => files.clone(),
            _ => unreachable!(),
        };
        let selected_file = self.selected_file.clone();
        let mode = self.diff.read(cx).mode();
        let mode_label = match mode {
            DiffMode::Split => "Unified",
            DiffMode::Unified => "Split",
            _ => "Toggle",
        };

        let annotations = self
            .current_annotations()
            .iter()
            .map(|annotation| {
                DiffAnnotation::line(
                    annotation.id.clone(),
                    DiffLinePosition::new(
                        annotation.range.path().clone(),
                        annotation.range.end_side(),
                        annotation.range.end(),
                    ),
                )
            })
            .collect::<Vec<_>>();
        let comments = Rc::new(
            self.current_annotations()
                .iter()
                .map(|a| (a.id.clone(), a.body.clone()))
                .collect::<Vec<_>>(),
        );
        let add_entity = entity.clone();

        div()
            .size_full()
            .min_h_0()
            .child(
                h_resizable("files-split")
                    .child(
                        resizable_panel()
                            .size(px(260.))
                            .size_range(px(180.)..px(420.))
                            .child(
                                v_flex()
                                    .size_full()
                                    .min_h_0()
                                    .border_r_1()
                                    .border_color(cx.theme().border)
                                    .child(
                                        h_flex()
                                            .w_full()
                                            .px_3()
                                            .py(px(4.))
                                            .justify_between()
                                            .border_b_1()
                                            .border_color(cx.theme().border)
                                            .child(
                                                div()
                                                    .text_xs()
                                                    .text_color(cx.theme().muted_foreground)
                                                    .font_medium()
                                                    .child("FILES"),
                                            )
                                            .child(
                                                Button::new("diff-mode")
                                                    .ghost()
                                                    .xsmall()
                                                    .label(mode_label)
                                                    .on_click(cx.listener(|this, _, _, cx| {
                                                        this.toggle_diff_mode(cx);
                                                    })),
                                            ),
                                    )
                                    .child({
                                        let files = files.clone();
                                        let selected_file = selected_file.clone();
                                        let file_entity = entity.clone();
                                        div()
                                            .flex_1()
                                            .min_h_0()
                                            .relative()
                                            .child(
                                                uniform_list(
                                                    "file-list",
                                                    files.len(),
                                                    move |range, _, _cx| {
                                                        range
                                                            .map(|ix| {
                                                                let file = &files[ix];
                                                                let path = file.path().clone();
                                                                let selected = selected_file
                                                                    .as_ref()
                                                                    == Some(&path);
                                                                let owner = file_entity.clone();
                                                                ListItem::new(path.clone())
                                                                    .selected(selected)
                                                                    .px_3()
                                                                    .py_2()
                                                                    .child(
                                                                        div()
                                                                            .text_sm()
                                                                            .truncate()
                                                                            .child(
                                                                                path.to_string(),
                                                                            ),
                                                                    )
                                                                    .on_click(move |_, _, cx| {
                                                                        let path = path.clone();
                                                                        let _ = owner.update(
                                                                            cx,
                                                                            |this, cx| {
                                                                                this.select_file(
                                                                                    path, cx,
                                                                                );
                                                                            },
                                                                        );
                                                                    })
                                                                    .into_any_element()
                                                            })
                                                            .collect()
                                                    },
                                                )
                                                .size_full()
                                                .track_scroll(&self.files_scroll),
                                            )
                                            .child(Scrollbar::vertical(&self.files_scroll))
                                    }),
                            ),
                    )
                    .child(
                        resizable_panel().child(
                            v_flex()
                                .size_full()
                                .min_h_0()
                                .min_w_0()
                                .p_2()
                                .child(
                                Diff::new(&self.diff)
                                    .size_full()
                                    .soft_wrap(true)
                                    .annotations(annotations)
                                    .render_annotation(move |annotation, _, cx| {
                                        let body = comments
                                            .iter()
                                            .find(|(id, _)| id == annotation.id())
                                            .map(|(_, body)| body.clone())
                                            .unwrap_or_default();
                                        v_flex()
                                            .w_full()
                                            .gap_1()
                                            .p_2()
                                            .rounded_md()
                                            .bg(cx.theme().secondary)
                                            .child(
                                                div()
                                                    .text_xs()
                                                    .text_color(cx.theme().muted_foreground)
                                                    .child("Annotation"),
                                            )
                                            .child(div().text_sm().child(body))
                                    })
                                    .on_add_annotation(move |range, _, _cx| {
                                        let _ = add_entity.update(_cx, |this, cx| {
                                            this.add_annotation(range, cx);
                                        });
                                    }),
                            ),
                        ),
                    ),
            )
            .into_any_element()
    }
}

struct DetailsTogglePlugin {
    owner: WeakEntity<ReviewDesk>,
}

impl MarkdownPlugin for DetailsTogglePlugin {
    fn is_block(&self) -> bool {
        true
    }

    fn name(&self) -> &str {
        "reviewdesk-details"
    }

    fn parse(&self, node: &markdown_ast::Node, _: &MarkdownParseContext<'_>) -> Option<MarkdownNode> {
        let markdown_ast::Node::Code(code) = node else {
            return None;
        };
        let fence = parse_details_fence(
            code.lang.as_deref(),
            code.meta.as_deref(),
            &code.value,
        )?;
        Some(
            MarkdownNode::new("reviewdesk-details", fence.clone())
                .text(fence.summary.clone())
                .markdown(fence.summary),
        )
    }

    fn render(&self, node: &MarkdownNode, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let fence = node
            .data::<DetailsFence>()
            .cloned()
            .expect("reviewdesk-details node data");
        let owner = self.owner.clone();
        let id = fence.id;
        let default_open = fence.default_open;
        let open = fence.open;
        div()
            .id(("pr-details-toggle", id))
            .h_flex()
            .w_full()
            .items_center()
            .gap_1()
            .py_1()
            .cursor_pointer()
            .rounded_sm()
            .hover(|style| style.bg(cx.theme().secondary_hover))
            .on_click(move |_, _, cx| {
                let _ = owner.update(cx, |this, cx| {
                    this.toggle_details(id, default_open, cx);
                });
            })
            .child(
                Icon::new(IconName::ChevronRight)
                    .xsmall()
                    .text_color(cx.theme().muted_foreground)
                    .rotate(percentage(if open { 0.25 } else { 0. })),
            )
            .child(
                div()
                    .text_sm()
                    .font_medium()
                    .min_w_0()
                    .child(fence.summary.clone()),
            )
    }
}

fn parse_patch(patch: &Patch) -> Result<Vec<DiffFile>, ProviderError> {
    DiffFile::parse(patch.as_str()).map_err(|error| {
        ProviderError::other(format!(
            "failed to parse patch at line {}: {}",
            error.line(),
            error.message()
        ))
    })
}

impl Render for ReviewDesk {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        v_flex()
            .size_full()
            .child(
                TitleBar::new().child(
                    div()
                        .text_sm()
                        .font_medium()
                        .child("Review Desk"),
                ),
            )
            .child(
                div().flex_1().min_h_0().child(
                    h_resizable("shell")
                        .child(
                            resizable_panel()
                                .size(px(240.))
                                .size_range(px(200.)..px(480.))
                                .child(self.render_inbox(cx)),
                        )
                        .child(
                            resizable_panel()
                                .size_range(px(480.)..px(2000.))
                                .child(self.render_review_pane(cx)),
                        ),
                ),
            )
    }
}
