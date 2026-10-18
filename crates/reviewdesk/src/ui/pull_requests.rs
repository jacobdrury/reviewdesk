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
    CiStatusFilter, DraftFilter, PullRequestListRow, PullRequestsEmptyKind, ReviewState,
    UpdatedWithin, review_state_label, visible_list_rows,
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
        .child(render_filter_bar(pane, entity.clone(), cx))
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
                        .child(pane.sync.status_label()),
                ),
        )
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

    tabs.child(div().flex_1()).child(
        h_flex()
            .gap_1()
            .pb(px(6.))
            .child(
                Button::new("view-move-left")
                    .ghost()
                    .xsmall()
                    .label("←")
                    .on_click({
                        let entity = entity.clone();
                        move |_, _, cx| {
                            let _ = entity.update(cx, |this, cx| {
                                this.dispatch(Intent::MoveActiveViewLeft, cx);
                            });
                        }
                    }),
            )
            .child(
                Button::new("view-move-right")
                    .ghost()
                    .xsmall()
                    .label("→")
                    .on_click({
                        let entity = entity.clone();
                        move |_, _, cx| {
                            let _ = entity.update(cx, |this, cx| {
                                this.dispatch(Intent::MoveActiveViewRight, cx);
                            });
                        }
                    }),
            )
            .when(pane.view_dirty, |el| {
            el.child(
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
            )
            }),
    )
}

fn render_filter_bar(
    pane: &crate::pull_requests_pane::PullRequestsPaneView,
    entity: WeakEntity<ReviewDesk>,
    cx: &mut Context<ReviewDesk>,
) -> impl IntoElement {
    let filter = &pane.working_filter;
    let can_rename = pane.views.iter().any(|view| {
        pane.active_view_id.as_ref() == Some(&view.id) && view.can_rename()
    });

    let drafts_label = match filter.drafts {
        DraftFilter::All => "Drafts: all",
        DraftFilter::ReadyOnly => "Drafts: ready",
        DraftFilter::DraftsOnly => "Drafts: only",
    };
    let updated_label = match filter.updated_within {
        UpdatedWithin::Any => "Updated: any",
        UpdatedWithin::Last24h => "Updated: 24h",
        UpdatedWithin::Last7d => "Updated: 7d",
        UpdatedWithin::Last30d => "Updated: 30d",
    };

    v_flex()
        .w_full()
        .border_b_1()
        .border_color(cx.theme().border)
        .child(
            h_flex()
                .w_full()
                .px_4()
                .py_2()
                .items_center()
                .gap_2()
                .flex_wrap()
                .child(
                    div()
                        .text_sm()
                        .text_color(cx.theme().muted_foreground)
                        .child("Filter"),
                )
                .when_some(pane.active_view_name.as_ref(), |el, name| {
                    el.child(filter_chip("View", name, cx))
                })
                .when_some(pane.repo_scope.as_ref(), |el, repo| {
                    el.child(filter_chip("Repository", repo, cx))
                })
                .child(toggle_filter_chip(
                    "Author: me",
                    filter.i_am_author,
                    entity.clone(),
                    cx,
                    |this, cx| this.dispatch(Intent::ToggleWorkingFilterAuthor, cx),
                ))
                .child(toggle_filter_chip(
                    "Reviewer: me",
                    filter.i_am_reviewer,
                    entity.clone(),
                    cx,
                    |this, cx| this.dispatch(Intent::ToggleWorkingFilterReviewer, cx),
                ))
                .child(cycle_filter_chip(
                    drafts_label.to_string(),
                    entity.clone(),
                    cx,
                    |this, cx| this.dispatch(Intent::CycleWorkingFilterDrafts, cx),
                ))
                .child(cycle_filter_chip(
                    updated_label.to_string(),
                    entity.clone(),
                    cx,
                    |this, cx| this.dispatch(Intent::CycleWorkingFilterUpdatedWithin, cx),
                ))
                .child(toggle_filter_chip(
                    "Review: pending",
                    filter.review_states.contains(&ReviewState::Pending),
                    entity.clone(),
                    cx,
                    |this, cx| {
                        this.dispatch(
                            Intent::ToggleWorkingFilterReviewState {
                                state: ReviewState::Pending,
                            },
                            cx,
                        )
                    },
                ))
                .child(disabled_filter_chip(
                    "Assignee",
                    "not cached yet",
                    cx,
                ))
                .child(disabled_filter_chip(
                    "CI status",
                    match filter.ci_status {
                        CiStatusFilter::Any => "not cached yet",
                        _ => "checks not on list rows",
                    },
                    cx,
                )),
        )
        .child(
            h_flex()
                .w_full()
                .px_4()
                .pb_2()
                .items_center()
                .gap_2()
                .child(div().flex_1())
                .child(
                    div()
                        .text_xs()
                        .text_color(cx.theme().muted_foreground)
                        .child(format!("Display · {}", pane.group_by_label)),
                )
                .child(
                    Button::new("view-rename")
                        .ghost()
                        .xsmall()
                        .label("Rename")
                        .disabled(!can_rename)
                        .on_click(cx.listener(|this, _, _, cx| {
                            this.dispatch(Intent::RequestRenameActiveView, cx);
                        })),
                )
        .child(
            Button::new("view-save-as")
                .ghost()
                .xsmall()
                .label("Save as new")
                .on_click(cx.listener(|this, _, _, cx| {
                    let name = this
                        .session
                        .view()
                        .pull_requests_pane
                        .active_view_name
                        .unwrap_or_else(|| "Custom View".into());
                    this.dispatch(Intent::SaveViewAsNew { name }, cx);
                })),
        )
        .when(
            pane.views
                .iter()
                .any(|v| pane.active_view_id.as_ref() == Some(&v.id) && v.can_delete()),
            |el| {
                el.child(
                    Button::new("view-delete")
                        .ghost()
                        .xsmall()
                        .label("Delete")
                        .on_click(cx.listener(|this, _, _, cx| {
                            this.dispatch(Intent::DeleteActiveView, cx);
                        })),
                )
            },
        ),
        )
}

fn filter_chip(field: &str, value: &str, cx: &mut Context<ReviewDesk>) -> impl IntoElement {
    h_flex()
        .items_center()
        .gap_1()
        .px_2()
        .py(px(3.))
        .rounded_md()
        .border_1()
        .border_color(cx.theme().border)
        .child(
            div()
                .text_xs()
                .text_color(cx.theme().muted_foreground)
                .child(field.to_string()),
        )
        .child(
            div()
                .text_xs()
                .font_medium()
                .text_color(cx.theme().foreground)
                .child(value.to_string()),
        )
}

fn toggle_filter_chip<F>(
    label: &str,
    active: bool,
    entity: WeakEntity<ReviewDesk>,
    cx: &mut Context<ReviewDesk>,
    dispatch: F,
) -> impl IntoElement
where
    F: Fn(&mut ReviewDesk, &mut Context<ReviewDesk>) + 'static,
{
    let label = label.to_string();
    Button::new(SharedString::from(format!("filter-{label}")))
        .ghost()
        .xsmall()
        .label(label)
        .when(active, |button| {
            button.bg(cx.theme().secondary.opacity(0.55))
        })
        .on_click(move |_, _, cx| {
            let _ = entity.update(cx, |this, cx| {
                dispatch(this, cx);
            });
        })
}

fn cycle_filter_chip<F>(
    label: String,
    entity: WeakEntity<ReviewDesk>,
    _cx: &mut Context<ReviewDesk>,
    dispatch: F,
) -> impl IntoElement
where
    F: Fn(&mut ReviewDesk, &mut Context<ReviewDesk>) + 'static,
{
    Button::new(SharedString::from(format!("cycle-{label}")))
        .ghost()
        .xsmall()
        .label(label)
        .on_click(move |_, _, cx| {
            let _ = entity.update(cx, |this, cx| {
                dispatch(this, cx);
            });
        })
}

fn disabled_filter_chip(title: &str, reason: &str, cx: &mut Context<ReviewDesk>) -> impl IntoElement {
    h_flex()
        .items_center()
        .gap_1()
        .px_2()
        .py(px(3.))
        .rounded_md()
        .border_1()
        .border_color(cx.theme().border.opacity(0.5))
        .opacity(0.55)
        .child(
            div()
                .text_xs()
                .text_color(cx.theme().muted_foreground)
                .child(title.to_string()),
        )
        .child(
            div()
                .text_xs()
                .text_color(cx.theme().muted_foreground)
                .child(reason.to_string()),
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
                            let chevron = if collapsed.contains(key) {
                                IconName::ChevronRight
                            } else {
                                IconName::ChevronDown
                            };
                            let group_key = Arc::new(key.clone());
                            let owner = entity.clone();
                            div()
                                .id(SharedString::from(format!("group-{ix}")))
                                .w_full()
                                .pl(px(list_row_pl(*depth)))
                                .pr_4()
                                .py(px(6.))
                                .cursor_pointer()
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
                                    h_flex()
                                        .items_center()
                                        .gap_2()
                                        .child(
                                            Icon::new(chevron)
                                                .xsmall()
                                                .text_color(cx.theme().muted_foreground),
                                        )
                                        .child(div().text_sm().font_medium().child(label.clone()))
                                        .child(
                                            div()
                                                .text_sm()
                                                .text_color(cx.theme().muted_foreground)
                                                .child(count.to_string()),
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

fn render_dense_row(item: &reviewdesk_core::PullRequestListItem, cx: &App) -> impl IntoElement {
    let status = review_state_label(item.review_state, item.is_draft);
    h_flex()
        .w_full()
        .items_center()
        .gap_2()
        .child(
            div().flex_none().w(px(14.)).child(
                Icon::new(IconName::CircleCheck)
                    .xsmall()
                    .text_color(cx.theme().muted_foreground),
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
        .child(
            div()
                .flex_none()
                .text_xs()
                .text_color(cx.theme().muted_foreground)
                .child(status),
        )
        .child(
            div()
                .flex_none()
                .text_xs()
                .text_color(cx.theme().muted_foreground)
                .truncate()
                .child(item.repo_label.clone()),
        )
}
