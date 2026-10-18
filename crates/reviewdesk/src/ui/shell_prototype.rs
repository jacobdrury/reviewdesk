//! PROTOTYPE paint: Linear Issues-style PR list + left nav. Throwaway with the branch.
//! Locked to former variant A; iterating in place. Demo mode cycles empty/zero-match.

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
    App, Context, Entity, InteractiveElement as _, IntoElement, ParentElement as _,
    SharedString, StatefulInteractiveElement as _, Styled as _, UniformListScrollHandle,
    WeakEntity, div, px, uniform_list,
    prelude::FluentBuilder as _,
};

use crate::app::ReviewDesk;
use crate::chrome_prototype::{
    ListDemoMode, METADATA_DEFAULT_WIDTH, PrototypeReviewTab, SIDEBAR_WIDTH, ShellNav, StubViewId,
    STUB_REPOS,
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
struct StubLabel {
    name: &'static str,
}

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
    labels: &'static [StubLabel],
    files: u32,
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
        labels: &[StubLabel {
            name: "wayfinder:prototype",
        }],
        files: 8,
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
        labels: &[
            StubLabel {
                name: "wayfinder:prototype",
            },
            StubLabel { name: "shell" },
        ],
        files: 12,
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
        labels: &[StubLabel {
            name: "wayfinder:grilling",
        }],
        files: 3,
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
        labels: &[StubLabel { name: "billing" }],
        files: 5,
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
        labels: &[StubLabel { name: "design" }, StubLabel { name: "billing" }],
        files: 4,
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
        labels: &[StubLabel { name: "ui" }],
        files: 2,
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
        labels: &[StubLabel { name: "theme" }],
        files: 6,
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
        labels: &[StubLabel { name: "core" }],
        files: 15,
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
        labels: &[StubLabel { name: "billing" }, StubLabel { name: "infra" }],
        files: 7,
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
        labels: &[StubLabel { name: "ui" }],
        files: 3,
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
    pub shell_nav: ShellNav,
    pub review_tab: PrototypeReviewTab,
    pub metadata_open: bool,
    pub selected_view: StubViewId,
    pub list_demo: ListDemoMode,
    pub selected_stub_id: Option<&'static str>,
    pub selected_repo: Option<&'static str>,
    pub collapsed_groups: HashSet<String>,
    pub filter_menu_open: bool,
    /// Dismissed Linear-style filter chips (prototype).
    pub dismissed_filters: HashSet<&'static str>,
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
    let sync_label = proto.view.inbox_sync.status_label();

    v_flex()
        .size_full()
        .bg(cx.theme().background)
        .child(render_title_bar(&sync_label, cx))
        .child(div().flex_1().min_h_0().child(render_body(proto, cx)))
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
    let notif_count = STUB_NOTIFICATIONS.len();

    // Focus Review — list gone until Back.
    if proto.selected_stub_id.is_some() {
        return h_flex()
            .size_full()
            .min_h_0()
            .child(render_linear_sidebar(&proto, notif_count, cx))
            .child(
                div()
                    .flex_1()
                    .min_h_0()
                    .h_full()
                    .child(render_focus_review(&proto, cx)),
            )
            .into_any_element();
    }

    // Full-bleed list / Inbox; no empty Review column.
    h_flex()
        .size_full()
        .min_h_0()
        .child(render_linear_sidebar(&proto, notif_count, cx))
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
        .into_any_element()
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
        .w(px(SIDEBAR_WIDTH))
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
            IconName::Github,
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





fn render_pull_requests(
    proto: &PrototypeChrome,
    cx: &mut Context<ReviewDesk>,
) -> impl IntoElement {
    let view = proto.selected_view;

    let pane = v_flex()
        .size_full()
        .min_h_0()
        .child(render_linear_view_tabs(proto, cx))
        .child(render_linear_filter_display(proto, cx));

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

/// Linear Filter row: Filter button + composed chips + Display.
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

    let mut bar = v_flex()
        .w_full()
        .border_b_1()
        .border_color(cx.theme().border);

    let mut row = h_flex()
        .w_full()
        .px_4()
        .py_2()
        .items_center()
        .gap_2()
        .child(
            h_flex()
                .id("filter-toggle")
                .items_center()
                .gap_1()
                .px_2()
                .py(px(4.))
                .rounded_md()
                .cursor_pointer()
                .when(proto.filter_menu_open, |el| {
                    el.bg(cx.theme().secondary.opacity(0.55))
                })
                .hover(|style| style.bg(cx.theme().secondary.opacity(0.5)))
                .on_click(cx.listener(|this, _, _, cx| {
                    this.toggle_filter_menu(cx);
                }))
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
        );

    // Active Linear-style chips for the current View / repo scope.
    if !proto.dismissed_filters.contains("view") && view != StubViewId::AllPrs {
        row = row.child(linear_filter_chip(
            "view",
            "View",
            "is",
            view.label(),
            None,
            cx,
        ));
    }
    if let Some(repo) = proto.selected_repo {
        if !proto.dismissed_filters.contains("repo") {
            row = row.child(linear_filter_chip(
                "repo",
                "Repository",
                "is",
                short_repo(repo),
                Some(IconName::Github),
                cx,
            ));
        }
    }
    if matches!(
        view,
        StubViewId::NeedsMyReview | StubViewId::Mine | StubViewId::Drafts
    ) && !proto.dismissed_filters.contains("review")
    {
        let value = match view {
            StubViewId::NeedsMyReview => "Review required",
            StubViewId::Drafts => "Draft",
            _ => "Open",
        };
        row = row.child(linear_filter_chip(
            "review",
            "Status",
            "is any of",
            value,
            Some(review_icon(value, value == "Draft")),
            cx,
        ));
    }

    let has_chips = view != StubViewId::AllPrs || proto.selected_repo.is_some();
    if has_chips {
        row = row.child(
            div()
                .id("clear-filters")
                .px_2()
                .py(px(4.))
                .rounded_md()
                .cursor_pointer()
                .hover(|style| style.bg(cx.theme().secondary.opacity(0.45)))
                .on_click(cx.listener(|this, _, _, cx| {
                    this.clear_filters(cx);
                }))
                .child(
                    div()
                        .text_sm()
                        .text_color(cx.theme().muted_foreground)
                        .child("Clear filters"),
                ),
        );
    }

    row = row.child(div().flex_1()).child(
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
    );

    bar = bar.child(row);

    if proto.filter_menu_open {
        bar = bar.child(render_filter_dimension_menu(cx));
    }

    bar
}

/// Linear “add filter” dimensions strip.
fn render_filter_dimension_menu(cx: &mut Context<ReviewDesk>) -> impl IntoElement {
    const DIMENSIONS: &[(&str, IconName)] = &[
        ("Status", IconName::CircleCheck),
        ("Repository", IconName::Github),
        ("Author", IconName::User),
        ("Label", IconName::Asterisk),
        ("Draft", IconName::Square),
        ("CI", IconName::LoaderCircle),
        ("I am reviewer", IconName::Eye),
        ("Updated", IconName::Calendar),
    ];
    let mut menu = h_flex()
        .w_full()
        .px_4()
        .pb_2()
        .gap_1()
        .items_center();
    menu = menu.child(
        div()
            .text_xs()
            .text_color(cx.theme().muted_foreground)
            .child("Filter by"),
    );
    for (label, icon) in DIMENSIONS {
        let icon = icon.clone();
        menu = menu.child(
            h_flex()
                .items_center()
                .gap_1()
                .px_2()
                .py(px(4.))
                .rounded_md()
                .border_1()
                .border_color(cx.theme().border)
                .cursor_pointer()
                .hover(|style| style.bg(cx.theme().secondary.opacity(0.45)))
                .child(
                    Icon::new(icon)
                        .xsmall()
                        .text_color(cx.theme().muted_foreground),
                )
                .child(
                    div()
                        .text_sm()
                        .text_color(cx.theme().foreground)
                        .child(*label),
                ),
        );
    }
    menu
}

/// Linear composed filter chip: `Status` `is any of` [icon] `value` ×
fn linear_filter_chip(
    id: &'static str,
    field: &str,
    op: &str,
    value: &str,
    value_icon: Option<IconName>,
    cx: &mut Context<ReviewDesk>,
) -> impl IntoElement {
    let field = field.to_string();
    let op = op.to_string();
    let value = value.to_string();
    h_flex()
        .id(SharedString::from(format!("fchip-{id}")))
        .items_center()
        .gap_1()
        .px_2()
        .py(px(3.))
        .rounded_md()
        .border_1()
        .border_color(cx.theme().border)
        .bg(cx.theme().secondary.opacity(0.35))
        .child(
            div()
                .text_xs()
                .font_medium()
                .text_color(cx.theme().muted_foreground)
                .child(field),
        )
        .child(
            div()
                .text_xs()
                .text_color(cx.theme().muted_foreground)
                .child(op),
        )
        .when_some(value_icon, |el, icon| {
            el.child(
                Icon::new(icon)
                    .xsmall()
                    .text_color(cx.theme().foreground),
            )
        })
        .child(
            div()
                .text_xs()
                .font_medium()
                .text_color(cx.theme().foreground)
                .child(value),
        )
        .child(
            div()
                .id(SharedString::from(format!("fchip-x-{id}")))
                .cursor_pointer()
                .px_1()
                .on_click(cx.listener(move |this, _, _, cx| {
                    this.dismiss_filter_chip(id, cx);
                }))
                .child(
                    Icon::new(IconName::Close)
                        .xsmall()
                        .text_color(cx.theme().muted_foreground),
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
/// grip · #71 · 2h · status · title ··· [repo] [labels] · files ±lines · avatar
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
        .child(
            Icon::new(IconName::EllipsisVertical)
                .xsmall()
                .text_color(cx.theme().muted_foreground.opacity(0.45)),
        )
        .child(
            div()
                .min_w(px(36.))
                .text_xs()
                .text_color(cx.theme().muted_foreground)
                .child(format!("#{}", pr.number)),
        )
        // Time sits immediately right of the PR number.
        .child(
            div()
                .min_w(px(28.))
                .text_xs()
                .text_color(cx.theme().muted_foreground)
                .child(pr.updated),
        )
        .child(
            Icon::new(review_icon(pr.review_state, pr.is_draft))
                .xsmall()
                .text_color(status_color),
        )
        .child(
            div()
                .flex_1()
                .min_w_0()
                .text_sm()
                .truncate()
                .child(pr.title),
        );

    row = row.child(repo_pill(short_repo(pr.repo), cx));
    for label in pr.labels.iter().take(2) {
        row = row.child(label_pill(label.name, cx));
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

    row.child(filesize_chip(pr.files, cx))
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

fn repo_pill(name: &str, cx: &App) -> impl IntoElement {
    h_flex()
        .items_center()
        .gap_1()
        .px_1()
        .py(px(2.))
        .rounded_md()
        .bg(cx.theme().secondary.opacity(0.65))
        .flex_shrink_0()
        .child(
            Icon::new(IconName::Github)
                .xsmall()
                .text_color(cx.theme().muted_foreground),
        )
        .child(
            div()
                .max_w(px(110.))
                .text_xs()
                .text_color(cx.theme().muted_foreground)
                .truncate()
                .child(name.to_string()),
        )
}

fn label_pill(name: &str, cx: &App) -> impl IntoElement {
    let color = provider_label_color(name, cx);
    h_flex()
        .items_center()
        .gap_1()
        .px_1()
        .py(px(2.))
        .rounded_md()
        .bg(color.opacity(0.22))
        .border_1()
        .border_color(color.opacity(0.55))
        .flex_shrink_0()
        .child(
            div()
                .size(px(6.))
                .rounded_full()
                .bg(color),
        )
        .child(
            div()
                .max_w(px(120.))
                .text_xs()
                .text_color(color)
                .truncate()
                .child(name.to_string()),
        )
}

/// Compact PR size signal: file count only (no ± line noise).
fn filesize_chip(files: u32, cx: &App) -> impl IntoElement {
    h_flex()
        .items_center()
        .gap_1()
        .flex_shrink_0()
        .child(
            Icon::new(IconName::File)
                .xsmall()
                .text_color(cx.theme().muted_foreground),
        )
        .child(
            div()
                .text_xs()
                .text_color(cx.theme().muted_foreground)
                .child(files.to_string()),
        )
}

/// Distinct colors per Provider label name (stub palette).
fn provider_label_color(name: &str, cx: &App) -> gpui_kit::Hsla {
    match name {
        "wayfinder:prototype" => cx.theme().accent,
        "wayfinder:grilling" => cx.theme().warning,
        "billing" => cx.theme().success,
        "design" => cx.theme().info,
        "ui" => cx.theme().link,
        "theme" => cx.theme().warning.opacity(0.85),
        "core" => cx.theme().danger.opacity(0.75),
        "shell" => cx.theme().accent.opacity(0.8),
        "infra" => cx.theme().muted_foreground,
        _ => cx.theme().muted_foreground,
    }
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




fn short_repo(repo: &str) -> &str {
    repo.rsplit_once('/')
        .map(|(_, name)| name)
        .unwrap_or(repo)
}

fn render_notifications(
    proto: &PrototypeChrome,
    cx: &mut Context<ReviewDesk>,
) -> impl IntoElement {
    let scroll = proto.notif_scroll.clone();
    let items = STUB_NOTIFICATIONS;

    v_flex()
        .size_full()
        .min_h_0()
        .child(
            h_flex()
                .w_full()
                .px_4()
                .py(px(8.))
                .justify_between()
                .border_b_1()
                .border_color(cx.theme().border)
                .child(
                    div()
                        .text_sm()
                        .font_medium()
                        .child("Inbox"),
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
                                    .px_4()
                                    .py(px(6.))
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

    let stage = v_flex()
        .size_full()
        .min_h_0()
        .child(render_review_tabs(proto.review_tab, cx));

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
                                .size(px(METADATA_DEFAULT_WIDTH))
                                .size_range(px(200.)..px(420.))
                                .child(render_metadata_panel(proto, cx)),
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
    cx: &mut Context<ReviewDesk>,
) -> impl IntoElement {
    let tab_ix = current.index();
    h_flex()
        .w_full()
        .items_center()
        .border_b_1()
        .border_color(cx.theme().border)
        .child(
            TabBar::new("proto-review-tabs")
                .flex_1()
                .px_2()
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
    cx: &mut Context<ReviewDesk>,
) -> impl IntoElement {
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
                        .child("Metadata"),
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
