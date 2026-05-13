use crate::home::connection_display::{connection_icon, connection_subtitle};
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
        .top(px(6.0))
        .left(px(6.0))
        .w(px(10.0))
        .h(px(10.0))
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
        .gap_2()
        .w_full()
        .child(
            div()
                .h(px(48.0))
                .rounded(px(8.0))
                .flex()
                .items_center()
                .justify_center()
                .child(connection_icon(&conn).with_size(px(40.0))),
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
        .gap_0p5()
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
                    .overflow_hidden()
                    .text_ellipsis()
                    .whitespace_nowrap()
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
                .overflow_hidden()
                .text_ellipsis()
                .whitespace_nowrap()
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
