use gpui::{App, Div, Styled as _, div, px};

use crate::{ActiveTheme, h_flex, v_flex};

pub const APP_RAIL_WIDTH: f32 = 52.0;
pub const APP_TOOLBAR_HEIGHT: f32 = 44.0;
pub const PANEL_HEADER_HEIGHT: f32 = 36.0;
pub const DENSE_ROW_HEIGHT: f32 = 30.0;

pub fn app_rail(cx: &App) -> Div {
    v_flex()
        .w(px(APP_RAIL_WIDTH))
        .h_full()
        .flex_shrink_0()
        .items_center()
        .gap_1()
        .py_2()
        .bg(cx.theme().sidebar)
        .border_r_1()
        .border_color(cx.theme().sidebar_border)
}

pub fn app_toolbar(cx: &App) -> Div {
    h_flex()
        .h(px(APP_TOOLBAR_HEIGHT))
        .min_h(px(APP_TOOLBAR_HEIGHT))
        .w_full()
        .items_center()
        .gap_2()
        .px_3()
        .border_b_1()
        .border_color(cx.theme().border)
        .bg(cx.theme().background)
}

pub fn panel_surface(cx: &App) -> Div {
    v_flex()
        .size_full()
        .min_w_0()
        .min_h_0()
        .bg(cx.theme().background)
        .text_color(cx.theme().foreground)
}

pub fn panel_header(cx: &App) -> Div {
    h_flex()
        .h(px(PANEL_HEADER_HEIGHT))
        .min_h(px(PANEL_HEADER_HEIGHT))
        .w_full()
        .items_center()
        .gap_2()
        .px_3()
        .border_b_1()
        .border_color(cx.theme().border)
        .bg(cx.theme().muted)
}

pub fn content_canvas(cx: &App) -> Div {
    div()
        .size_full()
        .min_w_0()
        .min_h_0()
        .overflow_hidden()
        .bg(cx.theme().muted)
}

pub fn status_pill(cx: &App) -> Div {
    h_flex()
        .h_5()
        .items_center()
        .gap_1()
        .px_2()
        .rounded(px(5.0))
        .border_1()
        .border_color(cx.theme().border)
        .bg(cx.theme().secondary)
        .text_xs()
        .text_color(cx.theme().secondary_foreground)
}
