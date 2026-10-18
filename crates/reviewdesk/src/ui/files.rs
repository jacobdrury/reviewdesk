use std::rc::Rc;

use gpui_kit::component::{
    ActiveTheme as _, Sizable as _, StyledExt as _,
    button::{Button, ButtonVariants as _},
    diff::{Diff, DiffAnnotation, DiffLinePosition, DiffMode, DiffState},
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

use crate::app::ReviewDesk;
use crate::review_session::{Intent, LoadState, SessionView};

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

    let annotations = view
        .annotations
        .iter()
        .map(|annotation| {
            DiffAnnotation::line(
                ElementId::from(SharedString::from(annotation.id.clone())),
                DiffLinePosition::new(
                    annotation.range.path().clone(),
                    annotation.range.end_side(),
                    annotation.range.end(),
                ),
            )
        })
        .collect::<Vec<_>>();
    let comments = Rc::new(
        view.annotations
            .iter()
            .map(|a| {
                (
                    ElementId::from(SharedString::from(a.id.clone())),
                    a.body.clone(),
                )
            })
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
                                }),
                        ),
                )
                .child({
                    let retry_path = selected_file.clone();
                    resizable_panel().child(
                        v_flex().size_full().min_h_0().min_w_0().p_2().children(
                            match selected_file_error {
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
                                                this.dispatch(
                                                    Intent::AddAnnotation(range.clone()),
                                                    cx,
                                                );
                                            });
                                        })
                                        .into_any_element(),
                                ],
                            },
                        ),
                    )
                }),
        )
        .into_any_element()
}
