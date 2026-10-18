//! Compact Check rollup control for Metadata and folded strip.

use gpui_kit::component::{
    ActiveTheme as _, Icon, IconName, Sizable as _, h_flex, v_flex,
};
use gpui_kit::{
    Context, InteractiveElement as _, IntoElement, ParentElement as _,
    StatefulInteractiveElement as _, Styled as _, div, px,
};

use reviewdesk_core::{CheckRollupState, PipelinesSnapshot};

use crate::app::ReviewDesk;
use crate::review_session::{Intent, LoadState, SessionView};

pub(crate) fn render_compact_check_rollup(
    view: &SessionView,
    cx: &mut Context<ReviewDesk>,
) -> impl IntoElement {
    let (status, color, icon) = match &view.pipelines {
        LoadState::Ready(snapshot) => rollup_from_snapshot(snapshot, cx),
        LoadState::Loading | LoadState::Idle => (
            "Loading…".to_string(),
            cx.theme().muted_foreground,
            IconName::LoaderCircle,
        ),
        LoadState::Error(_) => (
            "Unavailable".to_string(),
            cx.theme().danger,
            IconName::CircleX,
        ),
    };

    div()
        .id("metadata-pipelines-rollup")
        .cursor_pointer()
        .rounded_md()
        .px_1()
        .py(px(2.))
        .hover(|s| s.bg(cx.theme().secondary.opacity(0.45)))
        .on_click(cx.listener(|this, _, _, cx| {
            this.dispatch(Intent::ActivatePipelines, cx);
        }))
        .child(
            v_flex()
                .gap_0()
                .child(
                    div()
                        .text_xs()
                        .text_color(cx.theme().muted_foreground)
                        .child("Pipelines"),
                )
                .child(
                    h_flex()
                        .items_center()
                        .gap_1()
                        .child(Icon::new(icon).xsmall().text_color(color))
                        .child(div().text_sm().text_color(color).child(status)),
                ),
        )
}

fn rollup_from_snapshot(
    snapshot: &PipelinesSnapshot,
    cx: &mut Context<ReviewDesk>,
) -> (String, gpui_kit::Hsla, IconName) {
    let color = match snapshot.rollup.state {
        CheckRollupState::Success => cx.theme().success,
        CheckRollupState::Pending => cx.theme().warning,
        CheckRollupState::Failure | CheckRollupState::Error => cx.theme().danger,
        CheckRollupState::Absent => cx.theme().muted_foreground,
    };
    let icon = match snapshot.rollup.state {
        CheckRollupState::Success => IconName::CircleCheck,
        CheckRollupState::Pending => IconName::LoaderCircle,
        CheckRollupState::Failure | CheckRollupState::Error => IconName::CircleX,
        CheckRollupState::Absent => IconName::Square,
    };
    (snapshot.rollup.label.clone(), color, icon)
}
