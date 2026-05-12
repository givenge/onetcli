//! ChatMessageRenderer - 共享消息渲染工具
//!
//! 提供通用的消息渲染函数，可被不同的面板复用。
//! SQL 面板可以在此基础上覆盖特定渲染（如 SQL 代码块）。

use crate::ai_chat::panel::CodeBlockActionRegistry;
use crate::ai_chat::types::{ChatMessageUIGeneric, ChatRole, MessageExtension, MessageVariant};
use gpui::prelude::FluentBuilder;
use gpui::{
    AnyElement, App, InteractiveElement, IntoElement, ParentElement, SharedString, Styled, div,
};
use gpui_component::avatar::Avatar;
use gpui_component::button::Button;
use gpui_component::clipboard::Clipboard;
use gpui_component::{
    ActiveTheme, Icon, IconName, Sizable, Size, button::ButtonVariants, h_flex, text::TextView,
    v_flex,
};
use rust_i18n::t;

/// 共享消息渲染器
pub struct ChatMessageRenderer;

impl ChatMessageRenderer {
    pub fn render_assistant_shell(content: AnyElement, cx: &App) -> AnyElement {
        h_flex()
            .w_full()
            .items_start()
            .gap_2()
            .child(
                Avatar::new()
                    .placeholder(Icon::new(IconName::AI))
                    .with_size(Size::Small)
                    .bg(cx.theme().primary.opacity(0.1))
                    .text_color(cx.theme().primary),
            )
            .child(
                div()
                    .flex_1()
                    .min_w_0()
                    .bg(cx.theme().muted.opacity(0.3))
                    .border_1()
                    .border_color(cx.theme().border.opacity(0.5))
                    .rounded_lg()
                    .child(content),
            )
            .into_any_element()
    }

    /// 渲染用户消息
    pub fn render_user_message<E: MessageExtension>(
        msg: &ChatMessageUIGeneric<E>,
        cx: &App,
    ) -> AnyElement {
        h_flex()
            .w_full()
            .items_start()
            .justify_end()
            .gap_2()
            .child(
                div()
                    .min_w_0()
                    .max_w(gpui::px(480.0))
                    .px_3()
                    .py_2()
                    .bg(cx.theme().accent)
                    .text_color(cx.theme().accent_foreground)
                    .rounded_lg()
                    .child(
                        TextView::markdown(
                            SharedString::from(format!("user-msg-{}", msg.id)),
                            msg.content.clone(),
                        )
                        .selectable(true),
                    ),
            )
            .child(
                Avatar::new()
                    .placeholder(Icon::new(IconName::CircleUser))
                    .with_size(Size::Small)
                    .bg(cx.theme().accent.opacity(0.1))
                    .text_color(cx.theme().accent),
            )
            .into_any_element()
    }

    /// 渲染系统消息
    pub fn render_system_message<E: MessageExtension>(
        msg: &ChatMessageUIGeneric<E>,
        cx: &App,
    ) -> AnyElement {
        h_flex()
            .w_full()
            .justify_center()
            .py_2()
            .child(
                div()
                    .px_3()
                    .py_1()
                    .rounded_full()
                    .bg(cx.theme().muted.opacity(0.4))
                    .text_xs()
                    .text_color(cx.theme().muted_foreground)
                    .child(msg.content.clone()),
            )
            .into_any_element()
    }

    /// 渲染状态消息
    pub fn render_status_message(id: &str, title: &str, is_done: bool, cx: &App) -> AnyElement {
        let icon = if is_done {
            IconName::Check
        } else {
            IconName::Loader
        };

        h_flex()
            .id(SharedString::from(id.to_string()))
            .w_full()
            .items_center()
            .gap_2()
            .py_1()
            .px_10() // 为助手头像留出空间
            .child(
                Icon::new(icon)
                    .with_size(Size::Small)
                    .text_color(if is_done {
                        cx.theme().success
                    } else {
                        cx.theme().muted_foreground
                    }),
            )
            .child(
                div()
                    .text_xs()
                    .text_color(cx.theme().muted_foreground)
                    .child(title.to_string()),
            )
            .into_any_element()
    }

    /// 渲染 "思考中..." 占位符
    pub fn render_thinking(cx: &App) -> AnyElement {
        Self::render_assistant_shell(
            h_flex()
                .items_center()
                .gap_2()
                .px_3()
                .py_2()
                .child(Icon::new(IconName::LoaderCircle).with_size(Size::Small))
                .child(
                    div()
                        .text_sm()
                        .text_color(cx.theme().muted_foreground)
                        .child(t!("AiChat.thinking").to_string()),
                )
                .into_any_element(),
            cx,
        )
    }

    /// 渲染独立的思考消息
    pub fn render_thinking_message<E: MessageExtension>(
        msg: &ChatMessageUIGeneric<E>,
        cx: &App,
    ) -> AnyElement {
        let view_id = SharedString::from(format!("ai-thinking-{}", msg.id));
        v_flex()
            .w_full()
            .px_10() // 助手头像偏移
            .child(
                div()
                    .w_full()
                    .rounded_sm()
                    .px_1()
                    .py_1()
                    .child(
                        h_flex()
                            .w_full()
                            .items_center()
                            .justify_between()
                            .child(
                                h_flex()
                                    .items_center()
                                    .gap_1()
                                    .child(
                                        Icon::new(IconName::Bot)
                                            .xsmall()
                                            .text_color(cx.theme().muted_foreground),
                                    )
                                    .child(
                                        div()
                                            .text_xs()
                                            .text_color(cx.theme().muted_foreground)
                                            .child("Thinking".to_string()),
                                    ),
                            )
                            .child(
                                h_flex()
                                    .items_center()
                                    .gap_1()
                                    .child(
                                        div()
                                            .text_xs()
                                            .text_color(cx.theme().muted_foreground)
                                            .child(if msg.is_streaming {
                                                "进行中".to_string()
                                            } else {
                                                "详情".to_string()
                                            }),
                                    )
                                    .child(
                                        Icon::new(if msg.is_expanded {
                                            IconName::ChevronUp
                                        } else {
                                            IconName::ChevronDown
                                        })
                                        .xsmall()
                                        .text_color(cx.theme().muted_foreground),
                                    ),
                            ),
                    )
                    .when(msg.is_expanded, |this| {
                        this.child(
                            div()
                                .mt_1()
                                .border_l_2()
                                .border_color(cx.theme().muted_foreground.opacity(0.2))
                                .pl_2()
                                .py_0p5()
                                .child(
                                    TextView::markdown(view_id, msg.content.clone())
                                        .text_color(cx.theme().muted_foreground)
                                        .p_0()
                                        .selectable(true),
                                ),
                        )
                    }),
            )
            .into_any_element()
    }

    pub fn render_tool_history_message<E: MessageExtension>(
        msg: &ChatMessageUIGeneric<E>,
        title: &str,
        cx: &App,
    ) -> AnyElement {
        let view_id = SharedString::from(format!("ai-tool-history-{}", msg.id));
        v_flex()
            .w_full()
            .px_10()
            .child(
                div()
                    .w_full()
                    .rounded_sm()
                    .px_1()
                    .py_1()
                    .child(
                        h_flex()
                            .w_full()
                            .items_center()
                            .justify_between()
                            .child(
                                h_flex()
                                    .items_center()
                                    .gap_1()
                                    .child(
                                        Icon::new(IconName::Search)
                                            .xsmall()
                                            .text_color(cx.theme().muted_foreground),
                                    )
                                    .child(
                                        div()
                                            .text_xs()
                                            .text_color(cx.theme().muted_foreground)
                                            .child(title.to_string()),
                                    ),
                            )
                            .child(
                                h_flex()
                                    .items_center()
                                    .gap_1()
                                    .child(
                                        div()
                                            .text_xs()
                                            .text_color(cx.theme().muted_foreground)
                                            .child(if msg.is_streaming {
                                                "进行中".to_string()
                                            } else {
                                                "详情".to_string()
                                            }),
                                    )
                                    .child(
                                        Icon::new(if msg.is_expanded {
                                            IconName::ChevronUp
                                        } else {
                                            IconName::ChevronDown
                                        })
                                        .xsmall()
                                        .text_color(cx.theme().muted_foreground),
                                    ),
                            ),
                    )
                    .when(msg.is_expanded, |this| {
                        this.child(
                            div()
                                .mt_1()
                                .border_l_2()
                                .border_color(cx.theme().muted_foreground.opacity(0.2))
                                .pl_2()
                                .py_0p5()
                                .child(
                                    TextView::markdown(view_id, msg.content.clone())
                                        .text_color(cx.theme().muted_foreground)
                                        .p_0()
                                        .selectable(true),
                                ),
                        )
                    }),
            )
            .into_any_element()
    }

    /// 渲染助手文本消息（带代码块操作按钮）
    pub fn render_assistant_text<E: MessageExtension>(
        msg: &ChatMessageUIGeneric<E>,
        code_block_actions: &CodeBlockActionRegistry,
        cx: &App,
    ) -> AnyElement {
        if msg.is_streaming && msg.content.is_empty() {
            return Self::render_thinking(cx);
        }

        let view_id = SharedString::from(format!("ai-msg-{}", msg.id));

        Self::render_assistant_shell(
            {
                let text_view = TextView::markdown(view_id, msg.content.clone())
                    .p_3()
                    .selectable(true);

                if code_block_actions.is_empty() {
                    text_view.into_any_element()
                } else {
                    let registry = code_block_actions.clone();
                    text_view
                        .code_block_actions(move |code_block, _window, _cx| {
                            let code = code_block.code();
                            let lang = code_block.lang();
                            let lang_str = lang.as_ref().map(|s| s.as_ref());
                            let matched_actions = registry.get_actions_for_lang(lang_str);

                            let mut row = h_flex()
                                .gap_1()
                                .child(Clipboard::new("copy").value(code.clone()));

                            for (idx, action) in matched_actions.iter().enumerate() {
                                let btn_id = SharedString::from(format!("{}-{}", action.id, idx));
                                let callback = action.callback.clone();
                                let icon = action.icon.clone();
                                let label = action.label.clone();
                                let code = code.to_string();
                                let lang = lang.as_ref().map(|s| s.to_string());
                                let mut btn =
                                    Button::new(btn_id).icon(icon).ghost().xsmall().on_click({
                                        let code = code.clone();
                                        let lang = lang.clone();
                                        move |_, window, cx| {
                                            callback(code.clone(), lang.clone(), window, cx);
                                        }
                                    });

                                if let Some(lbl) = label {
                                    btn = btn.tooltip(lbl);
                                }

                                row = row.child(btn);
                            }

                            row
                        })
                        .into_any_element()
                }
            },
            cx,
        )
    }

    /// 渲染单条消息（通用路由）
    pub fn render_message<E: MessageExtension>(
        msg: &ChatMessageUIGeneric<E>,
        code_block_actions: &CodeBlockActionRegistry,
        cx: &App,
    ) -> AnyElement {
        match msg.role {
            ChatRole::User => Self::render_user_message(msg, cx),
            ChatRole::Assistant => match &msg.variant {
                MessageVariant::Status { title, is_done } => {
                    Self::render_status_message(&msg.id, title, *is_done, cx)
                }
                MessageVariant::Thinking => Self::render_thinking_message(msg, cx),
                MessageVariant::ToolHistory { title } => {
                    Self::render_tool_history_message(msg, title, cx)
                }
                MessageVariant::Text => Self::render_assistant_text(msg, code_block_actions, cx),
                MessageVariant::SqlResult => Self::render_assistant_shell(
                    div()
                        .px_3()
                        .py_2()
                        .text_xs()
                        .text_color(cx.theme().muted_foreground)
                        .child(t!("AiChat.sql_result").to_string())
                        .into_any_element(),
                    cx,
                ),
            },
            ChatRole::System => Self::render_system_message(msg, cx),
        }
    }
}
