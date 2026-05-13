use gpui::{
    App, AppContext as _, Context, Div, Entity, IntoElement, ParentElement as _, Render,
    Styled as _, Window, div, px,
};
use gpui_component::{
    ActiveTheme, Disableable as _, Icon, IconName, Sizable as _,
    button::{Button, ButtonVariants as _},
    chrome, h_flex,
    input::{Input, InputState},
    tab::{Tab, TabBar},
    v_flex,
};

use crate::section;

pub struct DesignSystemStory {
    search: Entity<InputState>,
}

impl super::Story for DesignSystemStory {
    fn title() -> &'static str {
        "Design System"
    }

    fn description() -> &'static str {
        "Unified desktop chrome, states, and density."
    }

    fn new_view(window: &mut Window, cx: &mut App) -> Entity<impl Render> {
        cx.new(|cx| Self {
            search: cx.new(|cx| InputState::new(window, cx).placeholder("Search connections")),
        })
    }
}

impl DesignSystemStory {
    fn render_chrome(&self, cx: &mut Context<Self>) -> Div {
        h_flex()
            .h(px(220.0))
            .w_full()
            .border_1()
            .border_color(cx.theme().border)
            .rounded(cx.theme().radius_lg)
            .overflow_hidden()
            .child(self.render_rail(cx))
            .child(
                chrome::panel_surface(cx)
                    .child(self.render_toolbar(cx))
                    .child(self.render_canvas(cx)),
            )
    }

    fn render_rail(&self, cx: &mut Context<Self>) -> Div {
        chrome::app_rail(cx)
            .child(Icon::new(IconName::Home).small())
            .child(Icon::new(IconName::Database).small())
            .child(Icon::new(IconName::Settings).small())
    }

    fn render_toolbar(&self, cx: &mut Context<Self>) -> Div {
        chrome::app_toolbar(cx)
            .child(Button::new("new").primary().label("New"))
            .child(Input::new(&self.search).w(px(240.0)))
            .child(div().flex_1())
            .child(chrome::status_pill(cx).child("Synced"))
    }

    fn render_canvas(&self, cx: &mut Context<Self>) -> Div {
        chrome::content_canvas(cx).p_3().child(
            chrome::panel_surface(cx)
                .border_1()
                .border_color(cx.theme().border)
                .rounded(cx.theme().radius)
                .child(chrome::panel_header(cx).child("Panel"))
                .child(div().p_3().child("Dense content area")),
        )
    }

    fn render_controls(&self, _cx: &mut Context<Self>) -> Div {
        v_flex()
            .gap_3()
            .child(
                h_flex()
                    .gap_2()
                    .flex_wrap()
                    .child(Button::new("primary").primary().label("Primary"))
                    .child(Button::new("secondary").label("Secondary"))
                    .child(Button::new("ghost").ghost().label("Ghost"))
                    .child(Button::new("danger").danger().label("Danger"))
                    .child(Button::new("disabled").label("Disabled").disabled(true)),
            )
            .child(
                TabBar::new("tabs")
                    .selected_index(0)
                    .child(Tab::new().label("Data"))
                    .child(Tab::new().label("AI")),
            )
    }
}

impl Render for DesignSystemStory {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        v_flex()
            .size_full()
            .gap_5()
            .child(section("Chrome").child(self.render_chrome(cx)))
            .child(section("Controls").child(self.render_controls(cx)))
    }
}
