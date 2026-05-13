//! 数据库视图侧边栏模块
//!
//! 提供数据库视图的侧边栏功能，包括：
//! - AI 聊天面板

pub(crate) mod cell_preview_panel;

use crate::chatdb::chat_panel::{ChatPanel, ChatPanelEvent};
use crate::chatdb::db_connection_selector::DbSelectorContext;
use crate::database_objects_tab::DatabaseObjectsPanel;
use gpui::prelude::FluentBuilder;
use gpui::{
    AnyElement, App, AppContext, ClipboardItem, Context, Entity, EventEmitter, FocusHandle,
    Focusable, IntoElement, ParentElement, Render, SharedString, StatefulInteractiveElement,
    Styled, Subscription, Window, div, px,
};
use gpui_component::button::{Button, ButtonVariants as _};
use gpui_component::scroll::ScrollableElement;
use gpui_component::{ActiveTheme, Icon, IconName, Sizable, Size, StyledExt, h_flex, v_flex};
use one_core::ai_chat::CodeBlockAction;
use one_core::ai_chat::ask_ai::{AskAiEvent, get_ask_ai_notifier};
use one_ui::workbench;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SidebarPanel {
    AiChat,
    DdlPreview,
}

impl SidebarPanel {
    pub fn icon(&self) -> Icon {
        match self {
            SidebarPanel::AiChat => IconName::ChatDB.color(),
            SidebarPanel::DdlPreview => IconName::Eye.color(),
        }
    }
}

#[derive(Clone, Debug)]
pub enum DatabaseSidebarEvent {
    PanelChanged,
    AskAi,
}

pub struct DatabaseSidebar {
    active_panel: Option<SidebarPanel>,
    chat_panel: Entity<ChatPanel>,
    objects_panel: Entity<DatabaseObjectsPanel>,
    focus_handle: FocusHandle,
    is_active: bool,
    _subs: Vec<Subscription>,
}

impl DatabaseSidebar {
    pub fn new(
        window: &mut Window,
        cx: &mut Context<Self>,
        selector_context: DbSelectorContext,
        objects_panel: Entity<DatabaseObjectsPanel>,
    ) -> Self {
        let chat_panel =
            cx.new(|cx| ChatPanel::new_for_sidebar(window, cx, selector_context.clone()));

        let mut subs = Vec::new();
        subs.push(
            cx.subscribe(&chat_panel, |this, _, _event: &ChatPanelEvent, cx| {
                this.active_panel = None;
                cx.emit(DatabaseSidebarEvent::PanelChanged);
                cx.notify();
            }),
        );

        if let Some(notifier) = get_ask_ai_notifier(cx) {
            subs.push(
                cx.subscribe(&notifier, move |this, _, event: &AskAiEvent, cx| {
                    if this.is_active {
                        let AskAiEvent::Request(message) = event;
                        this.ask_ai(message.clone(), cx);
                    }
                }),
            );
        }

        Self {
            active_panel: None,
            chat_panel,
            objects_panel,
            focus_handle: cx.focus_handle(),
            is_active: false,
            _subs: subs,
        }
    }

    pub fn set_active(&mut self, active: bool, cx: &mut Context<Self>) {
        self.is_active = active;
        cx.notify();
    }

    pub fn set_active_panel(&mut self, panel: Option<SidebarPanel>, cx: &mut Context<Self>) {
        if self.active_panel != panel {
            self.active_panel = panel;
            cx.emit(DatabaseSidebarEvent::PanelChanged);
            cx.notify();
        }
    }

    pub fn toggle_panel(&mut self, panel: SidebarPanel, cx: &mut Context<Self>) {
        if self.active_panel == Some(panel) {
            self.set_active_panel(None, cx);
        } else {
            self.set_active_panel(Some(panel), cx);
        }
    }

    pub fn is_panel_visible(&self) -> bool {
        self.active_panel.is_some()
    }

    pub fn ask_ai(&mut self, message: String, cx: &mut Context<Self>) {
        if self.active_panel != Some(SidebarPanel::AiChat) {
            self.active_panel = Some(SidebarPanel::AiChat);
        }

        self.chat_panel.update(cx, |panel, cx| {
            panel.send_external_message(message, cx);
        });

        cx.emit(DatabaseSidebarEvent::AskAi);
        cx.notify();
    }

    pub fn register_code_block_action(&self, _action: CodeBlockAction, _cx: &mut Context<Self>) {}

    fn render_toolbar_button(
        &self,
        panel: SidebarPanel,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let is_active = self.active_panel == Some(panel);
        let accent_fg = cx.theme().accent_foreground;
        let muted_fg = cx.theme().muted_foreground;

        workbench::side_toolbar_button(
            SharedString::from(format!("database-sidebar-btn-{:?}", panel)),
            is_active,
            cx,
        )
        .on_click(cx.listener(move |this, _event, _window, cx| {
            this.toggle_panel(panel, cx);
        }))
        .child(
            Icon::new(panel.icon())
                .with_size(Size::Medium)
                .text_color(if is_active { accent_fg } else { muted_fg }),
        )
    }

    pub fn render_toolbar(&self, window: &mut Window, cx: &mut Context<Self>) -> AnyElement {
        workbench::side_toolbar(cx)
            .child(self.render_toolbar_button(SidebarPanel::AiChat, window, cx))
            .child(self.render_toolbar_button(SidebarPanel::DdlPreview, window, cx))
            .into_any_element()
    }

    pub fn render_panel_content(
        &self,
        panel: SidebarPanel,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        match panel {
            SidebarPanel::AiChat => self.chat_panel.clone().into_any_element(),
            SidebarPanel::DdlPreview => self.render_ddl_preview_panel(window, cx),
        }
    }

    fn render_ddl_preview_panel(&self, _window: &mut Window, cx: &mut Context<Self>) -> AnyElement {
        let objects_panel = self.objects_panel.read(cx);
        let database_objects = objects_panel.database_objects().read(cx);
        let ddl_text = database_objects.ddl_preview_content().to_string();
        let title = database_objects
            .ddl_preview_table_name()
            .map(|name| format!("DDL: {name}"))
            .unwrap_or_else(|| "DDL Preview".to_string());

        fn approximate_display_columns(text: &str) -> usize {
            text.chars()
                .map(|ch| if ch.is_ascii() { 1 } else { 2 })
                .sum()
        }

        let longest_line_columns = ddl_text
            .lines()
            .map(approximate_display_columns)
            .max()
            .unwrap_or(0);
        let ddl_content_width = px((longest_line_columns as f32 * 9.8) + 120.0).max(px(420.0));

        let content = if ddl_text.trim().is_empty() {
            div()
                .text_sm()
                .text_color(cx.theme().muted_foreground)
                .child("Select a table to preview DDL")
                .into_any_element()
        } else {
            div()
                .min_w_full()
                .child(
                    v_flex()
                        .gap_1()
                        .children(ddl_text.lines().map(|line: &str| {
                            div()
                                .font_family(cx.theme().mono_font_family.clone())
                                .text_sm()
                                .text_color(cx.theme().foreground)
                                .whitespace_nowrap()
                                .child(if line.is_empty() {
                                    " ".to_string()
                                } else {
                                    line.to_string()
                                })
                        })),
                )
                .into_any_element()
        };

        v_flex()
            .size_full()
            .bg(cx.theme().background)
            .child(
                workbench::workbench_toolbar(cx)
                    .justify_between()
                    .child(
                        div()
                            .text_sm()
                            .font_semibold()
                            .text_color(cx.theme().foreground)
                            .child(title),
                    )
                    .child(
                        h_flex()
                            .gap_2()
                            .items_center()
                            .when(database_objects.ddl_preview_loading(), |this| {
                                this.child(
                                    div()
                                        .text_xs()
                                        .text_color(cx.theme().muted_foreground)
                                        .child("Loading"),
                                )
                            })
                            .when(!ddl_text.trim().is_empty(), |this| {
                                this.child(
                                    Button::new("sidebar-copy-ddl")
                                        .ghost()
                                        .xsmall()
                                        .icon(IconName::Copy)
                                        .label("复制")
                                        .on_click(move |_, _window: &mut Window, cx: &mut App| {
                                            cx.stop_propagation();
                                            cx.write_to_clipboard(ClipboardItem::new_string(
                                                ddl_text.to_string(),
                                            ));
                                        }),
                                )
                            })
                            .child(
                                Button::new("sidebar-close-ddl-preview")
                                    .icon(IconName::Close)
                                    .ghost()
                                    .small()
                                    .tooltip("关闭面板")
                                    .on_click(cx.listener(|this, _event, _window, cx| {
                                        this.set_active_panel(None, cx);
                                    })),
                            ),
                    ),
            )
            .child(
                div().flex_1().overflow_x_scrollbar_masked().child(
                    v_flex()
                        .w(ddl_content_width)
                        .min_w(ddl_content_width)
                        .h_full()
                        .flex_shrink_0()
                        .child(
                            div().flex_1().overflow_y_scrollbar_masked().child(
                                workbench::code_canvas(cx)
                                    .min_h_full()
                                    .p_3()
                                    .border_t_1()
                                    .border_color(cx.theme().border)
                                    .child(content),
                            ),
                        ),
                ),
            )
            .into_any_element()
    }
}

impl EventEmitter<DatabaseSidebarEvent> for DatabaseSidebar {}

impl Focusable for DatabaseSidebar {
    fn focus_handle(&self, _cx: &App) -> FocusHandle {
        self.focus_handle.clone()
    }
}

impl Render for DatabaseSidebar {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .h_full()
            .flex_shrink_0()
            .when_some(self.active_panel, |this, panel| {
                this.w_full().child(
                    workbench::side_panel_surface(cx)
                        .child(self.render_panel_content(panel, window, cx)),
                )
            })
            .when(!self.is_panel_visible(), |this| {
                this.child(self.render_toolbar(window, cx))
            })
    }
}
