use crate::home::connection_display::{connection_icon, connection_subtitle};
use crate::home::home_layout::{
    ACTIVE_DOT_INSET, ACTIVE_DOT_SIZE, CARD_BODY_GAP, CARD_ICON_SIZE, CARD_ICON_TILE_SIZE,
    CARD_RADIUS, CARD_TEXT_GAP,
};
use crate::home_tab::HomePage;
use gpui::{
    Context, InteractiveElement as _, IntoElement, ParentElement as _, SharedString,
    StatefulInteractiveElement as _, Styled as _, div, prelude::FluentBuilder as _, px,
};
use gpui_component::{ActiveTheme, Sizable as _, tooltip::Tooltip};
use gpui_component::{h_flex, v_flex};
use one_core::storage::StoredConnection;
use rust_i18n::t;

pub(super) fn active_dot(cx: &mut Context<HomePage>) -> impl IntoElement {
    div()
        .absolute()
        .top(px(ACTIVE_DOT_INSET))
        .left(px(ACTIVE_DOT_INSET))
        .w(px(ACTIVE_DOT_SIZE))
        .h(px(ACTIVE_DOT_SIZE))
        .rounded_full()
        .bg(cx.theme().success)
        .shadow_lg()
}

pub(super) fn connection_card_body(
    conn: StoredConnection,
    has_team: bool,
    cx: &mut Context<HomePage>,
) -> impl IntoElement {
    h_flex()
        .items_center()
        .gap(px(CARD_BODY_GAP))
        .w_full()
        .child(
            div()
                .size(px(CARD_ICON_TILE_SIZE))
                .rounded(px(CARD_RADIUS))
                .flex()
                .items_center()
                .justify_center()
                .bg(cx.theme().muted)
                .border_1()
                .border_color(cx.theme().border.opacity(0.7))
                .child(connection_icon(&conn).with_size(px(CARD_ICON_SIZE))),
        )
        .child(connection_card_text(conn, has_team, cx))
}

fn connection_card_text(
    conn: StoredConnection,
    has_team: bool,
    cx: &mut Context<HomePage>,
) -> impl IntoElement {
    v_flex()
        .flex_1()
        .min_w_0()
        .gap(px(CARD_TEXT_GAP))
        .overflow_hidden()
        .child(connection_name_row(&conn, has_team, cx))
        .when_some(connection_subtitle(&conn), |this, conn_info| {
            let tooltip_text: SharedString = conn_info.clone().into();
            this.child(
                div()
                    .id(SharedString::from(format!(
                        "conn-info-{}",
                        conn.id.unwrap_or(0)
                    )))
                    .text_xs()
                    .text_color(cx.theme().muted_foreground)
                    .whitespace_normal()
                    .line_clamp(2)
                    .max_w_full()
                    .tooltip(move |window, cx| Tooltip::new(tooltip_text.clone()).build(window, cx))
                    .child(conn_info),
            )
        })
}

fn connection_name_row(
    conn: &StoredConnection,
    has_team: bool,
    cx: &mut Context<HomePage>,
) -> impl IntoElement {
    let name_tooltip: SharedString = conn.name.clone().into();
    h_flex()
        .items_center()
        .gap_1()
        .overflow_hidden()
        .child(
            div()
                .id(SharedString::from(format!(
                    "conn-name-{}",
                    conn.id.unwrap_or(0)
                )))
                .text_sm()
                .font_weight(gpui::FontWeight::SEMIBOLD)
                .text_color(cx.theme().foreground)
                .whitespace_normal()
                .line_clamp(2)
                .flex_shrink()
                .min_w_0()
                .tooltip(move |window, cx| Tooltip::new(name_tooltip.clone()).build(window, cx))
                .child(conn.name.clone()),
        )
        .when(has_team, |this| {
            this.child(
                div()
                    .flex_shrink_0()
                    .px_1()
                    .rounded(px(3.0))
                    .bg(cx.theme().accent.opacity(0.15))
                    .text_color(cx.theme().accent)
                    .text_xs()
                    .child(t!("Home.team_badge").to_string()),
            )
        })
}
