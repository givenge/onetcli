use gpui::{App, Div, Styled as _, div, px};
use gpui_component::{ActiveTheme, h_flex, v_flex};

pub const RESOURCE_PANEL_WIDTH: f32 = 260.0;
pub const CONTEXT_PANEL_WIDTH: f32 = 360.0;
pub const WORKBENCH_TOOLBAR_HEIGHT: f32 = 40.0;

pub fn workbench_root(cx: &App) -> Div {
    h_flex()
        .size_full()
        .min_w_0()
        .min_h_0()
        .bg(cx.theme().background)
        .text_color(cx.theme().foreground)
}

pub fn resource_panel(cx: &App) -> Div {
    v_flex()
        .w(px(RESOURCE_PANEL_WIDTH))
        .h_full()
        .min_w(px(180.0))
        .flex_shrink_0()
        .border_r_1()
        .border_color(cx.theme().border)
        .bg(cx.theme().sidebar)
}

pub fn main_panel(cx: &App) -> Div {
    v_flex()
        .flex_1()
        .size_full()
        .min_w_0()
        .min_h_0()
        .bg(cx.theme().background)
}

pub fn context_panel(cx: &App) -> Div {
    v_flex()
        .w(px(CONTEXT_PANEL_WIDTH))
        .h_full()
        .min_w(px(280.0))
        .flex_shrink_0()
        .border_l_1()
        .border_color(cx.theme().border)
        .bg(cx.theme().background)
}

pub fn workbench_toolbar(cx: &App) -> Div {
    h_flex()
        .h(px(WORKBENCH_TOOLBAR_HEIGHT))
        .min_h(px(WORKBENCH_TOOLBAR_HEIGHT))
        .w_full()
        .items_center()
        .gap_2()
        .px_3()
        .border_b_1()
        .border_color(cx.theme().border)
        .bg(cx.theme().muted)
}

pub fn split_border(cx: &App) -> Div {
    div().w(px(1.0)).h_full().bg(cx.theme().border)
}
