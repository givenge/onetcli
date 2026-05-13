use crate::new_connection::NewConnectionWindow;
use gpui::{Context, FontWeight, IntoElement, ParentElement as _, Styled as _, div};
use gpui_component::{ActiveTheme, v_flex};

pub(super) fn card_text(
    label: String,
    description: String,
    cx: &mut Context<NewConnectionWindow>,
) -> impl IntoElement {
    v_flex()
        .flex_1()
        .min_w_0()
        .gap_1()
        .child(
            div()
                .text_sm()
                .font_weight(FontWeight::SEMIBOLD)
                .text_color(cx.theme().foreground)
                .overflow_hidden()
                .text_ellipsis()
                .whitespace_nowrap()
                .child(label),
        )
        .child(
            div()
                .text_xs()
                .text_color(cx.theme().muted_foreground)
                .overflow_hidden()
                .text_ellipsis()
                .whitespace_nowrap()
                .child(description),
        )
}
