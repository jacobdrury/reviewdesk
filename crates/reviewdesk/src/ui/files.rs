use std::rc::Rc;

use gpui_kit::component::{
    ActiveTheme as _, Sizable as _, StyledExt as _,
    button::{Button, ButtonVariants as _},
    diff::{Diff, DiffAnnotation, DiffLinePosition, DiffMode, DiffSide, DiffState},
    h_flex,
    list::ListItem,
    resizable::{h_resizable, resizable_panel},
    scroll::Scrollbar,
    spinner::Spinner,
    v_flex,
};
use gpui_kit::{
    Context, ElementId, Entity, IntoElement, ParentElement as _, SharedString, Styled as _,
    UniformListScrollHandle, WeakEntity, div, px, uniform_list,
};
use reviewdesk_core::DiffLineSide;

use crate::app::ReviewDesk;
use crate::review_session::{Intent, LoadState, SessionView, ThreadView};
use crate::ui::pending_review::render_pending_review_bar;

pub(crate) fn render_files(
    view: &SessionView,
    diff: &Entity<DiffState>,
    files_scroll: &UniformListScrollHandle,
    entity: WeakEntity<ReviewDesk>,
    cx: &mut Context<ReviewDesk>,
) -> impl IntoElement {
    match &view.patch_index {
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
                        .on_click({
                            let owner = entity.clone();
                            move |_, _, cx| {
                                let _ = owner.update(cx, |this, cx| {
                                    this.dispatch(Intent::RetryPatch, cx);
                                });
                            }
                        }),
                )
                .into_any_element();
        }
        LoadState::Ready(()) => {}
    }

    let file_paths = view.file_paths.clone();
    let selected_file = view.selected_file.clone();
    let selected_file_error = view.selected_file_error.clone();
    let mode = view.diff_mode;
    let mode_label = match mode {
        DiffMode::Split => "Unified",
        DiffMode::Unified => "Split",
        _ => "Toggle",
    };

    let diff_threads = diff_threads_for_file(view.shared_threads.as_ref(), selected_file.as_deref());
    let annotations = diff_threads
        .iter()
        .filter_map(|thread| diff_annotation_for_thread(thread))
        .collect::<Vec<_>>();
    let thread_cards = Rc::new(
        diff_threads
            .iter()
            .filter_map(|thread| thread_card_entry(thread))
            .collect::<Vec<_>>(),
    );
    let add_entity = entity.clone();
    let pending_entity = entity.clone();

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
                                                    this.dispatch(Intent::ToggleDiffMode, cx);
                                                })),
                                        ),
                                )
                                .child({
                                    let selected_file = selected_file.clone();
                                    let file_entity = entity.clone();
                                    div()
                                        .flex_1()
                                        .min_h_0()
                                        .relative()
                                        .child(
                                            uniform_list(
                                                "file-list",
                                                file_paths.len(),
                                                move |range, _, _cx| {
                                                    range
                                                        .map(|ix| {
                                                            let path = file_paths[ix].clone();
                                                            let selected = selected_file.as_ref()
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
                                                                        .child(path.to_string()),
                                                                )
                                                                .on_click(move |_, _, cx| {
                                                                    let path = path.clone();
                                                                    let _ = owner.update(
                                                                        cx,
                                                                        |this, cx| {
                                                                            this.dispatch(
                                                                                Intent::SelectFile(
                                                                                    path,
                                                                                ),
                                                                                cx,
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
                                            .track_scroll(files_scroll),
                                        )
                                        .child(Scrollbar::vertical(files_scroll))
                                })
                        ),
                )
                .child({
                    let retry_path = selected_file.clone();
                    let mut diff_column = v_flex().size_full().min_h_0().min_w_0();
                    if view.has_pending_review {
                        diff_column = diff_column.child(render_pending_review_bar(
                            pending_entity.clone(),
                            cx,
                        ));
                    }
                    resizable_panel().child(
                        diff_column.child(
                                div()
                                    .flex_1()
                                    .min_h_0()
                                    .p_2()
                                    .children(match selected_file_error {
                                        Some(error) => {
                                            let path = retry_path.clone();
                                            vec![
                                                v_flex()
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
                                                        Button::new("file-retry")
                                                            .outline()
                                                            .small()
                                                            .label("Retry")
                                                            .on_click(move |_, _, cx| {
                                                                let Some(path) = path.clone() else {
                                                                    return;
                                                                };
                                                                let _ = entity.update(cx, |this, cx| {
                                                                    this.dispatch(
                                                                        Intent::RetryFile(path),
                                                                        cx,
                                                                    );
                                                                });
                                                            }),
                                                    )
                                                    .into_any_element(),
                                            ]
                                        }
                                        None => vec![
                                            Diff::new(diff)
                                                .size_full()
                                                .soft_wrap(true)
                                                .annotations(annotations)
                                                .render_annotation({
                                                    let cards = thread_cards.clone();
                                                    move |annotation, _, cx| {
                                                        let Some(card) = cards
                                                            .iter()
                                                            .find(|(id, _)| id == annotation.id())
                                                        else {
                                                            return div().into_any_element();
                                                        };
                                                        v_flex()
                                                            .w_full()
                                                            .gap_1()
                                                            .p_2()
                                                            .rounded_md()
                                                            .bg(cx.theme().secondary)
                                                            .child(
                                                                div()
                                                                    .text_xs()
                                                                    .text_color(
                                                                        cx.theme().muted_foreground,
                                                                    )
                                                                    .child(card.1.header.clone()),
                                                            )
                                                            .child(
                                                                div()
                                                                    .text_sm()
                                                                    .child(card.1.body.clone()),
                                                            )
                                                            .children(card.1.suggestion.as_ref().map(
                                                                |suggestion| {
                                                                    div()
                                                                        .text_xs()
                                                                        .text_color(
                                                                            cx.theme()
                                                                                .muted_foreground,
                                                                        )
                                                                        .child(format!(
                                                                            "Suggestion: {suggestion}"
                                                                        ))
                                                                },
                                                            ))
                                                            .into_any_element()
                                                    }
                                                })
                                                .on_add_annotation(move |range, _, _cx| {
                                                    let _ = add_entity.update(_cx, |this, cx| {
                                                        this.dispatch(
                                                            Intent::AddDiffReviewComment(
                                                                range.clone(),
                                                            ),
                                                            cx,
                                                        );
                                                    });
                                                })
                                                .into_any_element(),
                                        ],
                                    }),
                            ),
                    )
                }),
        )
        .into_any_element()
}

fn diff_threads_for_file(threads: &[ThreadView], selected_file: Option<&str>) -> Vec<ThreadView> {
    let Some(path) = selected_file else {
        return Vec::new();
    };
    threads
        .iter()
        .filter(|thread| {
            thread
                .anchor
                .as_ref()
                .is_some_and(|anchor| anchor.path == path)
        })
        .cloned()
        .collect()
}

fn diff_annotation_for_thread(thread: &ThreadView) -> Option<DiffAnnotation> {
    let anchor = thread.anchor.as_ref()?;
    let side = match anchor.side {
        DiffLineSide::Left => DiffSide::Original,
        DiffLineSide::Right => DiffSide::Modified,
    };
    Some(DiffAnnotation::line(
        ElementId::from(SharedString::from(thread.id.clone())),
        DiffLinePosition::new(
            SharedString::from(anchor.path.as_str()),
            side,
            anchor.end_line as usize,
        ),
    ))
}

struct ThreadCardData {
    header: SharedString,
    body: SharedString,
    suggestion: Option<SharedString>,
}

fn thread_card_entry(thread: &ThreadView) -> Option<(ElementId, ThreadCardData)> {
    let comment = thread.comments.first()?;
    let mut header = if comment.is_pending {
        "Draft · Review comment".to_string()
    } else {
        "Review comment".to_string()
    };
    if comment.suggestion.is_some() {
        header.push_str(" · Suggestion");
    }
    Some((
        ElementId::from(SharedString::from(thread.id.clone())),
        ThreadCardData {
            header: SharedString::from(header),
            body: comment.body.clone(),
            suggestion: comment.suggestion.clone(),
        },
    ))
}
