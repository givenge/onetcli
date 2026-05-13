use gpui::{
    App, Div, ElementId, InteractiveElement as _, Stateful, Styled as _, div,
    prelude::FluentBuilder as _, px,
};
use gpui_component::{ActiveTheme, h_flex, v_flex};

pub const RESOURCE_PANEL_WIDTH: f32 = 260.0;
pub const CONTEXT_PANEL_WIDTH: f32 = 360.0;
pub const SIDE_TOOLBAR_BUTTON_SIZE: f32 = 36.0;
pub const SIDE_TOOLBAR_WIDTH: f32 = 44.0;
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

pub fn side_toolbar(cx: &App) -> Div {
    v_flex()
        .flex_shrink_0()
        .w(px(SIDE_TOOLBAR_WIDTH))
        .h_full()
        .bg(cx.theme().muted)
        .border_l_1()
        .border_color(cx.theme().border)
        .items_center()
        .py_2()
        .gap_1()
}

pub fn side_toolbar_button(id: impl Into<ElementId>, is_active: bool, cx: &App) -> Stateful<Div> {
    div()
        .id(id)
        .size(px(SIDE_TOOLBAR_BUTTON_SIZE))
        .flex()
        .items_center()
        .justify_center()
        .rounded_md()
        .cursor_pointer()
        .when(is_active, |this| this.bg(cx.theme().accent))
        .when(!is_active, |this| {
            this.hover(|style| style.bg(cx.theme().muted))
        })
}

pub fn split_border(cx: &App) -> Div {
    div().w(px(1.0)).h_full().bg(cx.theme().border)
}
