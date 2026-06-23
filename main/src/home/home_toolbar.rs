use crate::home::home_layout::SEARCH_WIDTH;
use crate::home_tab::HomePage;
use crate::setting_tab::{AppSettings, show_webdav_backup_settings_window};
use crate::webdav_backup_manager::show_webdav_backup_manager_window;
use gpui::{
    Context, Entity, IntoElement, ParentElement as _, Styled as _, Window, div,
    prelude::FluentBuilder as _, px,
};
use gpui_component::{
    ActiveTheme, Disableable as _, IconName, WindowExt,
    button::{Button, ButtonVariant, ButtonVariants as _},
    chrome, h_flex,
    input::Input,
};
use rust_i18n::t;

impl HomePage {
    pub(crate) fn render_toolbar(
        &mut self,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        chrome::app_toolbar(cx)
            .child(self.render_primary_toolbar_actions(window, cx))
            .child(div().flex_1())
            .child(self.render_secondary_toolbar_actions(window, cx))
    }

    fn render_primary_toolbar_actions(
        &mut self,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let view = cx.entity();

        h_flex()
            .gap_2()
            .items_center()
            .child(new_connection_button(view, window))
            .child(self.backup_button(window, cx))
            .child(self.restore_button(cx))
            .when_some(self.backup_error.clone(), |this, error| {
                this.child(
                    div()
                        .max_w(px(360.0))
                        .text_xs()
                        .text_color(cx.theme().danger)
                        .truncate()
                        .child(error),
                )
            })
    }

    fn render_secondary_toolbar_actions(
        &mut self,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let workspace_filter_open = self.workspace_filter_open;
        let workspace_filter =
            self.render_workspace_filter_popover(workspace_filter_open, window, cx);

        h_flex()
            .gap_1()
            .items_center()
            .child(
                Input::new(&self.search_input)
                    .cleanable(true)
                    .w(px(SEARCH_WIDTH)),
            )
            .child(
                Button::new("refresh-button")
                    .icon(IconName::Refresh)
                    .ghost()
                    .tooltip(t!("Home.refresh"))
                    .on_click(cx.listener(|this, _, _, cx| {
                        this.refresh_local_home_data(cx);
                    })),
            )
            .child(workspace_filter)
    }
}

fn new_connection_button(view: Entity<HomePage>, window: &mut Window) -> impl IntoElement {
    Button::new("new-connect-button")
        .icon(IconName::Plus)
        .with_variant(ButtonVariant::Secondary)
        .outline()
        .label(t!("Home.new_connection"))
        .tooltip(t!("Home.new_connection"))
        .on_click(window.listener_for(&view, move |this, _, window, cx| {
            this.show_new_connection_dialog(window, cx);
        }))
}

impl HomePage {
    fn backup_button(&self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let backing_up = self.backing_up;
        let backup_enabled = AppSettings::global(cx).webdav_backup.enabled;
        let view = cx.entity().clone();

        Button::new("webdav-backup-button")
            .icon(IconName::Upload)
            .label(if backing_up {
                t!("Home.backing_up").to_string()
            } else {
                t!("Home.backup").to_string()
            })
            .with_variant(ButtonVariant::Secondary)
            .outline()
            .disabled(backing_up)
            .tooltip(if backup_enabled {
                t!("Home.backup_tooltip")
            } else {
                t!("Settings.Backup.WebDav.open")
            })
            .on_click(window.listener_for(&view, move |_this, _, window, cx| {
                if backup_enabled {
                    let view = cx.entity().clone();
                    window.open_dialog(cx, move |dialog, _window, _cx| {
                        let view_for_ok = view.clone();
                        dialog
                            .title(t!("Home.backup_confirm_title").to_string())
                            .child(t!("Home.backup_confirm_message").to_string())
                            .confirm()
                            .on_ok(move |_, _, cx| {
                                let _ = view_for_ok.update(cx, |this, cx| {
                                    this.backup_to_webdav(cx);
                                });
                                true
                            })
                    });
                } else {
                    show_webdav_backup_settings_window(cx);
                }
            }))
    }

    fn restore_button(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let backup_enabled = AppSettings::global(cx).webdav_backup.enabled;

        Button::new("webdav-restore-button")
            .icon(IconName::HardDrive)
            .label(t!("Home.restore").to_string())
            .with_variant(ButtonVariant::Secondary)
            .outline()
            .tooltip(if backup_enabled {
                t!("Home.restore_tooltip")
            } else {
                t!("Settings.Backup.WebDav.open")
            })
            .on_click(cx.listener(move |_, _, _, cx| {
                if backup_enabled {
                    show_webdav_backup_manager_window(cx);
                } else {
                    show_webdav_backup_settings_window(cx);
                }
            }))
    }
}
