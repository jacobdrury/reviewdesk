use std::collections::HashSet;
use std::sync::Arc;

use gpui_kit::component::{
    ActiveTheme as _, Disableable as _, Icon, IconName, Sizable as _, StyledExt as _,
    button::{Button, ButtonVariants as _},
    h_flex,
    scroll::Scrollbar,
    spinner::Spinner,
    v_flex,
};
use gpui_kit::{
    App, Context, InteractiveElement as _, IntoElement, ParentElement as _, SharedString,
    StatefulInteractiveElement as _, Styled as _, UniformListScrollHandle, WeakEntity, div,
    prelude::FluentBuilder as _, px, uniform_list,
};
use reviewdesk_core::{
    DraftFilter, PullRequestListRow, PullRequestsEmptyKind, ReviewState, UpdatedWithin,
    VIEW_ID_ALL_PRS, review_state_label, visible_list_rows,
};

use crate::app::ReviewDesk;
use crate::review_session::{Intent, LoadState, SessionView};

const LIST_ROW_BASE_PL: f32 = 16.;
const LIST_NEST_STEP: f32 = 20.;

fn list_row_pl(depth: u8) -> f32 {
    LIST_ROW_BASE_PL + LIST_NEST_STEP * f32::from(depth)
}

pub(crate) struct PullRequestsListChrome<'a> {
    pub scroll: &'a UniformListScrollHandle,
    pub collapsed_groups: &'a HashSet<String>,
    pub filter_menu_open: bool,
}

pub(crate) fn render_pull_requests(
    view: &SessionView,
    chrome: PullRequestsListChrome<'_>,
    entity: WeakEntity<ReviewDesk>,
    cx: &mut Context<ReviewDesk>,
) -> impl IntoElement {
    let pull_requests_scroll = chrome.scroll;
    let collapsed_groups = chrome.collapsed_groups;
    let pane = &view.pull_requests_pane;

    v_flex()
        .size_full()
        .min_h_0()
        .child(render_view_tabs(pane, entity.clone(), cx))
        .child(render_linear_filter_display(
            pane,
            chrome.filter_menu_open,
            entity.clone(),
            cx,
        ))
        .child(render_list_body(
            view,
            pull_requests_scroll,
            collapsed_groups,
            entity,
            cx,
        ))
}

fn render_view_tabs(
    pane: &crate::pull_requests_pane::PullRequestsPaneView,
    entity: WeakEntity<ReviewDesk>,
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

    for view in pane.views.iter() {
        let active = pane.active_view_id.as_ref() == Some(&view.id);
        let id = view.id.clone();
        tabs = tabs.child(
            div()
                .id(SharedString::from(format!("view-tab-{}", view.id.as_str())))
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
                .on_click({
                    let entity = entity.clone();
                    move |_, _, cx| {
                        let id = id.clone();
                        let _ = entity.update(cx, |this, cx| {
                            this.dispatch(Intent::SelectView(id), cx);
                        });
                    }
                })
                .child(
                    div()
                        .text_sm()
                        .font_medium()
                        .text_color(if active {
                            cx.theme().foreground
                        } else {
                            cx.theme().muted_foreground
                        })
                        .child(view.name.clone()),
                ),
        );
    }

    tabs.child(div().flex_1()).when(pane.view_dirty, |el| {
        el.child(
            h_flex()
                .gap_1()
                .pb(px(6.))
                .child(
                    Button::new("view-save")
                        .ghost()
                        .xsmall()
                        .label("Save")
                        .on_click({
                            let entity = entity.clone();
                            move |_, _, cx| {
                                let _ = entity.update(cx, |this, cx| {
                                    this.dispatch(Intent::SaveActiveView, cx);
                                });
                            }
                        }),
                )
                .child(
                    Button::new("view-discard")
                        .ghost()
                        .xsmall()
                        .label("Discard")
                        .on_click({
                            let entity = entity.clone();
                            move |_, _, cx| {
                                let _ = entity.update(cx, |this, cx| {
                                    this.dispatch(Intent::DiscardViewEdits, cx);
                                });
                            }
                        }),
                ),
        )
    })
}

/// Linear Filter row: Filter + composed chips + Clear + Display (#71 @ cb0f2c3).
fn render_linear_filter_display(
    pane: &crate::pull_requests_pane::PullRequestsPaneView,
    filter_menu_open: bool,
    entity: WeakEntity<ReviewDesk>,
    cx: &mut Context<ReviewDesk>,
) -> impl IntoElement {
    let filter = &pane.working_filter;
    let active_id = pane.active_view_id.as_ref().map(|id| id.as_str());
    let is_all_prs = active_id == Some(VIEW_ID_ALL_PRS);
    let view_name = pane.active_view_name.clone();

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
                .when(filter_menu_open, |el| {
                    el.bg(cx.theme().secondary.opacity(0.55))
                })
                .hover(|style| style.bg(cx.theme().secondary.opacity(0.5)))
                .on_click({
                    let entity = entity.clone();
                    move |_, _, cx| {
                        let _ = entity.update(cx, |this, cx| {
                            this.toggle_filter_menu(cx);
                        });
                    }
                })
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

    if !is_all_prs {
        if let Some(name) = view_name.as_deref() {
            row = row.child(linear_filter_chip(
                "view",
                "View",
                "is",
                name,
                None,
                "clear",
                entity.clone(),
                cx,
            ));
        }
    }

    if let Some(repo) = pane.repo_scope.as_ref() {
        let short = short_repo_name(repo);
        row = row.child(linear_filter_chip(
            "repo",
            "Repository",
            "is",
            &short,
            Some(IconName::Github),
            "repo",
            entity.clone(),
            cx,
        ));
    }

    if filter.i_am_reviewer {
        row = row.child(linear_filter_chip(
            "reviewer",
            "I am reviewer",
            "is",
            "me",
            Some(IconName::Eye),
            "reviewer",
            entity.clone(),
            cx,
        ));
    }
    if filter.i_am_author {
        row = row.child(linear_filter_chip(
            "author",
            "Author",
            "is",
            "me",
            Some(IconName::User),
            "author",
            entity.clone(),
            cx,
        ));
    }
    if filter.drafts != DraftFilter::All {
        let value = match filter.drafts {
            DraftFilter::ReadyOnly => "Ready",
            DraftFilter::DraftsOnly => "Draft",
            DraftFilter::All => "All",
        };
        row = row.child(linear_filter_chip(
            "draft",
            "Draft",
            "is",
            value,
            Some(IconName::Square),
            "draft",
            entity.clone(),
            cx,
        ));
    }
    if filter.updated_within != UpdatedWithin::Any {
        let value = match filter.updated_within {
            UpdatedWithin::Last24h => "24h",
            UpdatedWithin::Last7d => "7d",
            UpdatedWithin::Last30d => "30d",
            UpdatedWithin::Any => "any",
        };
        row = row.child(linear_filter_chip(
            "updated",
            "Updated",
            "is",
            value,
            Some(IconName::Calendar),
            "updated",
            entity.clone(),
            cx,
        ));
    }
    if filter.review_states.contains(&ReviewState::Pending) {
        row = row.child(linear_filter_chip(
            "status",
            "Status",
            "is any of",
            "Review required",
            Some(IconName::LoaderCircle),
            "status",
            entity.clone(),
            cx,
        ));
    }

    let has_chips = !is_all_prs
        || pane.repo_scope.is_some()
        || filter.i_am_author
        || filter.i_am_reviewer
        || filter.drafts != DraftFilter::All
        || filter.updated_within != UpdatedWithin::Any
        || !filter.review_states.is_empty();

    if has_chips {
        row = row.child(
            div()
                .id("clear-filters")
                .px_2()
                .py(px(4.))
                .rounded_md()
                .cursor_pointer()
                .hover(|style| style.bg(cx.theme().secondary.opacity(0.45)))
                .on_click({
                    let entity = entity.clone();
                    move |_, _, cx| {
                        let _ = entity.update(cx, |this, cx| {
                            this.dispatch(Intent::ClearListFilters, cx);
                        });
                    }
                })
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
                    .child(format!("· {}", pane.group_by_label)),
            ),
    );

    let mut bar = v_flex()
        .w_full()
        .border_b_1()
        .border_color(cx.theme().border)
        .child(row);

    if filter_menu_open {
        bar = bar.child(render_filter_dimension_menu(entity, cx));
    }

    bar
}

fn render_filter_dimension_menu(
    entity: WeakEntity<ReviewDesk>,
    cx: &mut Context<ReviewDesk>,
) -> impl IntoElement {
    const DIMENSIONS: &[(&str, IconName, &str)] = &[
        ("Status", IconName::CircleCheck, "status"),
        ("Repository", IconName::Github, "repo"),
        ("Author", IconName::User, "author"),
        ("Draft", IconName::Square, "draft"),
        ("I am reviewer", IconName::Eye, "reviewer"),
        ("Updated", IconName::Calendar, "updated"),
    ];
    let mut menu = h_flex()
        .w_full()
        .px_4()
        .pb_2()
        .gap_1()
        .items_center()
        .child(
            div()
                .text_xs()
                .text_color(cx.theme().muted_foreground)
                .child("Filter by"),
        );
    for (label, icon, key) in DIMENSIONS {
        let entity = entity.clone();
        let key = *key;
        menu = menu.child(
            h_flex()
                .id(SharedString::from(format!("dim-{key}")))
                .items_center()
                .gap_1()
                .px_2()
                .py(px(4.))
                .rounded_md()
                .border_1()
                .border_color(cx.theme().border)
                .cursor_pointer()
                .hover(|style| style.bg(cx.theme().secondary.opacity(0.45)))
                .on_click(move |_, _, cx| {
                    let _ = entity.update(cx, |this, cx| match key {
                        "author" => this.dispatch(Intent::ToggleWorkingFilterAuthor, cx),
                        "reviewer" => this.dispatch(Intent::ToggleWorkingFilterReviewer, cx),
                        "draft" => this.dispatch(Intent::CycleWorkingFilterDrafts, cx),
                        "updated" => this.dispatch(Intent::CycleWorkingFilterUpdatedWithin, cx),
                        "status" => this.dispatch(
                            Intent::ToggleWorkingFilterReviewState {
                                state: ReviewState::Pending,
                            },
                            cx,
                        ),
                        _ => {}
                    });
                })
                .child(
                    Icon::new(icon.clone())
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
    id: &str,
    field: &str,
    op: &str,
    value: &str,
    value_icon: Option<IconName>,
    dismiss_key: &'static str,
    entity: WeakEntity<ReviewDesk>,
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
            el.child(Icon::new(icon).xsmall().text_color(cx.theme().foreground))
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
                .on_click(move |_, _, cx| {
                    let _ = entity.update(cx, |this, cx| match dismiss_key {
                        "clear" => this.dispatch(Intent::ClearListFilters, cx),
                        "repo" => this.dispatch(Intent::SetRepoScope { repo: None }, cx),
                        "author" => this.dispatch(Intent::ToggleWorkingFilterAuthor, cx),
                        "reviewer" => this.dispatch(Intent::ToggleWorkingFilterReviewer, cx),
                        "draft" => this.dispatch(Intent::CycleWorkingFilterDrafts, cx),
                        "updated" => this.dispatch(Intent::CycleWorkingFilterUpdatedWithin, cx),
                        "status" => this.dispatch(
                            Intent::ToggleWorkingFilterReviewState {
                                state: ReviewState::Pending,
                            },
                            cx,
                        ),
                        _ => {}
                    });
                })
                .child(
                    Icon::new(IconName::Close)
                        .xsmall()
                        .text_color(cx.theme().muted_foreground),
                ),
        )
}

fn render_list_body(
    view: &SessionView,
    pull_requests_scroll: &UniformListScrollHandle,
    collapsed_groups: &HashSet<String>,
    entity: WeakEntity<ReviewDesk>,
    cx: &mut Context<ReviewDesk>,
) -> impl IntoElement {
    let pane = &view.pull_requests_pane;

    match &pane.cache {
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
                    .child("Loading pull requests…"),
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
                    Button::new("pull-requests-retry")
                        .outline()
                        .small()
                        .label("Retry")
                        .on_click(cx.listener(|this, _, _, cx| {
                            this.dispatch(Intent::RefreshPullRequests, cx);
                        })),
                )
                .into_any_element()
        }
        LoadState::Ready(()) => match pane.empty {
            Some(PullRequestsEmptyKind::EmptyStore) => {
                empty_store_state(entity, cx).into_any_element()
            }
            Some(PullRequestsEmptyKind::ZeroMatch) => {
                zero_match_state(pane.active_view_name.as_deref(), entity, cx).into_any_element()
            }
            None => {
                let rows = visible_list_rows(pane.rows.to_vec(), collapsed_groups);
                if rows.is_empty() {
                    return zero_match_state(pane.active_view_name.as_deref(), entity, cx)
                        .into_any_element();
                }
                render_rows(
                    rows,
                    view.selected_id.clone(),
                    pull_requests_scroll,
                    collapsed_groups,
                    entity,
                    cx,
                )
                .into_any_element()
            }
        },
    }
}

fn empty_store_state(
    entity: WeakEntity<ReviewDesk>,
    cx: &mut Context<ReviewDesk>,
) -> impl IntoElement {
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
                .child("Sync from GitHub or check auth. The cache is empty — not a filter miss."),
        )
        .child(
            Button::new("empty-sync")
                .outline()
                .small()
                .label("Sync now")
                .on_click(move |_, _, cx| {
                    let _ = entity.update(cx, |this, cx| {
                        this.dispatch(Intent::RefreshPullRequests, cx);
                    });
                }),
        )
}

fn zero_match_state(
    view_name: Option<&str>,
    entity: WeakEntity<ReviewDesk>,
    cx: &mut Context<ReviewDesk>,
) -> impl IntoElement {
    let label = view_name.unwrap_or("this View");
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
                    "“{label}” filters match nothing in cache. Clear scope or switch View."
                )),
        )
        .child(
            Button::new("zero-clear-scope")
                .outline()
                .small()
                .label("Clear repository scope")
                .on_click(move |_, _, cx| {
                    let _ = entity.update(cx, |this, cx| {
                        this.dispatch(Intent::SetRepoScope { repo: None }, cx);
                    });
                }),
        )
}

fn render_rows(
    rows: Vec<PullRequestListRow>,
    selected_id: Option<reviewdesk_core::PullRequestId>,
    pull_requests_scroll: &UniformListScrollHandle,
    collapsed_groups: &HashSet<String>,
    entity: WeakEntity<ReviewDesk>,
    _cx: &mut Context<ReviewDesk>,
) -> impl IntoElement {
    let row_count = rows.len();
    let scroll = pull_requests_scroll.clone();
    let collapsed = collapsed_groups.clone();

    div()
        .flex_1()
        .min_h_0()
        .relative()
        .child(
            uniform_list("pull-requests-list", row_count, move |range, _, cx| {
                range
                    .map(|ix| match &rows[ix] {
                        PullRequestListRow::Group {
                            key,
                            label,
                            count,
                            depth,
                        } => {
                            let is_status = is_review_status_label(label);
                            let is_draft = label.as_str() == "Draft";
                            let pt = if *depth == 0 { 14. } else { 8. };
                            let chevron = if collapsed.contains(key) {
                                IconName::ChevronRight
                            } else {
                                IconName::ChevronDown
                            };
                            let group_key = Arc::new(key.clone());
                            let owner = entity.clone();
                            let mut header = h_flex().w_full().items_center().gap_2().child(
                                Icon::new(chevron)
                                    .xsmall()
                                    .text_color(cx.theme().muted_foreground),
                            );
                            if is_status {
                                header = header.child(
                                    Icon::new(review_status_icon(label, is_draft))
                                        .xsmall()
                                        .text_color(review_status_color(label, is_draft, cx)),
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
                                .pl(px(list_row_pl(*depth)))
                                .pr_4()
                                .pt(px(pt))
                                .pb(px(6.))
                                .cursor_pointer()
                                .rounded_md()
                                .hover(|style| style.bg(cx.theme().secondary.opacity(0.3)))
                                .on_click({
                                    let group_key = group_key.clone();
                                    let owner = owner.clone();
                                    move |_, _, cx| {
                                        let _ = owner.update(cx, |this, cx| {
                                            this.toggle_collapsed_group(
                                                group_key.as_ref().clone(),
                                                cx,
                                            );
                                        });
                                    }
                                })
                                .child(
                                    header
                                        .child(
                                            h_flex()
                                                .items_center()
                                                .gap_1()
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
                                                ),
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
                        PullRequestListRow::Item { item, depth } => {
                            let id = item.id.clone();
                            let selected = selected_id.as_ref() == Some(&item.id);
                            let owner = entity.clone();
                            div()
                                .id(SharedString::from(item.id.as_str()))
                                .w_full()
                                .pl(px(list_row_pl(*depth)))
                                .pr_4()
                                .py(px(7.))
                                .border_b_1()
                                .border_color(cx.theme().border.opacity(0.55))
                                .cursor_pointer()
                                .when(selected, |el| el.bg(cx.theme().secondary.opacity(0.4)))
                                .when(!selected, |el| {
                                    el.hover(|style| style.bg(cx.theme().secondary.opacity(0.25)))
                                })
                                .on_click(move |_, _, cx| {
                                    let id = id.clone();
                                    let _ = owner.update(cx, |this, cx| {
                                        this.dispatch(Intent::SelectPullRequest(id), cx);
                                    });
                                })
                                .child(render_dense_row(item, cx))
                                .into_any_element()
                        }
                    })
                    .collect()
            })
            .size_full()
            .track_scroll(&scroll),
        )
        .child(Scrollbar::vertical(&scroll))
}

/// Linear Issues/backlog row → PR (#71 HITL @ cb0f2c3).
fn render_dense_row(item: &reviewdesk_core::PullRequestListItem, cx: &App) -> impl IntoElement {
    let status_label = review_state_label(item.review_state, item.is_draft);
    let status_color = review_status_color(status_label, item.is_draft, cx);
    let number = pr_number_label(&item.id);
    let updated = relative_time(item.updated_at);
    let author_initial = item
        .author_login
        .chars()
        .next()
        .map(|c| c.to_ascii_uppercase().to_string())
        .unwrap_or_else(|| "?".into());
    let short = short_repo_name(&item.repo_label);

    let mut row = h_flex()
        .w_full()
        .items_center()
        .gap_2()
        .child(
            h_flex()
                .flex_none()
                .items_center()
                .gap(px(6.))
                .child(
                    div()
                        .flex_none()
                        .w(px(32.))
                        .text_xs()
                        .text_color(cx.theme().muted_foreground)
                        .child(number),
                )
                .child(
                    div()
                        .flex_none()
                        .w(px(28.))
                        .text_xs()
                        .text_color(cx.theme().muted_foreground)
                        .child(updated),
                )
                .child(
                    div()
                        .flex_none()
                        .w(px(14.))
                        .flex()
                        .items_center()
                        .justify_center()
                        .child(
                            Icon::new(review_status_icon(status_label, item.is_draft))
                                .xsmall()
                                .text_color(status_color),
                        ),
                ),
        )
        .child(
            div()
                .flex_1()
                .min_w_0()
                .text_sm()
                .truncate()
                .child(item.title.clone()),
        )
        .child(repo_pill(&short, cx));

    for label in item.labels.iter().take(2) {
        row = row.child(label_pill(&label.name, &label.color, cx));
    }

    row.child(
        div()
            .size(px(20.))
            .rounded_full()
            .bg(author_color(&item.author_login, cx))
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

fn label_pill(name: &str, hex: &str, cx: &App) -> impl IntoElement {
    let color = provider_label_color(name, hex, cx);
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
        .child(div().size(px(6.)).rounded_full().bg(color))
        .child(
            div()
                .max_w(px(120.))
                .text_xs()
                .text_color(color)
                .truncate()
                .child(name.to_string()),
        )
}

fn is_review_status_label(label: &str) -> bool {
    matches!(
        label,
        "Review required"
            | "Approved"
            | "Changes requested"
            | "Draft"
            | "Commented"
            | "Dismissed"
            | "Unknown"
    )
}

fn review_status_color(state: &str, is_draft: bool, cx: &App) -> gpui_kit::Hsla {
    if is_draft || state == "Draft" {
        return cx.theme().muted_foreground;
    }
    match state {
        "Approved" => cx.theme().success,
        "Changes requested" => cx.theme().danger,
        "Review required" => cx.theme().warning,
        _ => cx.theme().muted_foreground,
    }
}

fn review_status_icon(state: &str, is_draft: bool) -> IconName {
    if is_draft || state == "Draft" {
        return IconName::Square;
    }
    match state {
        "Approved" => IconName::CircleCheck,
        "Changes requested" => IconName::CircleAlert,
        "Review required" => IconName::LoaderCircle,
        _ => IconName::Dash,
    }
}

fn provider_label_color(name: &str, hex: &str, cx: &App) -> gpui_kit::Hsla {
    let _ = hex;
    match name {
        "wayfinder:prototype" => cx.theme().accent,
        "wayfinder:grilling" => cx.theme().warning,
        "billing" => cx.theme().success,
        "design" | "ui" => cx.theme().info,
        "core" => cx.theme().danger.opacity(0.75),
        "shell" => cx.theme().accent.opacity(0.8),
        "infra" => cx.theme().muted_foreground,
        _ => {
            let n = name
                .bytes()
                .fold(0u32, |acc, b| acc.wrapping_add(u32::from(b)));
            match n % 5 {
                0 => cx.theme().accent,
                1 => cx.theme().success,
                2 => cx.theme().warning,
                3 => cx.theme().info,
                _ => cx.theme().link,
            }
        }
    }
}

fn author_color(author: &str, cx: &App) -> gpui_kit::Hsla {
    match author.chars().next().unwrap_or('a').to_ascii_lowercase() {
        'a'..='e' => cx.theme().accent,
        'f'..='j' => cx.theme().success,
        'k'..='o' => cx.theme().warning,
        _ => cx.theme().info,
    }
}

fn pr_number_label(id: &reviewdesk_core::PullRequestId) -> String {
    id.as_str()
        .rsplit_once('#')
        .map(|(_, n)| format!("#{n}"))
        .unwrap_or_else(|| "#?".into())
}

fn short_repo_name(full: &str) -> String {
    full.rsplit_once('/')
        .map(|(_, name)| name.to_string())
        .unwrap_or_else(|| full.to_string())
}

fn relative_time(at: chrono::DateTime<chrono::Utc>) -> String {
    let now = chrono::Utc::now();
    let Ok(delta) = now.signed_duration_since(at).to_std() else {
        return "—".into();
    };
    let secs = delta.as_secs();
    if secs < 60 {
        return "now".into();
    }
    let mins = secs / 60;
    if mins < 60 {
        return format!("{mins}m");
    }
    let hours = mins / 60;
    if hours < 48 {
        return format!("{hours}h");
    }
    let days = hours / 24;
    if days < 14 {
        return format!("{days}d");
    }
    format!("{}w", days / 7)
}
