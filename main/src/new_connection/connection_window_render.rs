use crate::new_connection::connection_kind::{NewConnectionCategory, NewConnectionKind};
use crate::new_connection::connection_window::{KEY_CONTEXT, NewConnectionWindow};
use crate::new_connection::connection_window_layout::{
    BACK_BUTTON_INSET, CARD_BODY_GAP, CARD_GAP, CARD_HEIGHT, CARD_ICON_SIZE, CARD_ICON_TILE_SIZE,
    CARD_PADDING_X, CARD_PADDING_Y, CARD_RADIUS, CARD_WIDTH, CONTENT_PADDING, FOOTER_PADDING,
    NAV_ITEM_GAP, NAV_ITEM_HEIGHT, NAV_ITEM_ICON_SLOT, NAV_ITEM_PADDING_X, NAV_ITEM_RADIUS,
    NAV_LIST_GAP, SIDEBAR_PADDING, SIDEBAR_WIDTH,
};
use crate::new_connection::connection_window_parts::card_text;
use gpui::prelude::FluentBuilder as _;
use gpui::{
    AnyElement, AnyView, Context, FontWeight, InteractiveElement as _, IntoElement,
    ParentElement as _, Render, SharedString, StatefulInteractiveElement as _, Styled as _, div,
    px,
};
use gpui_component::{
    ActiveTheme, Disableable as _, Icon, InteractiveElementExt as _, Sizable as _, Size, TitleBar,
    button::{Button, ButtonVariants as _},
    chrome, h_flex,
    scroll::ScrollableElement as _,
    v_flex,
};
use rust_i18n::t;

impl NewConnectionWindow {
    fn render_header(&self, cx: &mut Context<Self>) -> impl IntoElement {
        TitleBar::new().child(
            div()
                .flex()
                .items_center()
                .justify_center()
                .flex_1()
                .text_sm()
                .font_weight(FontWeight::MEDIUM)
                .text_color(cx.theme().foreground)
                .child(t!("Home.new_connection").to_string()),
        )
    }

    fn render_sidebar(&self, cx: &mut Context<Self>) -> impl IntoElement {
        v_flex()
            .w(px(SIDEBAR_WIDTH))
            .h_full()
            .bg(cx.theme().sidebar)
            .border_r_1()
            .border_color(cx.theme().border)
            .p(px(SIDEBAR_PADDING))
            .gap(px(NAV_LIST_GAP))
            .children(NewConnectionCategory::all().into_iter().map(|category| {
                self.render_category_row(category, self.selected_category == category, cx)
            }))
    }

    fn render_category_row(
        &self,
        category: NewConnectionCategory,
        selected: bool,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        h_flex()
            .id(SharedString::from(format!(
                "new-connection-category-{}",
                category.label()
            )))
            .w_full()
            .h(px(NAV_ITEM_HEIGHT))
            .items_center()
            .gap(px(NAV_ITEM_GAP))
            .px(px(NAV_ITEM_PADDING_X))
            .rounded(px(NAV_ITEM_RADIUS))
            .text_color(cx.theme().muted_foreground)
            .cursor_pointer()
            .when(selected, |this| {
                this.bg(cx.theme().sidebar_accent)
                    .text_color(cx.theme().sidebar_accent_foreground)
            })
            .when(!selected, |this| {
                this.hover(|style| style.bg(cx.theme().sidebar_accent.opacity(0.55)))
            })
            .on_click(cx.listener(move |this, _, _, cx| {
                this.selected_category = category;
                this.selected_kind = Self::first_visible_item(category);
                cx.notify();
            }))
            .child(
                div()
                    .w(px(NAV_ITEM_ICON_SLOT))
                    .h_full()
                    .flex()
                    .items_center()
                    .justify_center()
                    .child(Icon::new(category.icon()).color().with_size(Size::Medium)),
            )
            .child(
                div()
                    .min_w_0()
                    .text_sm()
                    .font_weight(FontWeight::MEDIUM)
                    .child(category.label()),
            )
            .into_any_element()
    }

    fn render_card_area(&self, cx: &mut Context<Self>) -> impl IntoElement {
        v_flex()
            .flex_1()
            .h_full()
            .bg(cx.theme().muted)
            .child(
                chrome::panel_header(cx)
                    .child(t!("Home.new_connection").to_string())
                    .child(div().flex_1())
                    .child(
                        chrome::status_pill(cx).child(format!("{}", self.visible_items().len())),
                    ),
            )
            .child(
                div()
                    .flex_1()
                    .min_h_0()
                    .overflow_y_scrollbar()
                    .p(px(CONTENT_PADDING))
                    .child(self.render_card_grid(cx)),
            )
    }

    fn render_card_grid(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let mut grid = div().flex().flex_wrap().w_full().gap(px(CARD_GAP));
        for kind in self.visible_items() {
            grid = grid.child(
                div()
                    .w(px(CARD_WIDTH))
                    .flex_shrink_0()
                    .child(self.render_connection_type_card(kind, cx)),
            );
        }
        grid
    }

    fn render_connection_type_card(
        &self,
        kind: NewConnectionKind,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let selected = self.selected_kind.as_ref() == Some(&kind);
        let click_kind = kind.clone();
        let double_click_kind = kind.clone();
        let label = kind.label();
        let description = kind.description();

        v_flex()
            .id(SharedString::from(format!("new-connection-kind-{}", label)))
            .justify_center()
            .w_full()
            .h(px(CARD_HEIGHT))
            .rounded(px(CARD_RADIUS))
            .bg(cx.theme().background)
            .px(px(CARD_PADDING_X))
            .py(px(CARD_PADDING_Y))
            .border_1()
            .relative()
            .overflow_hidden()
            .shadow_sm()
            .cursor_pointer()
            .when(selected, |this| {
                this.border_color(cx.theme().list_active_border)
                    .bg(cx.theme().list_active)
                    .shadow_md()
            })
            .when(!selected, |this| this.border_color(cx.theme().border))
            .hover(|style| {
                style
                    .shadow_md()
                    .border_color(cx.theme().list_active_border)
            })
            .on_click(cx.listener(move |this, _, _, cx| {
                this.selected_kind = Some(click_kind.clone());
                cx.notify();
            }))
            .on_double_click(cx.listener(move |this, _, window, cx| {
                this.selected_kind = Some(double_click_kind.clone());
                this.open_selected(window, cx);
            }))
            .child(self.render_connection_type_card_body(kind, label, description, cx))
    }

    fn render_connection_type_card_body(
        &self,
        kind: NewConnectionKind,
        label: String,
        description: String,
        cx: &mut Context<Self>,
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
                    .child(kind.icon().with_size(px(CARD_ICON_SIZE))),
            )
            .child(card_text(label, description, cx))
    }

    fn render_selection_footer(&self, cx: &mut Context<Self>) -> impl IntoElement {
        h_flex()
            .justify_end()
            .gap_2()
            .p(px(FOOTER_PADDING))
            .border_t_1()
            .border_color(cx.theme().border)
            .child(
                Button::new("cancel-new-connection")
                    .small()
                    .label(t!("Common.cancel").to_string())
                    .on_click(cx.listener(|_, _, window, cx| {
                        window.remove_window();
                        cx.notify();
                    })),
            )
            .child(
                Button::new("next-new-connection")
                    .small()
                    .primary()
                    .label(t!("Common.next").to_string())
                    .disabled(self.selected_kind.is_none())
                    .on_click(cx.listener(|this, _, window, cx| {
                        this.open_selected(window, cx);
                    })),
            )
    }

    fn render_form_page(&self, form: AnyView, cx: &mut Context<Self>) -> impl IntoElement {
        div().size_full().relative().child(form).child(
            div()
                .absolute()
                .left(px(BACK_BUTTON_INSET))
                .bottom(px(BACK_BUTTON_INSET))
                .child(
                    Button::new("back-to-new-connection-kind")
                        .small()
                        .outline()
                        .label("上一步")
                        .on_click(cx.listener(|this, _, _, cx| {
                            this.go_back_to_selection(cx);
                        })),
                ),
        )
    }
}

impl Render for NewConnectionWindow {
    fn render(&mut self, _window: &mut gpui::Window, cx: &mut Context<Self>) -> impl IntoElement {
        if let Some(form) = self.form.clone() {
            return self.render_form_page(form, cx).into_any_element();
        }

        v_flex()
            .key_context(KEY_CONTEXT)
            .size_full()
            .track_focus(&self.focus_handle)
            .on_action(cx.listener(Self::on_action_select_previous))
            .on_action(cx.listener(Self::on_action_select_next))
            .on_action(cx.listener(Self::on_action_open_selected))
            .bg(cx.theme().background)
            .child(self.render_header(cx))
            .child(
                h_flex()
                    .flex_1()
                    .w_full()
                    .overflow_hidden()
                    .child(self.render_sidebar(cx))
                    .child(self.render_card_area(cx)),
            )
            .child(self.render_selection_footer(cx))
            .into_any_element()
    }
}
