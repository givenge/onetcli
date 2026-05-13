use crate::home_tab::HomePage;
use crate::setting_tab::GlobalCurrentUser;
use crate::user_avatar::render_user_avatar;
use gpui::{Context, IntoElement, ParentElement as _, Styled as _, Window, div, px};
use gpui_component::{
    ActiveTheme, Icon, IconName, Selectable as _,
    button::{Button, ButtonVariants as _},
    chrome, h_flex, v_flex,
};
use one_core::storage::ConnectionType;
use rust_i18n::t;

impl HomePage {
    pub(crate) fn render_home_navigation(
        &mut self,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        h_flex()
            .h_full()
            .child(self.render_global_rail(window, cx))
            .child(self.render_connection_type_sidebar(window, cx))
    }

    fn render_global_rail(
        &mut self,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        chrome::app_rail(cx)
            .child(
                Button::new("rail-home")
                    .icon(IconName::Home)
                    .ghost()
                    .selected(true)
                    .tooltip(t!("Home.title")),
            )
            .child(
                Button::new("rail-ai")
                    .icon(IconName::AI)
                    .ghost()
                    .tooltip("ChatDB")
                    .on_click(cx.listener(|this, _, window, cx| {
                        this.add_ai_chat_tab(window, cx);
                    })),
            )
            .child(div().flex_1())
            .child(
                Button::new("rail-settings")
                    .icon(IconName::Settings)
                    .ghost()
                    .tooltip(t!("Common.settings"))
                    .on_click(cx.listener(|this, _, window, cx| {
                        this.add_settings_tab(window, cx);
                    })),
            )
    }

    fn render_connection_type_sidebar(
        &mut self,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let global_user = GlobalCurrentUser::get_user(cx);
        if global_user.is_none() && self.current_user.is_some() {
            self.current_user = None;
        }

        v_flex()
            .w(px(184.0))
            .h_full()
            .flex_shrink_0()
            .gap_1()
            .p_2()
            .border_r_1()
            .border_color(cx.theme().border)
            .bg(cx.theme().sidebar)
            .child(self.render_connection_type_buttons(cx))
            .child(div().flex_1())
            .child(self.render_navigation_user(cx))
    }

    fn render_connection_type_buttons(&mut self, cx: &mut Context<Self>) -> impl IntoElement {
        v_flex().gap_1().children(
            ConnectionType::all()
                .into_iter()
                .filter(|kind| *kind != ConnectionType::ChatDB)
                .map(|kind| {
                    let selected = self.selected_filter == kind;
                    Button::new(kind.label())
                        .ghost()
                        .selected(selected)
                        .w_full()
                        .justify_start()
                        .icon(Icon::new(kind.icon()).color())
                        .label(kind.label())
                        .on_click(cx.listener(move |this, _, _, cx| {
                            this.selected_filter = kind;
                            cx.notify();
                        }))
                }),
        )
    }

    fn render_navigation_user(&mut self, cx: &mut Context<Self>) -> impl IntoElement {
        let user = self.current_user.as_ref();
        let view = cx.entity();

        v_flex()
            .w_full()
            .pt_2()
            .border_t_1()
            .border_color(cx.theme().border)
            .child(render_user_avatar(
                user,
                view,
                |this: &mut HomePage, window, cx| {
                    if this.current_user.is_none() {
                        this.show_login_dialog(window, cx);
                    }
                },
                cx,
            ))
    }
}
