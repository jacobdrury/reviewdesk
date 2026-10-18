//! PROTOTYPE paint: PR list in Views + left nav. Throwaway with the branch.
//!
//! Three variants of Views placement + list chrome on the #58 shell.
//! Switch with the bottom bar or `[` / `]`. Demo mode cycles empty/zero-match.

use gpui_kit::component::diff::DiffState;
use gpui_kit::component::{
    ActiveTheme as _, Icon, IconName, Sizable as _, StyledExt as _,
    button::{Button, ButtonVariants as _},
    h_flex,
    list::ListItem,
    resizable::{h_resizable, resizable_panel},
    scroll::Scrollbar,
    tab::{Tab, TabBar},
    v_flex,
};
use std::collections::HashSet;

use gpui_kit::{
    App, Context, Entity, InteractiveElement as _, IntoElement, KeyDownEvent, ParentElement as _,
    SharedString, StatefulInteractiveElement as _, Styled as _, UniformListScrollHandle,
    WeakEntity, div, px, uniform_list,
    prelude::FluentBuilder as _,
};

use crate::app::ReviewDesk;
use crate::chrome_prototype::{
    ChromeVariant, ListDemoMode, PrototypeReviewTab, ShellNav, StubViewId, STUB_REPOS,
};
use crate::review_session::{Intent, LoadState, ReviewTab, SessionView};
use crate::ui::{render_files, render_overview};

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
        detail: "reviewdesk · #71 · PR list + nav",
    },
    StubNotification {
        kind: "Mention",
        title: "@you in Conversation",
        detail: "reviewdesk · #58 · primary shell",
    },
    StubNotification {
        kind: "CI failed",
        title: "Checks failed on a watched pull request",
        detail: "reviewdesk · #43 · moon CI",
    },
];

const STUB_METADATA_ROWS: &[(&str, &str)] = &[
    ("Author", "jacobdrury"),
    ("Reviewers", "alice · bob (required)"),
    ("CODEOWNERS", "@reviewdesk/core → bob"),
    ("Labels", "wayfinder:prototype"),
    ("Draft", "Ready for review"),
    ("Checks", "3/4 passed · 1 failing"),
];

#[derive(Clone, Copy)]
struct StubPr {
    id: &'static str,
    number: u32,
    title: &'static str,
    repo: &'static str,
    author: &'static str,
    review_state: &'static str,
    ci: &'static str,
    updated: &'static str,
    labels: &'static [&'static str],
    is_draft: bool,
    i_am_reviewer: bool,
    i_am_author: bool,
}

const STUB_PRS: &[StubPr] = &[
    StubPr {
        id: "stub/reviewdesk#71",
        number: 71,
        title: "Prototype Pull requests list in Views + left nav",
        repo: "jacobdrury/reviewdesk",
        author: "jacobdrury",
        review_state: "Review required",
        ci: "Passing",
        updated: "2h",
        labels: &["wayfinder:prototype"],
        is_draft: false,
        i_am_reviewer: true,
        i_am_author: true,
    },
    StubPr {
        id: "stub/reviewdesk#58",
        number: 58,
        title: "Prototype primary shell chrome",
        repo: "jacobdrury/reviewdesk",
        author: "jacobdrury",
        review_state: "Approved",
        ci: "Passing",
        updated: "1d",
        labels: &["wayfinder:prototype", "shell"],
        is_draft: false,
        i_am_reviewer: false,
        i_am_author: true,
    },
    StubPr {
        id: "stub/reviewdesk#52",
        number: 52,
        title: "Grill Pull requests Views defaults",
        repo: "jacobdrury/reviewdesk",
        author: "alice",
        review_state: "Changes requested",
        ci: "Failing",
        updated: "3h",
        labels: &["wayfinder:grilling"],
        is_draft: false,
        i_am_reviewer: true,
        i_am_author: false,
    },
    StubPr {
        id: "stub/widgets#12",
        number: 12,
        title: "Add export CSV for invoices",
        repo: "acme/widgets",
        author: "bob",
        review_state: "Review required",
        ci: "Pending",
        updated: "4h",
        labels: &["billing"],
        is_draft: false,
        i_am_reviewer: true,
        i_am_author: false,
    },
    StubPr {
        id: "stub/widgets#9",
        number: 9,
        title: "Draft: redesign billing banner",
        repo: "acme/widgets",
        author: "jacobdrury",
        review_state: "Draft",
        ci: "—",
        updated: "5d",
        labels: &["design", "billing"],
        is_draft: true,
        i_am_reviewer: false,
        i_am_author: true,
    },
    StubPr {
        id: "stub/kit#3",
        number: 3,
        title: "Fix dense list row hit targets",
        repo: "acme/kit",
        author: "carol",
        review_state: "Approved",
        ci: "Passing",
        updated: "6h",
        labels: &["ui"],
        is_draft: false,
        i_am_reviewer: true,
        i_am_author: false,
    },
    StubPr {
        id: "stub/kit#1",
        number: 1,
        title: "WIP theme tokens for sidebar",
        repo: "acme/kit",
        author: "jacobdrury",
        review_state: "Draft",
        ci: "—",
        updated: "2d",
        labels: &["theme"],
        is_draft: true,
        i_am_reviewer: false,
        i_am_author: true,
    },
    StubPr {
        id: "stub/reviewdesk#45",
        number: 45,
        title: "Deepen ReviewDesk ReviewSession kernel",
        repo: "jacobdrury/reviewdesk",
        author: "alice",
        review_state: "Review required",
        ci: "Passing",
        updated: "8h",
        labels: &["core"],
        is_draft: false,
        i_am_reviewer: true,
        i_am_author: false,
    },
    StubPr {
        id: "stub/widgets#15",
        number: 15,
        title: "Connect Stripe webhook retries",
        repo: "acme/widgets",
        author: "carol",
        review_state: "Review required",
        ci: "Passing",
        updated: "1d",
        labels: &["billing", "infra"],
        is_draft: false,
        i_am_reviewer: true,
        i_am_author: false,
    },
    StubPr {
        id: "stub/kit#7",
        number: 7,
        title: "Invite flow empty states",
        repo: "acme/kit",
        author: "bob",
        review_state: "Approved",
        ci: "Passing",
        updated: "3d",
        labels: &["ui"],
        is_draft: false,
        i_am_reviewer: false,
        i_am_author: false,
    },
];

#[derive(Clone)]
enum ListRow {
    /// `depth` 0 = primary group (repo / status); 1 = nested (status under repo).
    Group {
        key: String,
        label: String,
        count: usize,
        depth: u8,
    },
    Pr { pr: StubPr },
}

pub(crate) struct PrototypeChrome {
    pub chrome: ChromeVariant,
    pub shell_nav: ShellNav,
    pub review_tab: PrototypeReviewTab,
    pub metadata_open: bool,
    pub selected_view: StubViewId,
    pub list_demo: ListDemoMode,
    pub selected_stub_id: Option<&'static str>,
    pub selected_repo: Option<&'static str>,
    pub collapsed_groups: HashSet<String>,
    pub view: SessionView,
    pub diff: Entity<DiffState>,
    pub inbox_scroll: UniformListScrollHandle,
    pub files_scroll: UniformListScrollHandle,
    pub notif_scroll: UniformListScrollHandle,
    pub entity: WeakEntity<ReviewDesk>,
}

pub(crate) fn render_prototype_shell(
    proto: PrototypeChrome,
    cx: &mut Context<ReviewDesk>,
) -> impl IntoElement {
    let chrome = proto.chrome;
    let sync_label = proto.view.inbox_sync.status_label();

    v_flex()
        .size_full()
        .bg(cx.theme().background)
        .on_key_down(cx.listener(|this, event: &KeyDownEvent, _, cx| {
            let key = event.keystroke.key.as_str();
            if key == "[" || key == "left" {
                this.cycle_chrome(false, cx);
                return;
            }
            if key == "]" || key == "right" {
                this.cycle_chrome(true, cx);
            }
        }))
        .child(render_title_bar(&sync_label, cx))
        .child(div().flex_1().min_h_0().child(render_body(proto, cx)))
        .child(render_chrome_switcher(chrome, cx))
}

fn render_title_bar(sync_label: &str, cx: &mut Context<ReviewDesk>) -> impl IntoElement {
    // Quiet chrome — Linear puts product identity in the sidebar, not a loud title.
    gpui_kit::component::TitleBar::new().child(
        h_flex()
            .w_full()
            .items_center()
            .justify_end()
            .pr_3()
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
    let notif_count = STUB_NOTIFICATIONS.len();

    // Focus Review (A/C) — list gone until Back.
    if chrome.focus_stack() && proto.selected_stub_id.is_some() {
        return h_flex()
            .size_full()
            .min_h_0()
            .child(render_sidebar(&proto, notif_count, cx))
            .child(
                div()
                    .flex_1()
                    .min_h_0()
                    .h_full()
                    .child(render_focus_review(&proto, cx)),
            )
            .into_any_element();
    }

    // Linear Issues (A): full-bleed list / Inbox; no empty Review column.
    if chrome.full_bleed_list() {
        return h_flex()
            .size_full()
            .min_h_0()
            .child(render_sidebar(&proto, notif_count, cx))
            .child(
                div()
                    .flex_1()
                    .min_w_0()
                    .min_h_0()
                    .h_full()
                    .child(match proto.shell_nav {
                        ShellNav::Inbox => render_notifications(&proto, cx).into_any_element(),
                        ShellNav::PullRequests => {
                            render_pull_requests(&proto, cx).into_any_element()
                        }
                    }),
            )
            .into_any_element();
    }

    h_flex()
        .size_full()
        .min_h_0()
        .child(render_sidebar(&proto, notif_count, cx))
        .child(
            h_flex()
                .flex_1()
                .min_h_0()
                .h_full()
                .child(render_main_column(&proto, cx))
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

fn render_main_column(
    proto: &PrototypeChrome,
    cx: &mut Context<ReviewDesk>,
) -> impl IntoElement {
    match proto.shell_nav {
        ShellNav::Inbox => div()
            .w(px(proto.chrome.list_default_width()))
            .flex_shrink_0()
            .h_full()
            .min_h_0()
            .child(render_notifications(proto, cx))
            .into_any_element(),
        ShellNav::PullRequests if proto.chrome.views_as_rail() => h_flex()
            .h_full()
            .min_h_0()
            .flex_shrink_0()
            .child(
                div()
                    .w(px(proto.chrome.views_rail_width()))
                    .flex_shrink_0()
                    .h_full()
                    .min_h_0()
                    .child(render_views_rail(proto, cx)),
            )
            .child(
                div()
                    .w(px(proto.chrome.list_default_width()))
                    .flex_shrink_0()
                    .h_full()
                    .min_h_0()
                    .child(render_pull_requests(proto, cx)),
            )
            .into_any_element(),
        ShellNav::PullRequests => div()
            .w(px(proto.chrome.list_default_width()))
            .flex_shrink_0()
            .h_full()
            .min_h_0()
            .child(render_pull_requests(proto, cx))
            .into_any_element(),
    }
}

fn render_focus_review(
    proto: &PrototypeChrome,
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
                            this.clear_selected_stub(cx);
                        })),
                )
                .child(
                    div()
                        .text_sm()
                        .font_medium()
                        .truncate()
                        .child(selected_stub_title(proto)),
                ),
        )
        .child(div().flex_1().min_h_0().child(render_review_stage(proto, cx)))
}

fn render_sidebar(
    proto: &PrototypeChrome,
    notif_count: usize,
    cx: &mut Context<ReviewDesk>,
) -> impl IntoElement {
    if proto.chrome.full_bleed_list() {
        return render_linear_sidebar(proto, notif_count, cx).into_any_element();
    }

    let chrome = proto.chrome;
    let pad = if chrome.dense() { px(6.) } else { px(8.) };
    let mut side = v_flex()
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
            proto.shell_nav,
            Some(notif_count),
            chrome.dense(),
            0.,
            cx,
        ));

    if chrome.views_in_sidebar() {
        side = side.child(section_label("Pull requests", chrome, cx));
        for view_id in StubViewId::ALL {
            side = side.child(nested_view_row(
                *view_id,
                proto.selected_view,
                proto.shell_nav,
                chrome,
                cx,
            ));
        }
    } else {
        side = side.child(linear_nav_row(
            "side-prs",
            IconName::Github,
            ShellNav::PullRequests,
            proto.shell_nav,
            Some(stub_count_for_view(
                proto.selected_view,
                proto.list_demo,
                proto.selected_repo,
            )),
            chrome.dense(),
            0.,
            cx,
        ));
    }

    side.into_any_element()
}

/// Linear sidebar: global Inbox / Pull requests, then per-repo scopes.
fn render_linear_sidebar(
    proto: &PrototypeChrome,
    notif_count: usize,
    cx: &mut Context<ReviewDesk>,
) -> impl IntoElement {
    let global_prs =
        proto.shell_nav == ShellNav::PullRequests && proto.selected_repo.is_none();
    let inbox_selected = proto.shell_nav == ShellNav::Inbox;

    let mut side = v_flex()
        .w(px(proto.chrome.sidebar_width()))
        .flex_shrink_0()
        .h_full()
        .min_h_0()
        .px_2()
        .py_2()
        .gap_0()
        .border_r_1()
        .border_color(cx.theme().border)
        .bg(cx.theme().sidebar)
        // Workspace header
        .child(
            h_flex()
                .w_full()
                .items_center()
                .gap_2()
                .px_2()
                .py(px(6.))
                .child(
                    div()
                        .size(px(18.))
                        .rounded_md()
                        .bg(cx.theme().accent.opacity(0.55))
                        .flex()
                        .items_center()
                        .justify_center()
                        .child(
                            div()
                                .text_xs()
                                .font_medium()
                                .text_color(cx.theme().foreground)
                                .child("R"),
                        ),
                )
                .child(
                    div()
                        .flex_1()
                        .min_w_0()
                        .text_sm()
                        .font_medium()
                        .truncate()
                        .child("Review Desk"),
                )
                .child(
                    Icon::new(IconName::Search)
                        .xsmall()
                        .text_color(cx.theme().muted_foreground),
                )
                .child(
                    Icon::new(IconName::Plus)
                        .xsmall()
                        .text_color(cx.theme().muted_foreground),
                ),
        )
        .child(div().h(px(6.)))
        .child(side_nav_item(
            "lin-inbox",
            IconName::Inbox,
            "Inbox",
            inbox_selected,
            Some(notif_count),
            0.,
            cx,
            SideNavTarget::Inbox,
        ))
        .child(side_nav_item(
            "lin-prs",
            IconName::Github,
            "Pull requests",
            global_prs,
            Some(stub_count_for_view(
                StubViewId::AllPrs,
                ListDemoMode::Populated,
                None,
            )),
            0.,
            cx,
            SideNavTarget::GlobalPrs,
        ))
        .child(div().h(px(10.)))
        .child(side_section("Workspace", cx))
        .child(side_static("lin-views", IconName::Eye, "Views", cx))
        .child(div().h(px(10.)))
        .child(side_section("Your repositories", cx));

    for repo in STUB_REPOS {
        let repo = *repo;
        let selected =
            proto.shell_nav == ShellNav::PullRequests && proto.selected_repo == Some(repo);
        let count = stub_count_for_view(
            StubViewId::AllPrs,
            ListDemoMode::Populated,
            Some(repo),
        );
        side = side.child(side_nav_item(
            SharedString::from(format!("repo-{repo}")),
            IconName::Folder,
            short_repo(repo),
            selected,
            Some(count),
            0.,
            cx,
            SideNavTarget::Repo(repo),
        ));
    }

    side
}

fn side_section(label: &str, cx: &mut Context<ReviewDesk>) -> impl IntoElement {
    div()
        .px_2()
        .py(px(4.))
        .text_xs()
        .font_medium()
        .text_color(cx.theme().muted_foreground)
        .child(label.to_string())
}

#[derive(Clone, Copy)]
enum SideNavTarget {
    Inbox,
    GlobalPrs,
    Repo(&'static str),
}

fn side_nav_item(
    id: impl Into<SharedString>,
    icon: IconName,
    label: &str,
    selected: bool,
    badge: Option<usize>,
    indent: f32,
    cx: &mut Context<ReviewDesk>,
    target: SideNavTarget,
) -> impl IntoElement {
    let label = label.to_string();
    // Soft selection — opaque sidebar_accent obliterates label text on some themes.
    let selected_bg = cx.theme().secondary.opacity(0.55);
    let hover_bg = cx.theme().secondary.opacity(0.35);
    let label_color = cx.theme().sidebar_foreground;
    let muted = cx.theme().muted_foreground;

    div()
        .id(id.into())
        .w_full()
        .pl(px(8. + indent))
        .pr_2()
        .py(px(5.))
        .rounded_md()
        .cursor_pointer()
        .when(selected, |el| el.bg(selected_bg))
        .when(!selected, |el| el.hover(move |style| style.bg(hover_bg)))
        .on_click(cx.listener(move |this, _, _, cx| match target {
            SideNavTarget::Inbox => this.set_shell_nav(ShellNav::Inbox, cx),
            SideNavTarget::GlobalPrs => this.set_selected_repo(None, cx),
            SideNavTarget::Repo(repo) => this.set_selected_repo(Some(repo), cx),
        }))
        .child(
            h_flex()
                .w_full()
                .items_center()
                .gap_2()
                .child(Icon::new(icon).xsmall().text_color(if selected {
                    label_color
                } else {
                    muted
                }))
                .child(
                    div()
                        .flex_1()
                        .min_w_0()
                        .text_sm()
                        .truncate()
                        .text_color(label_color)
                        .child(label),
                )
                .when_some(badge.filter(|n| *n > 0), |el, count| {
                    el.child(
                        div()
                            .text_xs()
                            .text_color(muted)
                            .child(count.to_string()),
                    )
                }),
        )
}

fn side_static(
    id: &'static str,
    icon: IconName,
    label: &'static str,
    cx: &mut Context<ReviewDesk>,
) -> impl IntoElement {
    div()
        .id(id)
        .w_full()
        .px_2()
        .py(px(5.))
        .rounded_md()
        .child(
            h_flex()
                .w_full()
                .items_center()
                .gap_2()
                .child(
                    Icon::new(icon)
                        .xsmall()
                        .text_color(cx.theme().muted_foreground),
                )
                .child(
                    div()
                        .text_sm()
                        .text_color(cx.theme().sidebar_foreground)
                        .child(label),
                ),
        )
}

fn section_label(
    text: &str,
    chrome: ChromeVariant,
    cx: &mut Context<ReviewDesk>,
) -> impl IntoElement {
    div()
        .px_2()
        .pt(if chrome.dense() { px(8.) } else { px(10.) })
        .pb(px(2.))
        .text_xs()
        .font_medium()
        .text_color(cx.theme().muted_foreground)
        .child(if chrome.uppercase_labels() {
            text.to_uppercase()
        } else {
            text.to_string()
        })
}

fn nested_view_row(
    view_id: StubViewId,
    selected: StubViewId,
    shell_nav: ShellNav,
    chrome: ChromeVariant,
    cx: &mut Context<ReviewDesk>,
) -> impl IntoElement {
    let active = shell_nav == ShellNav::PullRequests && selected == view_id;
    let row_py = if chrome.dense() { px(3.) } else { px(5.) };
    let count = stub_count_for_view(view_id, ListDemoMode::Populated, None);
    div()
        .id(SharedString::from(format!("view-{}", view_id.label())))
        .w_full()
        .pl_4()
        .pr_2()
        .py(row_py)
        .rounded_md()
        .cursor_pointer()
        .when(active, |el| el.bg(cx.theme().sidebar_accent))
        .when(!active, |el| {
            el.hover(|style| style.bg(cx.theme().sidebar_accent.opacity(0.45)))
        })
        .on_click(cx.listener(move |this, _, _, cx| {
            this.set_selected_view(view_id, cx);
            this.set_shell_nav(ShellNav::PullRequests, cx);
        }))
        .child(
            h_flex()
                .w_full()
                .items_center()
                .gap_2()
                .child(
                    div()
                        .flex_1()
                        .min_w_0()
                        .text_sm()
                        .truncate()
                        .text_color(if active {
                            cx.theme().sidebar_accent_foreground
                        } else {
                            cx.theme().sidebar_foreground
                        })
                        .child(view_id.label()),
                )
                .when(view_id.builtin(), |el| {
                    el.child(
                        div()
                            .text_xs()
                            .text_color(cx.theme().muted_foreground)
                            .child("builtin"),
                    )
                })
                .child(
                    div()
                        .text_xs()
                        .text_color(cx.theme().muted_foreground)
                        .child(count.to_string()),
                ),
        )
}

fn linear_nav_row(
    id: &'static str,
    icon: IconName,
    target: ShellNav,
    current: ShellNav,
    badge: Option<usize>,
    dense: bool,
    _indent: f32,
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

fn render_views_rail(
    proto: &PrototypeChrome,
    cx: &mut Context<ReviewDesk>,
) -> impl IntoElement {
    let chrome = proto.chrome;
    v_flex()
        .size_full()
        .min_h_0()
        .border_r_1()
        .border_color(cx.theme().border)
        .bg(cx.theme().sidebar.opacity(0.55))
        .child(
            h_flex()
                .w_full()
                .px_3()
                .py(px(6.))
                .items_center()
                .justify_between()
                .border_b_1()
                .border_color(cx.theme().border)
                .child(
                    div()
                        .text_xs()
                        .font_medium()
                        .text_color(cx.theme().muted_foreground)
                        .child(if chrome.uppercase_labels() {
                            "VIEWS"
                        } else {
                            "Views"
                        }),
                )
                .child(
                    Button::new("new-view-stub")
                        .ghost()
                        .xsmall()
                        .icon(IconName::Plus)
                        .tooltip("New View (stub)"),
                ),
        )
        .child({
            let mut rail = v_flex().flex_1().min_h_0().p_2().gap_1();
            for view_id in StubViewId::ALL {
                let view_id = *view_id;
                let active = proto.selected_view == view_id;
                let count = stub_count_for_view(view_id, proto.list_demo, None);
                rail = rail.child(
                    div()
                        .id(SharedString::from(format!("rail-{}", view_id.label())))
                        .w_full()
                        .px_2()
                        .py(px(5.))
                        .rounded_md()
                        .cursor_pointer()
                        .when(active, |el| el.bg(cx.theme().sidebar_accent))
                        .when(!active, |el| {
                            el.hover(|style| style.bg(cx.theme().sidebar_accent.opacity(0.45)))
                        })
                        .on_click(cx.listener(move |this, _, _, cx| {
                            this.set_selected_view(view_id, cx);
                        }))
                        .child(
                            v_flex()
                                .w_full()
                                .gap_0()
                                .child(
                                    h_flex()
                                        .w_full()
                                        .items_center()
                                        .justify_between()
                                        .child(
                                            div()
                                                .text_sm()
                                                .truncate()
                                                .text_color(if active {
                                                    cx.theme().sidebar_accent_foreground
                                                } else {
                                                    cx.theme().sidebar_foreground
                                                })
                                                .child(view_id.label()),
                                        )
                                        .child(
                                            div()
                                                .text_xs()
                                                .text_color(cx.theme().muted_foreground)
                                                .child(count.to_string()),
                                        ),
                                )
                                .child(
                                    div()
                                        .text_xs()
                                        .text_color(cx.theme().muted_foreground)
                                        .truncate()
                                        .child(view_id.filter_summary()),
                                ),
                        ),
                );
            }
            rail
        })
        .child(
            div()
                .px_3()
                .py_2()
                .border_t_1()
                .border_color(cx.theme().border)
                .child(
                    div()
                        .text_xs()
                        .text_color(cx.theme().muted_foreground)
                        .child("local-only · drag to reorder (stub)"),
                ),
        )
}

fn render_pull_requests(
    proto: &PrototypeChrome,
    cx: &mut Context<ReviewDesk>,
) -> impl IntoElement {
    let chrome = proto.chrome;
    let view = proto.selected_view;

    let mut pane = v_flex().size_full().min_h_0();
    if !chrome.full_bleed_list() {
        pane = pane.border_r_1().border_color(cx.theme().border);
    }

    if chrome.views_as_tabs() {
        pane = pane
            .child(render_linear_view_tabs(proto, cx))
            .child(render_linear_filter_display(proto, cx));
    } else {
        let header_py = if chrome.dense() { px(4.) } else { px(8.) };
        pane = pane.child(
            h_flex()
                .w_full()
                .px_3()
                .py(header_py)
                .items_center()
                .justify_between()
                .gap_2()
                .border_b_1()
                .border_color(cx.theme().border)
                .child(
                    v_flex()
                        .min_w_0()
                        .gap_0()
                        .child(
                            div()
                                .text_sm()
                                .font_medium()
                                .truncate()
                                .child(view.label()),
                        )
                        .child(
                            div()
                                .text_xs()
                                .text_color(cx.theme().muted_foreground)
                                .truncate()
                                .child(view.filter_summary()),
                        ),
                )
                .child(
                    Button::new("demo-mode")
                        .ghost()
                        .xsmall()
                        .label(proto.list_demo.label())
                        .tooltip("Cycle populated / empty store / zero-match")
                        .on_click(cx.listener(|this, _, _, cx| {
                            this.cycle_list_demo(cx);
                        })),
                ),
        );
        if chrome.views_as_chips() {
            pane = pane.child(render_view_chips(proto, cx));
        }
        pane = pane.child(render_filter_sort_chrome(proto, cx));
    }

    match proto.list_demo {
        ListDemoMode::EmptyStore => pane
            .child(empty_store_state(cx))
            .into_any_element(),
        ListDemoMode::ZeroMatch => pane
            .child(zero_match_state(view, cx))
            .into_any_element(),
        ListDemoMode::Populated => {
            let rows = build_list_rows(view, proto.selected_repo);
            let rows = visible_list_rows(rows, &proto.collapsed_groups);
            if rows.is_empty() {
                return pane.child(zero_match_state(view, cx)).into_any_element();
            }
            pane.child(render_pr_rows(proto, rows, cx))
                .into_any_element()
        }
    }
}

/// Linear top tabs: All issues / Active / Backlog → our Views.
fn render_linear_view_tabs(
    proto: &PrototypeChrome,
    cx: &mut Context<ReviewDesk>,
) -> impl IntoElement {
    let mut tabs = h_flex()
        .w_full()
        .px_4()
        .pt_2()
        .gap_0()
        .items_end()
        .border_b_1()
        .border_color(cx.theme().border);
    for view_id in StubViewId::ALL {
        let view_id = *view_id;
        let active = proto.selected_view == view_id;
        tabs = tabs.child(
            div()
                .id(SharedString::from(format!("tab-{}", view_id.label())))
                .px_3()
                .pb(px(8.))
                .pt(px(4.))
                .cursor_pointer()
                .border_b_2()
                .border_color(if active {
                    cx.theme().foreground
                } else {
                    cx.theme().border.opacity(0.)
                })
                .on_click(cx.listener(move |this, _, _, cx| {
                    this.set_selected_view(view_id, cx);
                }))
                .child(
                    div()
                        .text_sm()
                        .font_medium()
                        .text_color(if active {
                            cx.theme().foreground
                        } else {
                            cx.theme().muted_foreground
                        })
                        .child(view_id.label()),
                ),
        );
    }
    tabs.child(div().flex_1()).child(
        div()
            .pb(px(6.))
            .child(
                Button::new("demo-mode-tabs")
                    .ghost()
                    .xsmall()
                    .label(proto.list_demo.label())
                    .tooltip("Cycle populated / empty store / zero-match")
                    .on_click(cx.listener(|this, _, _, cx| {
                        this.cycle_list_demo(cx);
                    })),
            ),
    )
}

/// Linear Filter (left) + Display (right) — not a full search field.
fn render_linear_filter_display(
    proto: &PrototypeChrome,
    cx: &mut Context<ReviewDesk>,
) -> impl IntoElement {
    let view = proto.selected_view;
    let group = match view.group_by() {
        Some("repo→status") => "project → status",
        Some(g) => g,
        None => "Status",
    };
    let filter_label = match proto.selected_repo {
        Some(repo) => format!("repo:{repo} · {}", view.filter_summary()),
        None => view.filter_summary().to_string(),
    };
    h_flex()
        .w_full()
        .px_4()
        .py_2()
        .items_center()
        .gap_2()
        .border_b_1()
        .border_color(cx.theme().border)
        .child(
            h_flex()
                .items_center()
                .gap_1()
                .px_2()
                .py(px(4.))
                .rounded_md()
                .cursor_pointer()
                .hover(|style| style.bg(cx.theme().secondary.opacity(0.5)))
                .child(
                    Icon::new(IconName::Menu)
                        .xsmall()
                        .text_color(cx.theme().muted_foreground),
                )
                .child(
                    div()
                        .text_sm()
                        .text_color(cx.theme().muted_foreground)
                        .child("Filter"),
                ),
        )
        .child(
            div()
                .flex_1()
                .min_w_0()
                .text_xs()
                .text_color(cx.theme().muted_foreground)
                .truncate()
                .child(filter_label),
        )
        .child(
            h_flex()
                .items_center()
                .gap_1()
                .px_2()
                .py(px(4.))
                .rounded_md()
                .cursor_pointer()
                .hover(|style| style.bg(cx.theme().secondary.opacity(0.5)))
                .child(
                    Icon::new(IconName::Settings2)
                        .xsmall()
                        .text_color(cx.theme().muted_foreground),
                )
                .child(
                    div()
                        .text_sm()
                        .text_color(cx.theme().muted_foreground)
                        .child("Display"),
                )
                .child(
                    div()
                        .text_xs()
                        .text_color(cx.theme().muted_foreground)
                        .child(format!("· {group}")),
                ),
        )
}

fn render_view_chips(
    proto: &PrototypeChrome,
    cx: &mut Context<ReviewDesk>,
) -> impl IntoElement {
    let mut chips = h_flex()
        .w_full()
        .px_2()
        .py_2()
        .gap_1()
        .border_b_1()
        .border_color(cx.theme().border);
    for view_id in StubViewId::ALL {
        let view_id = *view_id;
        let active = proto.selected_view == view_id;
        chips = chips.child(
            div()
                .id(SharedString::from(format!("chip-{}", view_id.label())))
                .px_2()
                .py(px(3.))
                .rounded_md()
                .border_1()
                .border_color(if active {
                    cx.theme().border
                } else {
                    cx.theme().border.opacity(0.5)
                })
                .bg(if active {
                    cx.theme().accent.opacity(0.35)
                } else {
                    cx.theme().background
                })
                .cursor_pointer()
                .on_click(cx.listener(move |this, _, _, cx| {
                    this.set_selected_view(view_id, cx);
                }))
                .child(
                    div()
                        .text_xs()
                        .font_medium()
                        .text_color(cx.theme().foreground)
                        .child(view_id.label()),
                ),
        );
    }
    chips.child(
        Button::new("chip-new-view")
            .ghost()
            .xsmall()
            .icon(IconName::Plus)
            .tooltip("Save as new View (stub)"),
    )
}

fn render_filter_sort_chrome(
    proto: &PrototypeChrome,
    cx: &mut Context<ReviewDesk>,
) -> impl IntoElement {
    let view = proto.selected_view;
    let group = view.group_by().unwrap_or("None");
    let chrome = proto.chrome;

    if chrome.views_as_chips() {
        return h_flex()
            .w_full()
            .px_2()
            .py_2()
            .gap_1()
            .border_b_1()
            .border_color(cx.theme().border)
            .child(filter_chip("Filter", view.filter_summary(), cx))
            .child(filter_chip("Group", group, cx))
            .child(filter_chip("Sort", view.sort_label(), cx))
            .child(
                div()
                    .flex_1()
                    .min_w_0()
                    .text_xs()
                    .text_color(cx.theme().muted_foreground)
                    .truncate()
                    .child("unsaved edits → Discard / Save (stub)"),
            )
            .into_any_element();
    }

    h_flex()
        .w_full()
        .px_3()
        .py(px(4.))
        .items_center()
        .gap_2()
        .border_b_1()
        .border_color(cx.theme().border)
        .child(
            Icon::new(IconName::Search)
                .xsmall()
                .text_color(cx.theme().muted_foreground),
        )
        .child(
            div()
                .flex_1()
                .min_w_0()
                .text_xs()
                .text_color(cx.theme().muted_foreground)
                .truncate()
                .child(format!("Group: {group} · {}", view.sort_label())),
        )
        .child(
            Button::new("sort-stub")
                .ghost()
                .xsmall()
                .icon(IconName::SortDescending)
                .tooltip("Sort (stub)"),
        )
        .into_any_element()
}

fn filter_chip(
    label: &str,
    value: &str,
    cx: &mut Context<ReviewDesk>,
) -> impl IntoElement {
    div()
        .px_2()
        .py(px(3.))
        .rounded_md()
        .border_1()
        .border_color(cx.theme().border)
        .child(
            h_flex()
                .items_center()
                .gap_1()
                .child(
                    div()
                        .text_xs()
                        .text_color(cx.theme().muted_foreground)
                        .child(label.to_string()),
                )
                .child(
                    div()
                        .text_xs()
                        .font_medium()
                        .truncate()
                        .child(value.to_string()),
                ),
        )
}

fn empty_store_state(cx: &mut Context<ReviewDesk>) -> impl IntoElement {
    v_flex()
        .flex_1()
        .min_h_0()
        .items_center()
        .justify_center()
        .gap_3()
        .px_6()
        .child(
            div()
                .text_sm()
                .font_medium()
                .text_center()
                .child("No pull requests yet"),
        )
        .child(
            div()
                .text_xs()
                .text_color(cx.theme().muted_foreground)
                .text_center()
                .child("Sync from GitHub or check auth. This is the empty-store state (not a filter miss)."),
        )
        .child(
            Button::new("empty-sync")
                .outline()
                .small()
                .label("Sync now (stub)")
                .on_click(cx.listener(|this, _, _, cx| {
                    this.dispatch(Intent::RefreshInbox, cx);
                })),
        )
}

fn zero_match_state(view: StubViewId, cx: &mut Context<ReviewDesk>) -> impl IntoElement {
    v_flex()
        .flex_1()
        .min_h_0()
        .items_center()
        .justify_center()
        .gap_3()
        .px_6()
        .child(
            div()
                .text_sm()
                .font_medium()
                .text_center()
                .child("No pull requests in this View"),
        )
        .child(
            div()
                .text_xs()
                .text_color(cx.theme().muted_foreground)
                .text_center()
                .child(format!(
                    "“{}” filters match nothing. Clear filters or switch View.",
                    view.label()
                )),
        )
        .child(
            h_flex()
                .gap_2()
                .child(
                    Button::new("clear-filters")
                        .outline()
                        .small()
                        .label("Clear filters")
                        .on_click(cx.listener(|this, _, _, cx| {
                            this.set_selected_view(StubViewId::AllPrs, cx);
                            this.set_list_demo(ListDemoMode::Populated, cx);
                        })),
                )
                .child(
                    Button::new("switch-view")
                        .ghost()
                        .small()
                        .label("Switch View")
                        .on_click(cx.listener(|this, _, _, cx| {
                            this.set_selected_view(StubViewId::AllPrs, cx);
                            this.set_list_demo(ListDemoMode::Populated, cx);
                        })),
                ),
        )
}

fn render_pr_rows(
    proto: &PrototypeChrome,
    rows: Vec<ListRow>,
    _cx: &mut Context<ReviewDesk>,
) -> impl IntoElement {
    let chrome = proto.chrome;
    let scroll = proto.inbox_scroll.clone();
    let selected = proto.selected_stub_id;
    let entity = proto.entity.clone();
    let collapsed = proto.collapsed_groups.clone();
    let row_count = rows.len();

    div()
        .flex_1()
        .min_h_0()
        .relative()
        .child(
            uniform_list("proto-view-pr-list", row_count, move |range, _, cx| {
                range
                    .map(|ix| {
                        match &rows[ix] {
                            ListRow::Group {
                                key,
                                label,
                                count,
                                depth,
                            } => {
                                let is_status = matches!(
                                    label.as_str(),
                                    "Review required"
                                        | "Approved"
                                        | "Changes requested"
                                        | "Draft"
                                );
                                let is_draft = label.as_str() == "Draft";
                                let indent = if *depth > 0 { 20. } else { 0. };
                                let pt = if *depth == 0 { 14. } else { 8. };
                                let is_collapsed = collapsed.contains(key);
                                let chevron = if is_collapsed {
                                    IconName::ChevronRight
                                } else {
                                    IconName::ChevronDown
                                };
                                let group_key = key.clone();
                                let owner = entity.clone();
                                let mut header = h_flex().w_full().items_center().gap_2().child(
                                    Icon::new(chevron)
                                        .xsmall()
                                        .text_color(cx.theme().muted_foreground),
                                );
                                if is_status {
                                    header = header.child(
                                        Icon::new(review_icon(label, is_draft))
                                            .xsmall()
                                            .text_color(review_status_color(
                                                label, is_draft, cx,
                                            )),
                                    );
                                } else {
                                    header = header.child(
                                        Icon::new(IconName::Folder)
                                            .xsmall()
                                            .text_color(cx.theme().muted_foreground),
                                    );
                                }
                                div()
                                    .id(SharedString::from(format!("group-{ix}")))
                                    .w_full()
                                    .pl(px(16. + indent))
                                    .pr_4()
                                    .pt(px(pt))
                                    .pb(px(6.))
                                    .cursor_pointer()
                                    .rounded_md()
                                    .hover(|style| {
                                        style.bg(cx.theme().secondary.opacity(0.3))
                                    })
                                    .on_click(move |_, _, cx| {
                                        let key = group_key.clone();
                                        let _ = owner.update(cx, |this, cx| {
                                            this.toggle_collapsed_group(key, cx);
                                        });
                                    })
                                    .child(
                                        header
                                            .child(
                                                div()
                                                    .text_sm()
                                                    .font_medium()
                                                    .child(label.clone()),
                                            )
                                            .child(
                                                div()
                                                    .text_sm()
                                                    .text_color(cx.theme().muted_foreground)
                                                    .child(count.to_string()),
                                            )
                                            .child(div().flex_1())
                                            .child(
                                                Icon::new(IconName::Plus)
                                                    .xsmall()
                                                    .text_color(cx.theme().muted_foreground),
                                            ),
                                    )
                                    .into_any_element()
                            }
                            ListRow::Pr { pr } => {
                                let pr = *pr;
                                let is_sel = selected == Some(pr.id);
                                let owner = entity.clone();

                                if chrome.full_bleed_list() {
                                    // Linear clone: plain row + hairline, not ListItem chrome.
                                    div()
                                        .id(SharedString::from(pr.id))
                                        .w_full()
                                        .px_4()
                                        .py(px(7.))
                                        .border_b_1()
                                        .border_color(cx.theme().border.opacity(0.55))
                                        .cursor_pointer()
                                        .when(is_sel, |el| {
                                            el.bg(cx.theme().secondary.opacity(0.4))
                                        })
                                        .when(!is_sel, |el| {
                                            el.hover(|style| {
                                                style.bg(cx.theme().secondary.opacity(0.25))
                                            })
                                        })
                                        .on_click(move |_, _, cx| {
                                            let _ = owner.update(cx, |this, cx| {
                                                this.set_selected_stub(pr.id, cx);
                                            });
                                        })
                                        .child(render_dense_pr_row(pr, cx))
                                        .into_any_element()
                                } else {
                                    let row = ListItem::new(SharedString::from(pr.id))
                                        .selected(is_sel)
                                        .px(px(12.))
                                        .py(if chrome.roomy_rows() {
                                            px(8.)
                                        } else {
                                            px(5.)
                                        })
                                        .on_click(move |_, _, cx| {
                                            let _ = owner.update(cx, |this, cx| {
                                                this.set_selected_stub(pr.id, cx);
                                            });
                                        });

                                    if chrome.table_rows() {
                                        row.child(render_table_pr_row(pr, cx))
                                            .into_any_element()
                                    } else if chrome.roomy_rows() {
                                        row.child(render_roomy_pr_row(pr, cx))
                                            .into_any_element()
                                    } else {
                                        row.child(render_dense_pr_row(pr, cx))
                                            .into_any_element()
                                    }
                                }
                            }
                        }
                    })
                    .collect()
            })
            .size_full()
            .track_scroll(&scroll),
        )
        .child(Scrollbar::vertical(&scroll))
}

/// Linear Issues/backlog row → PR.
/// priority · #71 · status · title ··· [repo] [labels] · date · avatar
fn render_dense_pr_row(pr: StubPr, cx: &App) -> impl IntoElement {
    let author_initial = pr
        .author
        .chars()
        .next()
        .map(|c| c.to_ascii_uppercase().to_string())
        .unwrap_or_else(|| "?".into());
    let status_color = review_status_color(pr.review_state, pr.is_draft, cx);

    let mut row = h_flex()
        .w_full()
        .items_center()
        .gap_2()
        // Priority / grip (Linear left affordance)
        .child(
            Icon::new(IconName::EllipsisVertical)
                .xsmall()
                .text_color(cx.theme().muted_foreground.opacity(0.45)),
        )
        // PR id only
        .child(
            div()
                .min_w(px(36.))
                .text_xs()
                .text_color(cx.theme().muted_foreground)
                .child(format!("#{}", pr.number)),
        )
        // Status icon (review state)
        .child(
            Icon::new(review_icon(pr.review_state, pr.is_draft))
                .xsmall()
                .text_color(status_color),
        )
        // Title
        .child(
            div()
                .flex_1()
                .min_w_0()
                .text_sm()
                .truncate()
                .child(pr.title),
        );

    // Right-side pills sit immediately before date (Linear project/label zone).
    row = row.child(row_pill(Some(IconName::LayoutDashboard), short_repo(pr.repo), cx));
    for label in pr.labels.iter().take(2) {
        row = row.child(row_pill(None, label, cx));
    }

    if pr.ci == "Failing" {
        row = row.child(
            h_flex()
                .items_center()
                .justify_center()
                .size(px(16.))
                .rounded_full()
                .bg(cx.theme().danger.opacity(0.85))
                .child(
                    div()
                        .text_xs()
                        .text_color(cx.theme().danger_foreground)
                        .child("1"),
                ),
        );
    }

    row.child(
        div()
            .min_w(px(36.))
            .text_xs()
            .text_color(cx.theme().muted_foreground)
            .child(pr.updated),
    )
    .child(
        div()
            .size(px(20.))
            .rounded_full()
            .bg(author_color(pr.author, cx))
            .flex()
            .items_center()
            .justify_center()
            .child(
                div()
                    .text_xs()
                    .font_medium()
                    .text_color(cx.theme().background)
                    .child(author_initial),
            ),
    )
}

fn row_pill(icon: Option<IconName>, text: &str, cx: &App) -> impl IntoElement {
    let mut pill = h_flex()
        .items_center()
        .gap_1()
        .px_1()
        .py(px(2.))
        .rounded_md()
        .bg(cx.theme().secondary.opacity(0.65))
        .flex_shrink_0();
    if let Some(icon) = icon {
        pill = pill.child(
            Icon::new(icon)
                .xsmall()
                .text_color(cx.theme().muted_foreground),
        );
    }
    pill.child(
        div()
            .max_w(px(110.))
            .text_xs()
            .text_color(cx.theme().muted_foreground)
            .truncate()
            .child(text.to_string()),
    )
}

fn author_color(author: &str, cx: &App) -> gpui_kit::Hsla {
    // Stable stub colors so avatars aren't all the same accent.
    match author.chars().next().unwrap_or('a').to_ascii_lowercase() {
        'a'..='e' => cx.theme().accent,
        'f'..='j' => cx.theme().success,
        'k'..='o' => cx.theme().warning,
        _ => cx.theme().info,
    }
}

fn review_status_color(state: &str, is_draft: bool, cx: &App) -> gpui_kit::Hsla {
    if is_draft {
        return cx.theme().muted_foreground;
    }
    match state {
        "Approved" => cx.theme().success,
        "Changes requested" => cx.theme().danger,
        "Review required" => cx.theme().warning,
        _ => cx.theme().muted_foreground,
    }
}

fn review_icon(state: &str, is_draft: bool) -> IconName {
    if is_draft {
        return IconName::Square;
    }
    match state {
        "Approved" => IconName::CircleCheck,
        "Changes requested" => IconName::CircleAlert,
        "Review required" => IconName::LoaderCircle,
        _ => IconName::Dash,
    }
}

fn render_roomy_pr_row(pr: StubPr, cx: &App) -> impl IntoElement {
    v_flex()
        .w_full()
        .gap_1()
        .child(
            div()
                .text_sm()
                .font_medium()
                .truncate()
                .child(pr.title),
        )
        .child(
            h_flex()
                .w_full()
                .items_center()
                .gap_2()
                .child(
                    div()
                        .text_xs()
                        .text_color(cx.theme().muted_foreground)
                        .child(format!("{} · {}", pr.repo, pr.author)),
                )
                .child(meta_pill(pr.review_state, cx))
                .child(meta_pill(pr.ci, cx))
                .child(
                    div()
                        .text_xs()
                        .text_color(cx.theme().muted_foreground)
                        .child(pr.updated),
                ),
        )
}

fn render_table_pr_row(pr: StubPr, cx: &App) -> impl IntoElement {
    h_flex()
        .w_full()
        .items_center()
        .gap_2()
        .child(
            v_flex()
                .flex_1()
                .min_w_0()
                .gap_0()
                .child(
                    div()
                        .text_sm()
                        .font_medium()
                        .truncate()
                        .child(pr.title),
                )
                .child(
                    div()
                        .text_xs()
                        .text_color(cx.theme().muted_foreground)
                        .truncate()
                        .child(format!("#{} · {}", pr.number, pr.author)),
                ),
        )
        .child(
            div()
                .w(px(88.))
                .text_xs()
                .text_color(cx.theme().muted_foreground)
                .truncate()
                .child(pr.review_state),
        )
        .child(
            div()
                .w(px(56.))
                .text_xs()
                .text_color(cx.theme().muted_foreground)
                .child(pr.ci),
        )
        .child(
            div()
                .w(px(28.))
                .text_xs()
                .text_color(cx.theme().muted_foreground)
                .child(pr.updated),
        )
}

fn meta_pill(text: &str, cx: &App) -> impl IntoElement {
    div()
        .px_1()
        .rounded_md()
        .bg(cx.theme().secondary.opacity(0.55))
        .child(
            div()
                .text_xs()
                .text_color(cx.theme().muted_foreground)
                .child(text.to_string()),
        )
}

fn short_repo(repo: &str) -> &str {
    repo.rsplit_once('/')
        .map(|(_, name)| name)
        .unwrap_or(repo)
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
                        .child("notifications · stub"),
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
                        .child("Not the PR list. Open Pull requests for Views."),
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
    if proto.selected_stub_id.is_none() {
        return empty_review(proto.shell_nav, cx).into_any_element();
    }

    let chrome = proto.chrome;
    let stage = v_flex()
        .size_full()
        .min_h_0()
        .child(render_review_tabs(proto.review_tab, chrome.dense(), cx));

    let body = render_tab_body(proto, cx);
    stage
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
                                .child(render_metadata_panel(proto, chrome, cx)),
                        )
                    }),
            ),
        )
        .into_any_element()
}

fn empty_review(shell_nav: ShellNav, cx: &mut Context<ReviewDesk>) -> impl IntoElement {
    let hint = match shell_nav {
        ShellNav::Inbox => "Inbox is notifications (stub). Open a pull request from a View.",
        ShellNav::PullRequests => "Select a pull request in this View to open Review.",
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
    // Stub selection: paint stub Overview unless a real session detail is loaded.
    let has_real = matches!(&proto.view.detail, LoadState::Ready(_));
    match proto.review_tab {
        PrototypeReviewTab::Overview if has_real => {
            render_overview(&proto.view, proto.entity.clone(), cx).into_any_element()
        }
        PrototypeReviewTab::Overview => {
            let title = selected_stub_title(proto);
            stub_overview(&title, cx).into_any_element()
        }
        PrototypeReviewTab::Pipelines => stub_tab(
            "Pipelines",
            &[
                "ci / test · failed · 2m ago",
                "ci / lint · passed · 2m ago",
                "ci / build · passed · 3m ago",
            ],
            cx,
        )
        .into_any_element(),
        PrototypeReviewTab::Conversation => stub_tab(
            "Conversation",
            &[
                "alice: Nested Views feel closest to Linear.",
                "you: Chips keep the sidebar quieter.",
                "bob: Views rail makes counts scannable.",
            ],
            cx,
        )
        .into_any_element(),
        PrototypeReviewTab::Changes if has_real => render_files(
            &proto.view,
            &proto.diff,
            &proto.files_scroll,
            proto.entity.clone(),
            cx,
        )
        .into_any_element(),
        PrototypeReviewTab::Changes => stub_tab(
            "Changes",
            &["Diff stub — open a synced PR from a real Provider for Files."],
            cx,
        )
        .into_any_element(),
    }
}

fn stub_overview(title: &str, cx: &mut Context<ReviewDesk>) -> impl IntoElement {
    let lines = vec![
        title.to_string(),
        "Stub body — Views prototype does not load Provider detail.".into(),
        "CODEOWNERS breakdown would land here per #53.".into(),
    ];
    stub_tab_owned("Overview", lines, cx)
}

fn stub_tab(
    title: &str,
    lines: &[&str],
    cx: &mut Context<ReviewDesk>,
) -> impl IntoElement {
    let lines: Vec<String> = lines.iter().map(|s| (*s).to_string()).collect();
    stub_tab_owned(title, lines, cx)
}

fn stub_tab_owned(
    title: &str,
    lines: Vec<String>,
    cx: &mut Context<ReviewDesk>,
) -> impl IntoElement {
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
                .child(line)
        }))
}

fn render_metadata_panel(
    proto: &PrototypeChrome,
    chrome: ChromeVariant,
    cx: &mut Context<ReviewDesk>,
) -> impl IntoElement {
    let header = if chrome.uppercase_labels() {
        "METADATA"
    } else {
        "Metadata"
    };
    let header_py = if chrome.dense() { px(4.) } else { px(8.) };
    let link = proto
        .selected_stub_id
        .map(|id| id.trim_start_matches("stub/").to_string());

    v_flex()
        .size_full()
        .min_h_0()
        .border_l_1()
        .border_color(cx.theme().border)
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
                .when_some(link, |el, short| {
                    el.child(
                        v_flex()
                            .gap_0()
                            .child(
                                div()
                                    .text_xs()
                                    .text_color(cx.theme().muted_foreground)
                                    .child("Link"),
                            )
                            .child(div().text_sm().child(short)),
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
                })),
        )
}

fn render_chrome_switcher(
    chrome: ChromeVariant,
    cx: &mut Context<ReviewDesk>,
) -> impl IntoElement {
    h_flex()
        .id("chrome-prototype-switcher")
        .w_full()
        .justify_center()
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
                    Button::new("chrome-prev")
                        .ghost()
                        .xsmall()
                        .label("←")
                        .on_click(cx.listener(|this, _, _, cx| {
                            this.cycle_chrome(false, cx);
                        })),
                )
                .child(
                    div()
                        .min_w(px(240.))
                        .text_sm()
                        .text_center()
                        .font_medium()
                        .child(chrome.label()),
                )
                .child(
                    Button::new("chrome-next")
                        .ghost()
                        .xsmall()
                        .label("→")
                        .on_click(cx.listener(|this, _, _, cx| {
                            this.cycle_chrome(true, cx);
                        })),
                ),
        )
}

fn selected_stub_title(proto: &PrototypeChrome) -> String {
    let Some(id) = proto.selected_stub_id else {
        return "Pull request".into();
    };
    STUB_PRS
        .iter()
        .find(|pr| pr.id == id)
        .map(|pr| pr.title.to_string())
        .unwrap_or_else(|| "Pull request".into())
}

fn prs_for_view(view: StubViewId, repo_scope: Option<&str>) -> Vec<StubPr> {
    STUB_PRS
        .iter()
        .copied()
        .filter(|pr| match repo_scope {
            Some(repo) => pr.repo == repo,
            None => true,
        })
        .filter(|pr| match view {
            StubViewId::AllPrs | StubViewId::ByProject => true,
            StubViewId::NeedsMyReview => pr.i_am_reviewer && !pr.is_draft,
            StubViewId::Mine => pr.i_am_author && !pr.is_draft,
            StubViewId::Drafts => pr.i_am_author && pr.is_draft,
        })
        .collect()
}

fn stub_count_for_view(
    view: StubViewId,
    demo: ListDemoMode,
    repo_scope: Option<&str>,
) -> usize {
    match demo {
        ListDemoMode::EmptyStore | ListDemoMode::ZeroMatch => 0,
        ListDemoMode::Populated => prs_for_view(view, repo_scope).len(),
    }
}

fn build_list_rows(view: StubViewId, repo_scope: Option<&str>) -> Vec<ListRow> {
    let prs = prs_for_view(view, repo_scope);
    let Some(group_by) = view.group_by() else {
        return prs
            .into_iter()
            .map(|pr| ListRow::Pr { pr })
            .collect();
    };

    if group_by == "repo→status" {
        return build_nested_repo_status_rows(&prs);
    }

    let mut keys: Vec<String> = Vec::new();
    for pr in &prs {
        let key = match group_by {
            "repo" => pr.repo.to_string(),
            "review state" => pr.review_state.to_string(),
            other => other.to_string(),
        };
        if !keys.iter().any(|k| k == &key) {
            keys.push(key);
        }
    }

    let mut rows = Vec::new();
    for key in keys {
        let group_prs: Vec<StubPr> = prs
            .iter()
            .copied()
            .filter(|pr| match group_by {
                "repo" => pr.repo == key,
                "review state" => pr.review_state == key,
                _ => true,
            })
            .collect();
        let group_key = format!("{group_by}:{key}");
        rows.push(ListRow::Group {
            key: group_key,
            label: key,
            count: group_prs.len(),
            depth: 0,
        });
        for pr in group_prs {
            rows.push(ListRow::Pr { pr });
        }
    }
    rows
}

/// Primary groups by repository; nested groups by review status.
fn build_nested_repo_status_rows(prs: &[StubPr]) -> Vec<ListRow> {
    let mut repos: Vec<&str> = Vec::new();
    for pr in prs {
        if !repos.iter().any(|r| *r == pr.repo) {
            repos.push(pr.repo);
        }
    }

    let mut rows = Vec::new();
    for repo in repos {
        let repo_prs: Vec<StubPr> = prs.iter().copied().filter(|pr| pr.repo == repo).collect();
        rows.push(ListRow::Group {
            key: format!("repo:{repo}"),
            label: repo.to_string(),
            count: repo_prs.len(),
            depth: 0,
        });

        let mut states: Vec<&str> = Vec::new();
        for pr in &repo_prs {
            if !states.iter().any(|s| *s == pr.review_state) {
                states.push(pr.review_state);
            }
        }
        for state in states {
            let state_prs: Vec<StubPr> = repo_prs
                .iter()
                .copied()
                .filter(|pr| pr.review_state == state)
                .collect();
            rows.push(ListRow::Group {
                key: format!("repo:{repo}/status:{state}"),
                label: state.to_string(),
                count: state_prs.len(),
                depth: 1,
            });
            for pr in state_prs {
                rows.push(ListRow::Pr { pr });
            }
        }
    }
    rows
}

/// Drop children of collapsed groups from the visible uniform_list (Zed flatten pattern).
fn visible_list_rows(rows: Vec<ListRow>, collapsed: &HashSet<String>) -> Vec<ListRow> {
    let mut out = Vec::with_capacity(rows.len());
    let mut skip_until_depth: Option<u8> = None;

    for row in rows {
        match row {
            ListRow::Group {
                key,
                label,
                count,
                depth,
            } => {
                if let Some(skip_depth) = skip_until_depth {
                    if depth > skip_depth {
                        continue;
                    }
                    skip_until_depth = None;
                }
                let collapse_this = collapsed.contains(&key);
                if collapse_this {
                    skip_until_depth = Some(depth);
                }
                out.push(ListRow::Group {
                    key,
                    label,
                    count,
                    depth,
                });
            }
            ListRow::Pr { pr } => {
                if skip_until_depth.is_some() {
                    continue;
                }
                out.push(ListRow::Pr { pr });
            }
        }
    }
    out
}

/// Keep session Overview/Changes in sync when the prototype tab changes.
pub(crate) fn sync_session_tab(tab: PrototypeReviewTab) -> Option<ReviewTab> {
    match tab {
        PrototypeReviewTab::Overview => Some(ReviewTab::Overview),
        PrototypeReviewTab::Changes => Some(ReviewTab::Files),
        PrototypeReviewTab::Pipelines | PrototypeReviewTab::Conversation => None,
    }
}
