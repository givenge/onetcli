use crate::home_tab::HomePage;
use crate::license::{is_feature_enabled, show_upgrade_dialog};
use gpui::{
    Context, Entity, IntoElement, ParentElement as _, Styled as _, Window, div,
    prelude::FluentBuilder as _, px,
};
use gpui_component::{
    ActiveTheme, Disableable as _, IconName,
    button::{Button, ButtonVariants as _},
    chrome, h_flex,
    input::Input,
};
use one_core::{crypto, license::Feature};
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
        let has_master_key = crypto::has_master_key();
        let has_conflicts = !self.pending_conflicts.is_empty();
        let conflict_count = self.pending_conflicts.len();

        h_flex()
            .gap_2()
            .items_center()
            .child(new_connection_button(view, window))
            .child(self.sync_button(cx))
            .when(has_conflicts, |this| {
                this.child(self.conflict_button(conflict_count, cx))
            })
            .child(encryption_key_button(has_master_key, cx))
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
            .child(Input::new(&self.search_input).cleanable(true).w(px(260.0)))
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
        .primary()
        .label(t!("Home.new_connection"))
        .tooltip(t!("Home.new_connection"))
        .on_click(window.listener_for(&view, move |this, _, window, cx| {
            this.show_new_connection_dialog(window, cx);
        }))
}

impl HomePage {
    fn sync_button(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let is_syncing = self.syncing;
        let is_logged_in = self.current_user.is_some();
        let has_sync_license = is_feature_enabled(Feature::CloudSync, cx);

        Button::new("sync-button")
            .icon(if has_sync_license {
                IconName::Refresh
            } else {
                IconName::Key
            })
            .label(sync_button_label(is_syncing, has_sync_license))
            .ghost()
            .disabled((!is_logged_in && has_sync_license) || is_syncing)
            .tooltip(sync_button_tooltip(is_logged_in, has_sync_license))
            .on_click(cx.listener(move |this, _, window, cx| {
                if !has_sync_license {
                    show_upgrade_dialog(window, cx);
                } else {
                    this.trigger_sync(cx);
                }
            }))
    }

    fn conflict_button(&self, conflict_count: usize, cx: &mut Context<Self>) -> impl IntoElement {
        Button::new("conflict-button")
            .icon(IconName::TriangleAlert)
            .label(format!("{conflict_count}"))
            .ghost()
            .text_color(cx.theme().warning)
            .tooltip(t!("Home.conflict_tooltip", count = conflict_count))
            .on_click(cx.listener(|this, _, window, cx| {
                this.show_conflict_dialog(window, cx);
            }))
    }
}

fn sync_button_label(is_syncing: bool, has_sync_license: bool) -> String {
    if is_syncing {
        t!("Home.syncing").to_string()
    } else if !has_sync_license {
        t!("License.upgrade_to_pro").to_string()
    } else {
        t!("Home.sync").to_string()
    }
}

fn sync_button_tooltip(is_logged_in: bool, has_sync_license: bool) -> String {
    if !is_logged_in && has_sync_license {
        t!("Home.cloud_need_login").to_string()
    } else if !has_sync_license {
        t!("License.pro_required").to_string()
    } else {
        t!("Home.sync_tooltip").to_string()
    }
}

fn encryption_key_button(has_master_key: bool, cx: &mut Context<HomePage>) -> impl IntoElement {
    Button::new("encryption-key-button")
        .icon(IconName::Key)
        .label(if has_master_key {
            t!("Encryption.key_unlocked").to_string()
        } else {
            t!("Encryption.edit_repo_password").to_string()
        })
        .ghost()
        .when(has_master_key, |btn| btn.text_color(cx.theme().success))
        .when(!has_master_key, |btn| {
            btn.text_color(cx.theme().muted_foreground)
        })
        .tooltip(if has_master_key {
            t!("Encryption.key_unlocked_tooltip")
        } else {
            t!("Encryption.key_locked_tooltip")
        })
        .on_click(cx.listener(|this, _, window, cx| {
            this.show_encryption_key_dialog(window, cx);
        }))
}
