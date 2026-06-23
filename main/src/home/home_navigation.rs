use crate::home::home_layout::{
    NAV_ITEM_GAP, NAV_ITEM_HEIGHT, NAV_ITEM_ICON_SLOT, NAV_ITEM_PADDING_X, NAV_ITEM_RADIUS,
    NAV_LIST_GAP, SIDEBAR_PADDING, SIDEBAR_WIDTH,
};
use crate::home_tab::HomePage;
use gpui::{
    AnyElement, Context, InteractiveElement as _, IntoElement, ParentElement as _, SharedString,
    StatefulInteractiveElement as _, Styled as _, Window, div, prelude::FluentBuilder as _, px,
};
use gpui_component::{
    ActiveTheme, Icon, IconName, Selectable as _, Sizable as _, Size,
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
                    .icon(IconName::ChatDB)
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
        v_flex()
            .w(px(SIDEBAR_WIDTH))
            .h_full()
            .flex_shrink_0()
            .p(px(SIDEBAR_PADDING))
            .border_r_1()
            .border_color(cx.theme().border)
            .bg(cx.theme().sidebar)
            .child(self.render_connection_type_buttons(cx))
            .child(div().flex_1())
    }

    fn render_connection_type_buttons(&mut self, cx: &mut Context<Self>) -> impl IntoElement {
        v_flex().gap(px(NAV_LIST_GAP)).children(
            ConnectionType::all()
                .into_iter()
                .filter(|kind| *kind != ConnectionType::ChatDB)
                .map(|kind| {
                    let selected = self.selected_filter == kind;
                    self.render_connection_type_row(kind, selected, cx)
                }),
        )
    }

    fn render_connection_type_row(
        &self,
        kind: ConnectionType,
        selected: bool,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        h_flex()
            .id(SharedString::from(format!("connection-type-{}", kind)))
            .w_full()
            .h(px(NAV_ITEM_HEIGHT))
            .items_center()
            .gap(px(NAV_ITEM_GAP))
            .px(px(NAV_ITEM_PADDING_X))
            .rounded(px(NAV_ITEM_RADIUS))
            .text_color(cx.theme().muted_foreground)
            .cursor_pointer()
            .when(selected, |this| {
                this.bg(cx.theme().blue.opacity(0.1))
                    .text_color(cx.theme().foreground)
            })
            .when(!selected, |this| {
                this.hover(|style| style.bg(cx.theme().sidebar_accent.opacity(0.55)))
            })
            .child(
                div()
                    .w(px(NAV_ITEM_ICON_SLOT))
                    .h_full()
                    .flex()
                    .items_center()
                    .justify_center()
                    .child(Icon::new(kind.icon()).color().with_size(Size::Medium)),
            )
            .child(
                div()
                    .min_w_0()
                    .text_sm()
                    .font_weight(gpui::FontWeight::MEDIUM)
                    .child(kind.label()),
            )
            .on_click(cx.listener(move |this, _, _, cx| {
                this.selected_filter = kind;
                cx.notify();
            }))
            .into_any_element()
    }
}
