use std::collections::HashSet;
use std::rc::Rc;
use std::sync::Arc;

use gpui_kit::component::{
    ActiveTheme as _, Sizable as _,
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
use crate::ui::file_icons::{dir_icons, file_icon};
use crate::ui::file_tree::{TreeRowKind, visible_file_tree_rows};
use crate::ui::pending_review::render_pending_review_bar;
use crate::ui::render_diff_thread_annotation;

const TREE_ROW_BASE_PL: f32 = 8.;
const TREE_NEST_STEP: f32 = 12.;

fn tree_row_pl(depth: u8) -> f32 {
    TREE_ROW_BASE_PL + TREE_NEST_STEP * f32::from(depth)
}

pub(crate) struct ChangesListChrome<'a> {
    pub scroll: &'a UniformListScrollHandle,
    pub collapsed_dirs: &'a HashSet<String>,
}

pub(crate) fn render_files(
    view: &SessionView,
    diff: &Entity<DiffState>,
    chrome: ChangesListChrome<'_>,
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

    let files_scroll = chrome.scroll;
    let file_paths = view.file_paths.clone();
    let tree_rows = Arc::new(visible_file_tree_rows(
        file_paths.as_ref(),
        chrome.collapsed_dirs,
    ));
    let selected_file = view.selected_file.clone();
    let focused_file = view.focused_file.clone();
    let selected_file_error = view.selected_file_error.clone();
    let mode = view.diff_mode;
    let mode_label = match mode {
        DiffMode::Split => "Unified",
        DiffMode::Unified => "Split",
        _ => "Toggle",
    };
    let single_file_focus = focused_file.is_some();

    let annotations = diff_annotations_for_surface(
        view.shared_threads.as_ref(),
        focused_file.as_deref(),
        file_paths.as_ref(),
    );
    let thread_cards = Rc::new(
        diff_threads_for_surface(
            view.shared_threads.as_ref(),
            focused_file.as_deref(),
            file_paths.as_ref(),
        )
        .iter()
        .filter_map(thread_card_entry)
        .collect::<Vec<_>>(),
    );
    let add_entity = entity.clone();
    let pending_entity = entity.clone();

    // Diff needs a bounded flex_1().min_h_0() column (gpui-kit Diff tests);
    // size_full alone under a row-flex resizable panel can leave ListState
    // with a 0-height viewport on the first Changes paint — no scroll until
    // a later reflow.
    v_flex()
        .size_full()
        .min_h_0()
        .child(
            div().flex_1().min_h_0().w_full().child(
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
                                        .child({
                                            let chrome_entity = entity.clone();
                                            let label = if single_file_focus {
                                                "ALL FILES"
                                            } else {
                                                "FILES"
                                            };
                                            Button::new("files-chrome")
                                                .ghost()
                                                .xsmall()
                                                .label(label)
                                                .on_click(move |_, _, cx| {
                                                    let _ = chrome_entity.update(cx, |this, cx| {
                                                        this.dispatch(Intent::ShowAllFiles, cx);
                                                    });
                                                })
                                        })
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
                                    let focused_file = focused_file.clone();
                                    let file_entity = entity.clone();
                                    let row_count = tree_rows.len();
                                    let rows = tree_rows.clone();
                                    div()
                                        .flex_1()
                                        .min_h_0()
                                        .relative()
                                        .child(
                                            uniform_list(
                                                "file-tree",
                                                row_count,
                                                move |range, _, cx| {
                                                    range
                                                        .map(|ix| {
                                                            let row = &rows[ix];
                                                            let owner = file_entity.clone();
                                                            match row.kind {
                                                                TreeRowKind::Dir => {
                                                                    let path = row.path.clone();
                                                                    let (chevron, folder) =
                                                                        dir_icons(row.expanded);
                                                                    ListItem::new(
                                                                        SharedString::from(
                                                                            format!("dir:{path}"),
                                                                        ),
                                                                    )
                                                                    .px_1()
                                                                    .py_1()
                                                                    .pl(px(tree_row_pl(row.depth)))
                                                                    .child(
                                                                        h_flex()
                                                                            .w_full()
                                                                            .items_center()
                                                                            .gap_1()
                                                                            .min_w_0()
                                                                            .child(
                                                                                chevron.text_color(
                                                                                    cx.theme()
                                                                                        .muted_foreground,
                                                                                ),
                                                                            )
                                                                            .child(
                                                                                folder.text_color(
                                                                                    cx.theme()
                                                                                        .muted_foreground,
                                                                                ),
                                                                            )
                                                                            .child(
                                                                                div()
                                                                                    .text_sm()
                                                                                    .truncate()
                                                                                    .child(
                                                                                        row.name
                                                                                            .clone(),
                                                                                    ),
                                                                            ),
                                                                    )
                                                                    .on_click(move |_, _, cx| {
                                                                        let path = path.clone();
                                                                        let _ = owner.update(
                                                                            cx,
                                                                            |this, cx| {
                                                                                this.toggle_collapsed_file_dir(
                                                                                    path, cx,
                                                                                );
                                                                            },
                                                                        );
                                                                    })
                                                                    .into_any_element()
                                                                }
                                                                TreeRowKind::File => {
                                                                    let path = SharedString::from(
                                                                        row.path.clone(),
                                                                    );
                                                                    let selected = selected_file
                                                                        .as_ref()
                                                                        == Some(&path)
                                                                        || focused_file.as_ref()
                                                                            == Some(&path);
                                                                    let icon =
                                                                        file_icon(&row.name);
                                                                    ListItem::new(path.clone())
                                                                        .selected(selected)
                                                                        .px_1()
                                                                        .py_1()
                                                                        .pl(px(tree_row_pl(
                                                                            row.depth,
                                                                        )))
                                                                        .child(
                                                                            h_flex()
                                                                                .w_full()
                                                                                .items_center()
                                                                                .gap_1()
                                                                                .min_w_0()
                                                                                .child(
                                                                                    // Chevron gutter so files align with dir labels.
                                                                                    div().w(px(12.)),
                                                                                )
                                                                                .child(
                                                                                    icon.text_color(
                                                                                        cx.theme()
                                                                                            .muted_foreground,
                                                                                    ),
                                                                                )
                                                                                .child(
                                                                                    div()
                                                                                        .text_sm()
                                                                                        .truncate()
                                                                                        .child(
                                                                                            row.name
                                                                                                .clone(),
                                                                                        ),
                                                                                ),
                                                                        )
                                                                        .on_click(
                                                                            move |event, _, cx| {
                                                                                let path =
                                                                                    path.clone();
                                                                                let focus = event
                                                                                    .modifiers()
                                                                                    .secondary();
                                                                                let _ = owner
                                                                                    .update(
                                                                                    cx,
                                                                                    |this, cx| {
                                                                                        if focus {
                                                                                            this.dispatch(
                                                                                                Intent::FocusFile(path),
                                                                                                cx,
                                                                                            );
                                                                                        } else {
                                                                                            this.dispatch(
                                                                                                Intent::SelectFile(path),
                                                                                                cx,
                                                                                            );
                                                                                        }
                                                                                    },
                                                                                );
                                                                            },
                                                                        )
                                                                        .into_any_element()
                                                                }
                                                            }
                                                        })
                                                        .collect()
                                                },
                                            )
                                            .size_full()
                                            .track_scroll(files_scroll),
                                        )
                                        .child(Scrollbar::vertical(files_scroll))
                                }),
                        ),
                )
                .child({
                    let retry_path = focused_file.clone();
                    let mut diff_column = v_flex().size_full().min_h_0().min_w_0();
                    if view.has_pending_review {
                        diff_column = diff_column
                            .child(render_pending_review_bar(pending_entity.clone(), cx));
                    }
                    resizable_panel().child(diff_column.child(
                        div()
                            .flex_1()
                            .min_h_0()
                            .w_full()
                            .p_2()
                            .overflow_hidden()
                            .child(match selected_file_error {
                                Some(error) => {
                                    let path = retry_path.clone();
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
                                                        this.dispatch(Intent::RetryFile(path), cx);
                                                    });
                                                }),
                                        )
                                        .into_any_element()
                                }
                                None => Diff::new(diff)
                                    .size_full()
                                    .min_h_0()
                                    // Soft-wrap ties row height to viewport width; the
                                    // vertical scrollbar then thrash-notifies and can
                                    // clamp scroll. Prefer horizontal overflow instead.
                                    .soft_wrap(false)
                                    .annotations(annotations)
                                    .render_annotation({
                                        let cards = thread_cards.clone();
                                        move |annotation, _, cx| {
                                            let Some(card) =
                                                cards.iter().find(|(id, _)| id == annotation.id())
                                            else {
                                                return div().into_any_element();
                                            };
                                            render_diff_thread_annotation(
                                                card.1.author.as_ref(),
                                                card.1.body.as_ref(),
                                                card.1.is_pending,
                                                card.1.suggestion.as_deref(),
                                                cx,
                                            )
                                            .into_any_element()
                                        }
                                    })
                                    .on_add_annotation(move |range, _, _cx| {
                                        let _ = add_entity.update(_cx, |this, cx| {
                                            this.dispatch(
                                                Intent::AddDiffReviewComment(range.clone()),
                                                cx,
                                            );
                                        });
                                    })
                                    .into_any_element(),
                            }),
                    ))
                }),
            ),
        )
        .into_any_element()
}

fn diff_threads_for_surface(
    threads: &[ThreadView],
    focused_file: Option<&str>,
    file_paths: &[SharedString],
) -> Vec<ThreadView> {
    threads
        .iter()
        .filter(|thread| {
            let Some(anchor) = thread.anchor.as_ref() else {
                return false;
            };
            if let Some(path) = focused_file {
                return anchor.path == path;
            }
            file_paths.iter().any(|path| path.as_str() == anchor.path)
        })
        .cloned()
        .collect()
}

fn diff_annotations_for_surface(
    threads: &[ThreadView],
    focused_file: Option<&str>,
    file_paths: &[SharedString],
) -> Vec<DiffAnnotation> {
    diff_threads_for_surface(threads, focused_file, file_paths)
        .iter()
        .filter_map(diff_annotation_for_thread)
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
    author: SharedString,
    body: SharedString,
    is_pending: bool,
    suggestion: Option<SharedString>,
}

fn thread_card_entry(thread: &ThreadView) -> Option<(ElementId, ThreadCardData)> {
    let comment = thread.comments.first()?;
    Some((
        ElementId::from(SharedString::from(thread.id.clone())),
        ThreadCardData {
            author: comment.author_login.clone(),
            body: comment.body.clone(),
            is_pending: comment.is_pending,
            suggestion: comment.suggestion.clone(),
        },
    ))
}
