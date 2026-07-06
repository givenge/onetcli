use gpui::{
    App, AppContext, Context, Entity, EventEmitter, FocusHandle, Focusable, IntoElement,
    ParentElement, Render, Styled, Window, div, px,
};
use gpui_component::{
    ActiveTheme, Icon, IconName, Sizable, Size,
    button::Button,
    button::ButtonVariants,
    h_flex,
    input::{Input, InputEvent, InputState},
    slider::{Slider, SliderEvent, SliderState},
    v_flex,
};

#[derive(Clone, Debug)]
pub struct ModelSettings {
    pub temperature: f32,
    pub history_count: usize,
    pub max_tokens: usize,
    /// 上下文窗口大小（近似 token）
    pub context_window_size: usize,
    /// 推理强度
    pub reasoning_effort: ReasoningEffort,
}

impl Default for ModelSettings {
    fn default() -> Self {
        Self {
            temperature: 0.7,
            history_count: 10,
            max_tokens: 2000,
            context_window_size: 32000,
            reasoning_effort: ReasoningEffort::Medium,
        }
    }
}

impl ModelSettings {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_temperature(mut self, temperature: f32) -> Self {
        self.temperature = temperature.clamp(0.0, 2.0);
        self
    }

    pub fn with_history_count(mut self, count: usize) -> Self {
        self.history_count = count.min(50);
        self
    }

    pub fn with_max_tokens(mut self, max_tokens: usize) -> Self {
        self.max_tokens = max_tokens.max(100);
        self
    }

    pub fn with_context_window_size(mut self, context_window_size: usize) -> Self {
        self.context_window_size = context_window_size.max(1024);
        self
    }

    pub fn with_reasoning_effort(mut self, reasoning_effort: ReasoningEffort) -> Self {
        self.reasoning_effort = reasoning_effort;
        self
    }

    pub fn from_provider_config(config: &ProviderConfig) -> Self {
        Self {
            temperature: config.temperature.unwrap_or(0.7),
            history_count: config.history_count.unwrap_or(10).clamp(0, 50) as usize,
            max_tokens: config.max_tokens.unwrap_or(2000).max(100) as usize,
            context_window_size: config.context_window_size.unwrap_or(32_000).max(1024) as usize,
            reasoning_effort: config.reasoning_effort.unwrap_or(ReasoningEffort::Medium),
        }
    }

    pub fn apply_to_provider_config(&self, config: &mut ProviderConfig) {
        config.temperature = Some(self.temperature);
        config.max_tokens = Some(self.max_tokens as i32);
        config.reasoning_effort = Some(self.reasoning_effort);
        config.history_count = Some(self.history_count as i32);
        config.context_window_size = Some(self.context_window_size as i32);
    }
}

#[derive(Clone, Debug)]
pub enum ModelSettingsEvent {
    Changed(ModelSettings),
}

pub struct ModelSettingsPanel {
    focus_handle: FocusHandle,
    settings: ModelSettings,
    temperature_slider: Entity<SliderState>,
    history_input: Entity<InputState>,
    max_tokens_input: Entity<InputState>,
    labels: ModelSettingsLabels,
}

#[derive(Clone, Debug)]
pub struct ModelSettingsLabels {
    pub title: String,
    pub temperature_label: String,
    pub temperature_desc: String,
    pub history_label: String,
    pub history_desc: String,
    pub max_tokens_label: String,
    pub max_tokens_desc: String,
    pub context_window_label: String,
    pub context_window_desc: String,
    pub reasoning_effort_label: String,
    pub reasoning_effort_desc: String,
    pub footer_notice: String,
}

impl Default for ModelSettingsLabels {
    fn default() -> Self {
        Self {
            title: "模型设置".to_string(),
            temperature_label: "温度".to_string(),
            temperature_desc: "控制输出随机性".to_string(),
            history_label: "历史记录".to_string(),
            history_desc: "携带的历史消息数".to_string(),
            max_tokens_label: "最大 Token".to_string(),
            max_tokens_desc: "单次回复 token 上限".to_string(),
            footer_notice: "设置会应用到后续请求".to_string(),
        }
    }
}

impl ModelSettingsPanel {
    pub fn new(settings: ModelSettings, window: &mut Window, cx: &mut Context<Self>) -> Self {
        Self::with_labels(settings, ModelSettingsLabels::default(), window, cx)
    }

    pub fn with_labels(
        settings: ModelSettings,
        labels: ModelSettingsLabels,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Self {
        let focus_handle = cx.focus_handle();
        let temperature_slider = cx.new(|_cx| {
            SliderState::new()
                .min(0.0)
                .max(2.0)
                .step(0.1)
                .default_value(settings.temperature)
        });
        cx.subscribe_in(
            &temperature_slider,
            window,
            |this, _, event, _window, cx| {
                if let SliderEvent::Change(gpui_component::slider::SliderValue::Single(v)) = event {
                    this.settings.temperature = *v;
                    this.emit_change(cx);
                }
            },
        )
        .detach();

        let history_input = cx.new(|cx| {
            InputState::new(window, cx).default_value(settings.history_count.to_string())
        });
        cx.subscribe_in(&history_input, window, |this, input, event, _window, cx| {
            if let InputEvent::Change = event {
                let text = input.read(cx).text().to_string();
                if let Ok(count) = text.parse::<usize>() {
                    this.settings.history_count = count.min(50);
                    this.emit_change(cx);
                }
            }
        })
        .detach();

        let max_tokens_input =
            cx.new(|cx| InputState::new(window, cx).default_value(settings.max_tokens.to_string()));
        cx.subscribe_in(
            &max_tokens_input,
            window,
            |this, input, event, _window, cx| {
                if let InputEvent::Change = event {
                    let text = input.read(cx).text().to_string();
                    if let Ok(tokens) = text.parse::<usize>() {
                        this.settings.max_tokens = tokens.max(100);
                        this.emit_change(cx);
                    }
                }
            },
        )
        .detach();

        cx.subscribe_in(
            &context_window_input,
            window,
            |this, input, event, _window, cx| {
                if let InputEvent::Change = event {
                    let text = input.read(cx).text().to_string();
                    if let Ok(tokens) = text.parse::<usize>() {
                        this.settings.context_window_size = tokens.max(1024);
                        this.emit_change(cx);
                    }
                }
            },
        )
        .detach();

        Self {
            focus_handle,
            settings,
            temperature_slider,
            history_input,
            max_tokens_input,
            context_window_input,
            labels,
        }
    }

    pub fn settings(&self) -> &ModelSettings {
        &self.settings
    }

    pub fn update_settings(
        &mut self,
        settings: ModelSettings,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.settings = settings.clone();
        self.temperature_slider.update(cx, |state, cx| {
            state.set_value(settings.temperature, window, cx);
        });
        self.history_input.update(cx, |input, cx| {
            input.set_value(settings.history_count.to_string(), window, cx);
        });
        self.max_tokens_input.update(cx, |input, cx| {
            input.set_value(settings.max_tokens.to_string(), window, cx);
        });
        cx.notify();
    }

    fn emit_change(&self, cx: &mut Context<Self>) {
        cx.emit(ModelSettingsEvent::Changed(self.settings.clone()));
    }

    fn set_reasoning_effort(&mut self, effort: ReasoningEffort, cx: &mut Context<Self>) {
        if self.settings.reasoning_effort == effort {
            return;
        }
        self.settings.reasoning_effort = effort;
        self.emit_change(cx);
        cx.notify();
    }

    fn render_setting_row(
        &self,
        label: &str,
        description: &str,
        content: impl IntoElement,
        cx: &App,
    ) -> impl IntoElement {
        h_flex()
            .w_full()
            .items_center()
            .justify_between()
            .gap_4()
            .py_2()
            .child(
                v_flex()
                    .flex_1()
                    .min_w_0()
                    .overflow_x_hidden()
                    .gap_0p5()
                    .child(
                        div()
                            .text_sm()
                            .font_weight(gpui::FontWeight::MEDIUM)
                            .truncate()
                            .child(label.to_string()),
                    )
                    .child(
                        div()
                            .text_xs()
                            .text_color(cx.theme().muted_foreground)
                            .truncate()
                            .child(description.to_string()),
                    ),
            )
            .child(div().flex_shrink_0().child(content))
    }
}

impl EventEmitter<ModelSettingsEvent> for ModelSettingsPanel {}

impl Focusable for ModelSettingsPanel {
    fn focus_handle(&self, _cx: &App) -> FocusHandle {
        self.focus_handle.clone()
    }
}

impl Render for ModelSettingsPanel {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let border = cx.theme().border;

        v_flex()
            .w(px(320.0))
            .gap_1()
            .child(
                h_flex()
                    .w_full()
                    .items_center()
                    .gap_2()
                    .pb_2()
                    .border_b_1()
                    .border_color(border)
                    .child(
                        Icon::new(IconName::Settings)
                            .with_size(Size::Small)
                            .text_color(cx.theme().muted_foreground),
                    )
                    .child(
                        div()
                            .text_sm()
                            .font_weight(gpui::FontWeight::SEMIBOLD)
                            .child(self.labels.title.clone()),
                    ),
            )
            .child(
                self.render_setting_row(
                    &self.labels.temperature_label,
                    &self.labels.temperature_desc,
                    h_flex()
                        .items_center()
                        .gap_2()
                        .child(
                            div()
                                .w(px(100.0))
                                .child(Slider::new(&self.temperature_slider)),
                        )
                        .child(
                            div()
                                .text_sm()
                                .text_color(cx.theme().muted_foreground)
                                .min_w(px(32.0))
                                .child(format!("{:.1}", self.settings.temperature)),
                        ),
                    cx,
                ),
            )
            .child(
                self.render_setting_row(
                    &self.labels.history_label,
                    &self.labels.history_desc,
                    div()
                        .w(px(80.0))
                        .child(Input::new(&self.history_input).with_size(Size::Small)),
                    cx,
                ),
            )
            .child(
                self.render_setting_row(
                    &self.labels.max_tokens_label,
                    &self.labels.max_tokens_desc,
                    div()
                        .w(px(80.0))
                        .child(Input::new(&self.max_tokens_input).with_size(Size::Small)),
                    cx,
                ),
            )
            .child(
                div()
                    .w_full()
                    .pt_2()
                    .mt_1()
                    .border_t_1()
                    .border_color(border)
                    .child(
                        div()
                            .text_xs()
                            .text_color(cx.theme().muted_foreground)
                            .child(self.labels.footer_notice.clone()),
                    ),
            )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_match_chat_runtime_expectations() {
        let settings = ModelSettings::default();

        assert_eq!(settings.temperature, 0.7);
        assert_eq!(settings.history_count, 10);
        assert_eq!(settings.max_tokens, 2000);
    }

    #[test]
    fn builder_methods_clamp_to_supported_ranges() {
        let settings = ModelSettings::new()
            .with_temperature(9.0)
            .with_history_count(500)
            .with_max_tokens(1);

        assert_eq!(settings.temperature, 2.0);
        assert_eq!(settings.history_count, 50);
        assert_eq!(settings.max_tokens, 100);
    }
}
