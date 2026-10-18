//! PROTOTYPE paint: shell host + Pipelines tab variants. Throwaway with the branch.

use gpui_kit::component::diff::DiffState;
use gpui_kit::component::{
    ActiveTheme as _, Icon, IconName, Sizable as _, StyledExt as _,
    button::{Button, ButtonVariants as _},
    h_flex,
    list::ListItem,
    resizable::{h_resizable, resizable_panel, v_resizable},
    scroll::Scrollbar,
    tab::{Tab, TabBar},
    v_flex,
};
use gpui_kit::{
    Context, Entity, InteractiveElement as _, IntoElement, KeyDownEvent, ParentElement as _,
    SharedString, StatefulInteractiveElement as _, Styled as _, UniformListScrollHandle,
    WeakEntity, div, percentage, px, uniform_list,
    prelude::FluentBuilder as _,
};

use crate::app::ReviewDesk;
use crate::chrome_prototype::{ChromeVariant, PrototypeReviewTab, ShellNav};
use crate::pipelines_prototype::{PipelinesVariant, StubScenario};
use crate::review_session::{Intent, LoadState, ReviewTab, SessionView};
use crate::ui::{
    PipelinesPaint, render_compact_check_rollup, render_files, render_overview,
    render_pipelines_tab,
};

#[derive(Clone, Copy)]
struct StubNotification {
    kind: &'static str,
    title: &'static str,
    detail: &'static str,
}

const STUB_NOTIFICATIONS: &[StubNotification] = &[
    StubNotification {
        kind: "Review request",
        title: "alice requested your review",
        detail: "reviewdesk · #58 · Prototype primary shell",
    },
    StubNotification {
        kind: "Mention",
        title: "@you in Conversation",
        detail: "reviewdesk · #45 · ReviewSession kernel",
    },
    StubNotification {
        kind: "CI failed",
        title: "Checks failed on a watched pull request",
        detail: "reviewdesk · #43 · moon CI",
    },
    StubNotification {
        kind: "Merge",
        title: "Auto-merge enabled",
        detail: "reviewdesk · #39 · Zed performance",
    },
];

const STUB_METADATA_ROWS: &[(&str, &str)] = &[
    ("Author", "jacobdrury"),
    ("Reviewers", "alice · bob (required)"),
    ("CODEOWNERS", "@reviewdesk/core → bob"),
    ("Labels", "wayfinder:prototype"),
    ("Draft", "Ready for review"),
    ("Merge", "on Metadata · not Pipelines"),
];

pub(crate) struct PrototypeChrome {
    pub chrome: ChromeVariant,
    pub shell_nav: ShellNav,
    pub review_tab: PrototypeReviewTab,
    pub metadata_open: bool,
    pub pipelines_variant: PipelinesVariant,
    pub pipelines_scenario: StubScenario,
    pub pipeline_expanded: Vec<bool>,
    pub selected_pipeline: usize,
    pub view: SessionView,
    pub diff: Entity<DiffState>,
    pub inbox_scroll: UniformListScrollHandle,
    pub files_scroll: UniformListScrollHandle,
    pub notif_scroll: UniformListScrollHandle,
    pub pipelines_scroll: UniformListScrollHandle,
    pub pipelines_detail_scroll: UniformListScrollHandle,
    pub entity: WeakEntity<ReviewDesk>,
}

pub(crate) fn render_prototype_shell(
    proto: PrototypeChrome,
    cx: &mut Context<ReviewDesk>,
) -> impl IntoElement {
    let variant = proto.pipelines_variant;
    let scenario = proto.pipelines_scenario;
    let sync_label = proto.view.inbox_sync.status_label();

    v_flex()
        .size_full()
        .bg(cx.theme().background)
        .on_key_down(cx.listener(|this, event: &KeyDownEvent, _, cx| {
            let key = event.keystroke.key.as_str();
            if key == "[" || key == "left" {
                this.cycle_pipelines_variant(false, cx);
                return;
            }
            if key == "]" || key == "right" {
                this.cycle_pipelines_variant(true, cx);
                return;
            }
            if key == "," {
                this.cycle_pipelines_scenario(false, cx);
                return;
            }
            if key == "." {
                this.cycle_pipelines_scenario(true, cx);
            }
        }))
        .child(render_title_bar(&sync_label, cx))
        .child(
            div()
                .flex_1()
                .min_h_0()
                .child(render_body(proto, cx)),
        )
        .child(render_pipelines_switcher(variant, scenario, cx))
}

fn render_title_bar(sync_label: &str, cx: &mut Context<ReviewDesk>) -> impl IntoElement {
    gpui_kit::component::TitleBar::new().child(
        h_flex()
            .w_full()
            .items_center()
            .justify_between()
            .pr_2()
            .gap_3()
            .child(
                h_flex()
                    .items_center()
                    .gap_3()
                    .child(div().text_sm().font_medium().child("Review Desk"))
                    .child(
                        div()
                            .text_xs()
                            .text_color(cx.theme().muted_foreground)
                            .child("prototype pipelines"),
                    ),
            )
            .child(
                div()
                    .text_xs()
                    .text_color(cx.theme().muted_foreground)
                    .truncate()
                    .child(sync_label.to_string()),
            ),
    )
}

fn render_body(proto: PrototypeChrome, cx: &mut Context<ReviewDesk>) -> impl IntoElement {
    let chrome = proto.chrome;
    let pr_count = match &proto.view.inbox {
        LoadState::Ready(items) => items.len(),
        _ => 0,
    };

    if chrome.focus_stack() && proto.view.selected_id.is_some() {
        return h_flex()
            .size_full()
            .min_h_0()
            .child(render_linear_sidebar(
                proto.shell_nav,
                pr_count,
                chrome,
                cx,
            ))
            .child(
                div()
                    .flex_1()
                    .min_h_0()
                    .h_full()
                    .child(render_focus_review(proto, cx)),
            )
            .into_any_element();
    }

    h_flex()
        .size_full()
        .min_h_0()
        .child(render_linear_sidebar(
            proto.shell_nav,
            pr_count,
            chrome,
            cx,
        ))
        .child(
            h_flex()
                .flex_1()
                .min_h_0()
                .h_full()
                .child(
                    div()
                        .w(px(chrome.list_default_width()))
                        .flex_shrink_0()
                        .h_full()
                        .min_h_0()
                        .child(render_list_pane(&proto, cx)),
                )
                .child(
                    div()
                        .flex_1()
                        .min_w_0()
                        .min_h_0()
                        .h_full()
                        .child(render_review_stage(&proto, cx)),
                ),
        )
        .into_any_element()
}

fn render_focus_review(
    proto: PrototypeChrome,
    cx: &mut Context<ReviewDesk>,
) -> impl IntoElement {
    v_flex()
        .size_full()
        .min_h_0()
        .child(
            h_flex()
                .w_full()
                .px_3()
                .py(px(6.))
                .items_center()
                .gap_2()
                .border_b_1()
                .border_color(cx.theme().border)
                .child(
                    Button::new("focus-back")
                        .ghost()
                        .small()
                        .label("← Pull requests")
                        .on_click(cx.listener(|this, _, _, cx| {
                            this.clear_selected_pull_request(cx);
                        })),
                )
                .child(
                    div()
                        .text_sm()
                        .font_medium()
                        .truncate()
                        .child(selected_title(&proto.view)),
                ),
        )
        .child(div().flex_1().min_h_0().child(render_review_stage(&proto, cx)))
}

/// Linear-style primary nav: icon + label rows, selection pill, optional count.
fn render_linear_sidebar(
    shell_nav: ShellNav,
    pr_count: usize,
    chrome: ChromeVariant,
    cx: &mut Context<ReviewDesk>,
) -> impl IntoElement {
    let pad = if chrome.dense() { px(6.) } else { px(8.) };
    v_flex()
        .w(px(chrome.sidebar_width()))
        .flex_shrink_0()
        .h_full()
        .min_h_0()
        .gap_1()
        .px_2()
        .py(pad)
        .border_r_1()
        .border_color(cx.theme().border)
        .bg(cx.theme().sidebar)
        .child(
            div()
                .px_2()
                .py_1()
                .text_xs()
                .font_medium()
                .text_color(cx.theme().muted_foreground)
                .child("Review Desk"),
        )
        .child(linear_nav_row(
            "side-inbox",
            IconName::Inbox,
            ShellNav::Inbox,
            shell_nav,
            None,
            chrome.dense(),
            cx,
        ))
        .child(linear_nav_row(
            "side-prs",
            IconName::Github,
            ShellNav::PullRequests,
            shell_nav,
            Some(pr_count),
            chrome.dense(),
            cx,
        ))
}

fn linear_nav_row(
    id: &'static str,
    icon: IconName,
    target: ShellNav,
    current: ShellNav,
    badge: Option<usize>,
    dense: bool,
    cx: &mut Context<ReviewDesk>,
) -> impl IntoElement {
    let selected = target == current;
    let row_py = if dense { px(4.) } else { px(6.) };
    div()
        .id(id)
        .w_full()
        .px_2()
        .py(row_py)
        .rounded_md()
        .cursor_pointer()
        .when(selected, |el| el.bg(cx.theme().sidebar_accent))
        .when(!selected, |el| {
            el.hover(|style| style.bg(cx.theme().sidebar_accent.opacity(0.45)))
        })
        .on_click(cx.listener(move |this, _, _, cx| {
            this.set_shell_nav(target, cx);
        }))
        .child(
            h_flex()
                .w_full()
                .items_center()
                .gap_2()
                .child(
                    Icon::new(icon)
                        .xsmall()
                        .text_color(if selected {
                            cx.theme().sidebar_accent_foreground
                        } else {
                            cx.theme().sidebar_foreground
                        }),
                )
                .child(
                    div()
                        .flex_1()
                        .min_w_0()
                        .text_sm()
                        .truncate()
                        .text_color(if selected {
                            cx.theme().sidebar_accent_foreground
                        } else {
                            cx.theme().sidebar_foreground
                        })
                        .child(target.label()),
                )
                .when_some(badge.filter(|n| *n > 0), |el, count| {
                    el.child(
                        div()
                            .text_xs()
                            .text_color(cx.theme().muted_foreground)
                            .child(count.to_string()),
                    )
                }),
        )
}

fn render_list_pane(
    proto: &PrototypeChrome,
    cx: &mut Context<ReviewDesk>,
) -> impl IntoElement {
    match proto.shell_nav {
        ShellNav::Inbox => render_notifications(proto, cx).into_any_element(),
        ShellNav::PullRequests => {
            render_pull_requests(proto, cx).into_any_element()
        }
    }
}

fn render_pull_requests(
    proto: &PrototypeChrome,
    cx: &mut Context<ReviewDesk>,
) -> impl IntoElement {
    let chrome = proto.chrome;
    let header = if chrome.uppercase_labels() {
        "PULL REQUESTS"
    } else {
        "Pull requests"
    };
    let header_py = if chrome.dense() { px(4.) } else { px(8.) };
    let row_py = if chrome.dense() { px(4.) } else { px(8.) };
    let entity = proto.entity.clone();
    let scroll = proto.inbox_scroll.clone();

    v_flex()
        .size_full()
        .min_h_0()
        .border_r_1()
        .border_color(cx.theme().border)
        .child(
            h_flex()
                .w_full()
                .px_3()
                .py(header_py)
                .justify_between()
                .border_b_1()
                .border_color(cx.theme().border)
                .child(
                    div()
                        .text_xs()
                        .font_medium()
                        .text_color(cx.theme().muted_foreground)
                        .child(header),
                )
                .child(
                    Button::new("prs-refresh")
                        .ghost()
                        .xsmall()
                        .icon(IconName::RefreshCw)
                        .on_click(cx.listener(|this, _, _, cx| {
                            this.dispatch(Intent::RefreshInbox, cx);
                        })),
                ),
        )
        .child(
            div()
                .px_3()
                .py(px(2.))
                .border_b_1()
                .border_color(cx.theme().border)
                .child(
                    div()
                        .text_xs()
                        .text_color(cx.theme().muted_foreground)
                        .truncate()
                        .child(proto.view.inbox_sync.status_label()),
                ),
        )
        .child(match &proto.view.inbox {
            LoadState::Idle | LoadState::Loading => v_flex()
                .flex_1()
                .min_h_0()
                .items_center()
                .justify_center()
                .child(
                    div()
                        .text_sm()
                        .text_color(cx.theme().muted_foreground)
                        .child("Loading pull requests…"),
                )
                .into_any_element(),
            LoadState::Error(error) => v_flex()
                .flex_1()
                .min_h_0()
                .items_center()
                .justify_center()
                .gap_2()
                .px_4()
                .child(
                    div()
                        .text_sm()
                        .text_center()
                        .text_color(cx.theme().danger)
                        .child(error.to_string()),
                )
                .child(
                    Button::new("prs-retry")
                        .outline()
                        .small()
                        .label("Retry")
                        .on_click(cx.listener(|this, _, _, cx| {
                            this.dispatch(Intent::RefreshInbox, cx);
                        })),
                )
                .into_any_element(),
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
                let selected = proto.view.selected_id.clone();
                div()
                    .flex_1()
                    .min_h_0()
                    .relative()
                    .child(
                        uniform_list("proto-pr-list", items.len(), move |range, _, cx| {
                            range
                                .map(|ix| {
                                    let item = &items[ix];
                                    let id = item.id.clone();
                                    let selected = selected.as_ref() == Some(&item.id);
                                    let owner = entity.clone();
                                    ListItem::new(SharedString::from(item.id.as_str()))
                                        .selected(selected)
                                        .px_3()
                                        .py(row_py)
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
                                                this.dispatch(
                                                    Intent::SelectPullRequest(id),
                                                    cx,
                                                );
                                            });
                                        })
                                        .into_any_element()
                                })
                                .collect()
                        })
                        .size_full()
                        .track_scroll(&scroll),
                    )
                    .child(Scrollbar::vertical(&scroll))
                    .into_any_element()
            }
        })
}

fn render_notifications(
    proto: &PrototypeChrome,
    cx: &mut Context<ReviewDesk>,
) -> impl IntoElement {
    let chrome = proto.chrome;
    let header = if chrome.uppercase_labels() {
        "INBOX"
    } else {
        "Inbox"
    };
    let header_py = if chrome.dense() { px(4.) } else { px(8.) };
    let row_py = if chrome.dense() { px(4.) } else { px(8.) };
    let scroll = proto.notif_scroll.clone();
    let items = STUB_NOTIFICATIONS;

    v_flex()
        .size_full()
        .min_h_0()
        .border_r_1()
        .border_color(cx.theme().border)
        .child(
            h_flex()
                .w_full()
                .px_3()
                .py(header_py)
                .justify_between()
                .border_b_1()
                .border_color(cx.theme().border)
                .child(
                    div()
                        .text_xs()
                        .font_medium()
                        .text_color(cx.theme().muted_foreground)
                        .child(header),
                )
                .child(
                    div()
                        .text_xs()
                        .text_color(cx.theme().muted_foreground)
                        .child("stub · not GitHub yet"),
                ),
        )
        .child(
            div()
                .px_3()
                .py(px(4.))
                .border_b_1()
                .border_color(cx.theme().border)
                .child(
                    div()
                        .text_xs()
                        .text_color(cx.theme().muted_foreground)
                        .child("Real review requests live under Pull requests."),
                ),
        )
        .child(
            div()
                .flex_1()
                .min_h_0()
                .relative()
                .child(
                    uniform_list("proto-notifications", items.len(), move |range, _, cx| {
                        range
                            .map(|ix| {
                                let item = &items[ix];
                                ListItem::new(SharedString::from(format!("notif-{ix}")))
                                    .px_3()
                                    .py(row_py)
                                    .child(
                                        v_flex()
                                            .w_full()
                                            .gap_1()
                                            .child(
                                                div()
                                                    .text_xs()
                                                    .text_color(cx.theme().muted_foreground)
                                                    .child(item.kind),
                                            )
                                            .child(
                                                div()
                                                    .text_sm()
                                                    .font_medium()
                                                    .truncate()
                                                    .child(item.title),
                                            )
                                            .child(
                                                div()
                                                    .text_xs()
                                                    .text_color(cx.theme().muted_foreground)
                                                    .truncate()
                                                    .child(item.detail),
                                            ),
                                    )
                                    .into_any_element()
                            })
                            .collect()
                    })
                    .size_full()
                    .track_scroll(&scroll),
                )
                .child(Scrollbar::vertical(&scroll)),
        )
}

fn render_review_stage(
    proto: &PrototypeChrome,
    cx: &mut Context<ReviewDesk>,
) -> impl IntoElement {
    let chrome = proto.chrome;
    if proto.view.selected_id.is_none() {
        return empty_review(proto.shell_nav, cx).into_any_element();
    }

    let mut stage = v_flex().size_full().min_h_0();
    stage = stage.child(render_review_tabs(proto.review_tab, chrome.dense(), cx));

    if chrome.metadata_under_tabs() {
        stage = stage.child(render_metadata_band(
            proto,
            proto.metadata_open,
            chrome.dense(),
            cx,
        ));
    }

    let body = render_tab_body(proto, cx);
    if chrome.metadata_right_dock() {
        return stage
            .child(
                div().flex_1().min_h_0().child(
                    h_resizable("proto-review-meta")
                        .child(
                            resizable_panel()
                                .size_range(px(360.)..px(2000.))
                                .child(body),
                        )
                        .when(proto.metadata_open, |row| {
                            row.child(
                                resizable_panel()
                                    .size(px(chrome.metadata_default_width()))
                                    .size_range(px(200.)..px(420.))
                                    .child(render_metadata_panel(proto, chrome, true, cx)),
                            )
                        }),
                ),
            )
            .into_any_element();
    }

    if chrome.metadata_bottom_drawer() {
        return stage
            .child(
                div().flex_1().min_h_0().child(
                    v_resizable("proto-review-drawer")
                        .child(
                            resizable_panel()
                                .size_range(px(200.)..px(2000.))
                                .child(body),
                        )
                        .when(proto.metadata_open, |col| {
                            col.child(
                                resizable_panel()
                                    .size(px(160.))
                                    .size_range(px(120.)..px(320.))
                                    .child(render_metadata_panel(proto, chrome, false, cx)),
                            )
                        }),
                ),
            )
            .into_any_element();
    }

    stage
        .child(div().flex_1().min_h_0().child(body))
        .into_any_element()
}

fn empty_review(shell_nav: ShellNav, cx: &mut Context<ReviewDesk>) -> impl IntoElement {
    let hint = match shell_nav {
        ShellNav::Inbox => "Inbox is notifications (stub). Open a pull request from Pull requests.",
        ShellNav::PullRequests => "Select a pull request to open the Review pane.",
    };
    v_flex()
        .size_full()
        .items_center()
        .justify_center()
        .px_6()
        .child(
            div()
                .text_sm()
                .text_color(cx.theme().muted_foreground)
                .text_center()
                .child(hint),
        )
}

fn render_review_tabs(
    current: PrototypeReviewTab,
    dense: bool,
    cx: &mut Context<ReviewDesk>,
) -> impl IntoElement {
    let tab_ix = current.index();
    let pad_y = if dense { px(0.) } else { px(2.) };
    h_flex()
        .w_full()
        .items_center()
        .border_b_1()
        .border_color(cx.theme().border)
        .child(
            TabBar::new("proto-review-tabs")
                .flex_1()
                .px_2()
                .py(pad_y)
                .selected_index(tab_ix)
                .on_click(cx.listener(|this, ix: &usize, _, cx| {
                    this.set_prototype_tab(PrototypeReviewTab::from_index(*ix), cx);
                }))
                .child(Tab::new().label(PrototypeReviewTab::Overview.label()))
                .child(Tab::new().label(PrototypeReviewTab::Pipelines.label()))
                .child(Tab::new().label(PrototypeReviewTab::Conversation.label()))
                .child(Tab::new().label(PrototypeReviewTab::Changes.label())),
        )
        .child(
            Button::new("toggle-metadata")
                .ghost()
                .xsmall()
                .mr_2()
                .icon(IconName::PanelRight)
                .tooltip("Toggle Metadata")
                .on_click(cx.listener(|this, _, _, cx| {
                    this.toggle_metadata(cx);
                })),
        )
}

fn render_tab_body(
    proto: &PrototypeChrome,
    cx: &mut Context<ReviewDesk>,
) -> impl IntoElement {
    match proto.review_tab {
        PrototypeReviewTab::Overview => {
            render_overview(&proto.view, proto.entity.clone(), cx).into_any_element()
        }
        PrototypeReviewTab::Pipelines => render_pipelines_tab(
            PipelinesPaint {
                variant: proto.pipelines_variant,
                scenario: proto.pipelines_scenario,
                expanded: proto.pipeline_expanded.clone(),
                selected_pipeline: proto.selected_pipeline,
                list_scroll: proto.pipelines_scroll.clone(),
                detail_scroll: proto.pipelines_detail_scroll.clone(),
                entity: proto.entity.clone(),
            },
            cx,
        )
        .into_any_element(),
        PrototypeReviewTab::Conversation => stub_tab(
            "Conversation",
            &[
                "alice: Can we keep Dock Dense padding on the list?",
                "you: Metadata should stay open by default.",
                "bob: CODEOWNERS wants a re-request control here.",
                "(threaded replies stub)",
            ],
            cx,
        )
        .into_any_element(),
        PrototypeReviewTab::Changes => render_files(
            &proto.view,
            &proto.diff,
            &proto.files_scroll,
            proto.entity.clone(),
            cx,
        )
        .into_any_element(),
    }
}

fn stub_tab(
    title: &str,
    lines: &[&str],
    cx: &mut Context<ReviewDesk>,
) -> impl IntoElement {
    let lines = lines.to_vec();
    v_flex()
        .size_full()
        .min_h_0()
        .p_4()
        .gap_2()
        .child(
            div()
                .text_xs()
                .font_medium()
                .text_color(cx.theme().muted_foreground)
                .child(format!("{title} · stub")),
        )
        .children(lines.into_iter().map(|line| {
            div()
                .text_sm()
                .text_color(cx.theme().foreground)
                .child(line.to_string())
        }))
}

fn render_metadata_band(
    proto: &PrototypeChrome,
    open: bool,
    dense: bool,
    cx: &mut Context<ReviewDesk>,
) -> impl IntoElement {
    let py = if dense { px(4.) } else { px(8.) };
    let provider_link = provider_link_short_and_url(&proto.view);
    v_flex()
        .w_full()
        .border_b_1()
        .border_color(cx.theme().border)
        .bg(cx.theme().secondary.opacity(0.35))
        .child(
            h_flex()
                .w_full()
                .px_3()
                .py(py)
                .items_center()
                .justify_between()
                .id("metadata-band-toggle")
                .cursor_pointer()
                .on_click(cx.listener(|this, _, _, cx| {
                    this.toggle_metadata(cx);
                }))
                .child(
                    h_flex()
                        .items_center()
                        .gap_1()
                        .child(
                            Icon::new(IconName::ChevronRight)
                                .xsmall()
                                .text_color(cx.theme().muted_foreground)
                                .rotate(percentage(if open { 0.25 } else { 0. })),
                        )
                        .child(
                            div()
                                .text_xs()
                                .font_medium()
                                .text_color(cx.theme().muted_foreground)
                                .child(if dense { "METADATA" } else { "Metadata" }),
                        ),
                )
                .child(
                    div()
                        .text_xs()
                        .text_color(cx.theme().muted_foreground)
                        .child(if open { "open by default" } else { "hidden" }),
                ),
        )
        .when(open, |el| {
            el.child(
                h_flex()
                    .w_full()
                    .px_3()
                    .pb_2()
                    .gap_4()
                    .when_some(provider_link, |row, (short, url)| {
                        row.child(
                            v_flex()
                                .gap_0()
                                .child(
                                    div()
                                        .text_xs()
                                        .text_color(cx.theme().muted_foreground)
                                        .child("Link"),
                                )
                                .child(
                                    div()
                                        .id("metadata-band-provider-link")
                                        .text_sm()
                                        .text_color(cx.theme().link)
                                        .underline()
                                        .cursor_pointer()
                                        .child(short)
                                        .on_click(move |_, _, cx| {
                                            cx.open_url(&url);
                                        }),
                                ),
                        )
                    })
                    .children(STUB_METADATA_ROWS.iter().take(4).map(|(k, v)| {
                        v_flex()
                            .gap_0()
                            .child(
                                div()
                                    .text_xs()
                                    .text_color(cx.theme().muted_foreground)
                                    .child(*k),
                            )
                            .child(div().text_sm().child(*v))
                    })),
            )
        })
}

fn render_metadata_panel(
    proto: &PrototypeChrome,
    chrome: ChromeVariant,
    vertical: bool,
    cx: &mut Context<ReviewDesk>,
) -> impl IntoElement {
    let header = if chrome.uppercase_labels() {
        "METADATA"
    } else {
        "Metadata"
    };
    let header_py = if chrome.dense() { px(4.) } else { px(8.) };
    let provider_link = provider_link_short_and_url(&proto.view);
    let mut panel = v_flex().size_full().min_h_0();
    if vertical {
        panel = panel.border_l_1().border_color(cx.theme().border);
    } else {
        panel = panel
            .border_t_1()
            .border_color(cx.theme().border)
            .bg(cx.theme().secondary.opacity(0.25));
    }

    panel
        .child(
            h_flex()
                .w_full()
                .px_3()
                .py(header_py)
                .items_center()
                .justify_between()
                .border_b_1()
                .border_color(cx.theme().border)
                .child(
                    div()
                        .text_xs()
                        .font_medium()
                        .text_color(cx.theme().muted_foreground)
                        .child(header),
                )
                .child(
                    Button::new("metadata-hide")
                        .ghost()
                        .xsmall()
                        .icon(IconName::PanelRightClose)
                        .tooltip("Hide Metadata")
                        .on_click(cx.listener(|this, _, _, cx| {
                            this.toggle_metadata(cx);
                        })),
                ),
        )
        .child(
            v_flex()
                .flex_1()
                .min_h_0()
                .p_3()
                .gap_3()
                .child(render_compact_check_rollup(proto.pipelines_scenario, cx))
                .when_some(provider_link, |el, (short, url)| {
                    el.child(
                        v_flex()
                            .gap_0()
                            .child(
                                div()
                                    .text_xs()
                                    .text_color(cx.theme().muted_foreground)
                                    .child("Link"),
                            )
                            .child(
                                div()
                                    .id("metadata-provider-link")
                                    .text_sm()
                                    .text_color(cx.theme().link)
                                    .underline()
                                    .cursor_pointer()
                                    .child(short)
                                    .on_click(move |_, _, cx| {
                                        cx.open_url(&url);
                                    }),
                            ),
                    )
                })
                .children(STUB_METADATA_ROWS.iter().map(|(k, v)| {
                    v_flex()
                        .gap_0()
                        .child(
                            div()
                                .text_xs()
                                .text_color(cx.theme().muted_foreground)
                                .child(*k),
                        )
                        .child(div().text_sm().child(*v))
                }))
                .child(
                    Button::new("metadata-rerequest")
                        .outline()
                        .small()
                        .label("Re-request review (stub)")
                        .on_click(|_, _, _| {}),
                ),
        )
}

/// Short provider id (`owner/repo#n`) plus URL for the Metadata link row.
fn provider_link_short_and_url(view: &SessionView) -> Option<(String, String)> {
    let id = view.selected_id.as_ref()?;
    let short = id.as_str().to_string();
    if let LoadState::Ready(detail) = &view.detail
        && !detail.web_url.is_empty()
    {
        return Some((short, detail.web_url.clone()));
    }
    let url = github_url_from_short_id(&short).unwrap_or_else(|| short.clone());
    Some((short, url))
}

fn github_url_from_short_id(short: &str) -> Option<String> {
    let (path, number) = short.rsplit_once('#')?;
    if path.is_empty() || number.is_empty() {
        return None;
    }
    Some(format!("https://github.com/{path}/pull/{number}"))
}

fn render_pipelines_switcher(
    variant: PipelinesVariant,
    scenario: StubScenario,
    cx: &mut Context<ReviewDesk>,
) -> impl IntoElement {
    h_flex()
        .id("pipelines-prototype-switcher")
        .w_full()
        .justify_center()
        .gap_3()
        .py_2()
        .bg(cx.theme().background)
        .child(
            h_flex()
                .gap_2()
                .px_3()
                .py_1()
                .rounded_md()
                .border_1()
                .border_color(cx.theme().border)
                .bg(cx.theme().popover)
                .items_center()
                .child(
                    Button::new("pipelines-prev")
                        .ghost()
                        .xsmall()
                        .label("←")
                        .on_click(cx.listener(|this, _, _, cx| {
                            this.cycle_pipelines_variant(false, cx);
                        })),
                )
                .child(
                    div()
                        .min_w(px(200.))
                        .text_sm()
                        .text_center()
                        .font_medium()
                        .child(variant.label()),
                )
                .child(
                    Button::new("pipelines-next")
                        .ghost()
                        .xsmall()
                        .label("→")
                        .on_click(cx.listener(|this, _, _, cx| {
                            this.cycle_pipelines_variant(true, cx);
                        })),
                ),
        )
        .child(
            h_flex()
                .gap_2()
                .px_3()
                .py_1()
                .rounded_md()
                .border_1()
                .border_color(cx.theme().border)
                .bg(cx.theme().popover)
                .items_center()
                .child(
                    Button::new("scenario-prev")
                        .ghost()
                        .xsmall()
                        .label(",")
                        .on_click(cx.listener(|this, _, _, cx| {
                            this.cycle_pipelines_scenario(false, cx);
                        })),
                )
                .child(
                    div()
                        .min_w(px(160.))
                        .text_sm()
                        .text_center()
                        .child(format!("Scenario: {}", scenario.label())),
                )
                .child(
                    Button::new("scenario-next")
                        .ghost()
                        .xsmall()
                        .label(".")
                        .on_click(cx.listener(|this, _, _, cx| {
                            this.cycle_pipelines_scenario(true, cx);
                        })),
                ),
        )
}

fn selected_title(view: &SessionView) -> String {
    let LoadState::Ready(items) = &view.inbox else {
        return "Pull request".into();
    };
    let Some(id) = &view.selected_id else {
        return "Pull request".into();
    };
    items
        .iter()
        .find(|item| &item.id == id)
        .map(|item| item.title.clone())
        .unwrap_or_else(|| "Pull request".into())
}

/// Keep session Overview/Changes in sync when the prototype tab changes.
pub(crate) fn sync_session_tab(tab: PrototypeReviewTab) -> Option<ReviewTab> {
    match tab {
        PrototypeReviewTab::Overview => Some(ReviewTab::Overview),
        PrototypeReviewTab::Changes => Some(ReviewTab::Files),
        PrototypeReviewTab::Pipelines | PrototypeReviewTab::Conversation => None,
    }
}
