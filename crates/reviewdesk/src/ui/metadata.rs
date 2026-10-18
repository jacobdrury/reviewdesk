//! Metadata right-dock panel and its folded status+merge strip.
//!
//! Reads [`SessionView`] only. Mutations beyond dock visibility (labels,
//! reviewers, merge, …) are out of scope here; write-through lands in #77.

use gpui_kit::component::{
    ActiveTheme as _, Icon, IconName, Sizable as _, StyledExt as _,
    button::{Button, ButtonVariants as _},
    h_flex, v_flex,
};
use gpui_kit::{
    Context, InteractiveElement as _, IntoElement, ParentElement as _,
    StatefulInteractiveElement as _, Styled as _, div, px,
};

use crate::app::ReviewDesk;
use crate::review_session::{Intent, LoadState, SessionView};
use crate::ui::check_rollup::render_compact_check_rollup;

/// Full Metadata dock shown on the right when the dock is `Open`.
pub(crate) fn render_metadata_panel(
    view: &SessionView,
    cx: &mut Context<ReviewDesk>,
) -> impl IntoElement {
    let header = h_flex()
        .w_full()
        .px_3()
        .py(px(4.))
        .items_center()
        .justify_between()
        .border_b_1()
        .border_color(cx.theme().border)
        .child(
            div()
                .text_xs()
                .font_medium()
                .text_color(cx.theme().muted_foreground)
                .child("METADATA"),
        )
        .child(
            h_flex()
                .gap_1()
                .child(
                    Button::new("metadata-fold")
                        .ghost()
                        .xsmall()
                        .icon(IconName::Minus)
                        .on_click(cx.listener(|this, _, _, cx| {
                            this.dispatch(Intent::FoldMetadata, cx);
                        })),
                )
                .child(
                    Button::new("metadata-hide")
                        .ghost()
                        .xsmall()
                        .icon(IconName::EyeOff)
                        .on_click(cx.listener(|this, _, _, cx| {
                            this.dispatch(Intent::HideMetadata, cx);
                        })),
                ),
        );

    v_flex()
        .w(px(320.))
        .min_h_0()
        .border_l_1()
        .border_color(cx.theme().border)
        .child(header)
        .child(div().flex_1().min_h_0().child(metadata_body(view, cx)))
}

fn metadata_body(view: &SessionView, cx: &mut Context<ReviewDesk>) -> impl IntoElement {
    let LoadState::Ready(detail) = &view.detail else {
        return v_flex().p_4().child(
            div()
                .text_sm()
                .text_color(cx.theme().muted_foreground)
                .child("Metadata loads with the pull request."),
        );
    };

    let provider_ref = detail.id.as_str().to_string();
    let web_url = detail.web_url.clone();
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

    v_flex()
        .p_4()
        .gap_3()
        .child(metadata_field(
            "Pull request",
            provider_link(provider_ref, web_url, cx),
            cx,
        ))
        .child(metadata_field(
            "Status",
            status_label(detail.is_draft).into_any_element(),
            cx,
        ))
        .child(metadata_field(
            "Reviewers",
            div().text_sm().child(reviewers).into_any_element(),
            cx,
        ))
        .child(metadata_field(
            "Labels",
            div().text_sm().child(labels).into_any_element(),
            cx,
        ))
        .child(render_compact_check_rollup(view, cx))
}

fn metadata_field(
    label: &'static str,
    value: gpui_kit::AnyElement,
    cx: &mut Context<ReviewDesk>,
) -> impl IntoElement {
    v_flex()
        .gap_1()
        .child(
            div()
                .text_xs()
                .font_medium()
                .text_color(cx.theme().muted_foreground)
                .child(label),
        )
        .child(value)
}

/// Provider link rendered as `owner/repo#n`, opening the Provider web URL.
fn provider_link(
    provider_ref: String,
    web_url: String,
    cx: &mut Context<ReviewDesk>,
) -> gpui_kit::AnyElement {
    h_flex()
        .id("metadata-provider-link")
        .items_center()
        .gap_1()
        .cursor_pointer()
        .rounded_sm()
        .text_color(cx.theme().link)
        .hover(|style| style.text_color(cx.theme().accent))
        .on_click(move |_, _, cx| {
            cx.open_url(&web_url);
        })
        .child(div().text_sm().font_medium().child(provider_ref))
        .child(
            Icon::new(IconName::ExternalLink)
                .xsmall()
                .text_color(cx.theme().muted_foreground),
        )
        .into_any_element()
}

fn status_label(is_draft: bool) -> impl IntoElement {
    let text = if is_draft { "Draft" } else { "Ready" };
    div().text_sm().child(text)
}

/// Folded footer strip: keeps status + merge visible after a fold.
/// Hide leaves no strip; restoring the full dock is via the chrome toggle.
pub(crate) fn render_metadata_strip(
    view: &SessionView,
    cx: &mut Context<ReviewDesk>,
) -> impl IntoElement {
    let is_draft = matches!(&view.detail, LoadState::Ready(detail) if detail.is_draft);
    let status = if is_draft { "Draft" } else { "Ready" };

    h_flex()
        .w_full()
        .px_3()
        .py(px(4.))
        .items_center()
        .justify_between()
        .border_t_1()
        .border_color(cx.theme().border)
        .child(
            h_flex()
                .gap_3()
                .items_center()
                .child(
                    div()
                        .text_xs()
                        .font_medium()
                        .text_color(cx.theme().muted_foreground)
                        .child("METADATA"),
                )
                .child(div().text_xs().child(status))
                .child(render_compact_check_rollup(view, cx)),
        )
        .child(
            Button::new("metadata-strip-expand")
                .ghost()
                .xsmall()
                .icon(IconName::PanelRightOpen)
                .on_click(cx.listener(|this, _, _, cx| {
                    this.dispatch(Intent::ToggleMetadataDock, cx);
                })),
        )
}
