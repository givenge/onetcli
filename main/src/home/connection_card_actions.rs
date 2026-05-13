use crate::home_tab::HomePage;
use gpui::{
    AppContext as _, Context, InteractiveElement as _, IntoElement, ParentElement as _, Render,
    SharedString, StatefulInteractiveElement as _, Styled as _, Window, div,
    prelude::FluentBuilder as _, px,
};
use gpui_component::{
    ActiveTheme, Icon, IconName, Sizable as _, Size,
    button::{Button, ButtonVariants as _},
    h_flex,
};
use one_core::cloud_sync::can_edit_connection;
use one_core::storage::{ConnectionType, StoredConnection};
use rust_i18n::t;

#[derive(Clone)]
pub(super) struct DragConnectionCard {
    pub(super) connection_id: i64,
    pub(super) workspace_id: Option<i64>,
    name: SharedString,
}

impl DragConnectionCard {
    fn new(connection_id: i64, workspace_id: Option<i64>, name: SharedString) -> Self {
        Self {
            connection_id,
            workspace_id,
            name,
        }
    }
}

impl Render for DragConnectionCard {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .id("drag-connection-card")
            .cursor_grabbing()
            .py_2()
            .px_3()
            .min_w(px(180.0))
            .max_w(px(320.0))
            .rounded(px(8.0))
            .border_1()
            .border_color(cx.theme().drag_border)
            .bg(cx.theme().background)
            .text_sm()
            .text_color(cx.theme().foreground)
            .shadow_lg()
            .opacity(0.9)
            .child(self.name.clone())
    }
}

pub(super) struct ConnectionCardActions {
    pub(super) conn: StoredConnection,
    pub(super) workspace_id: Option<i64>,
    pub(super) drag_name: SharedString,
    pub(super) reorder_enabled: bool,
}

pub(super) fn connection_card_actions(
    data: ConnectionCardActions,
    cx: &mut Context<HomePage>,
) -> impl IntoElement {
    let drag = drag_data(&data.conn, &data);
    let conn = data.conn;
    let is_sftp = conn.connection_type == ConnectionType::SshSftp;
    let can_edit = can_edit_connection(&conn, cx);

    h_flex()
        .absolute()
        .top_2()
        .right_2()
        .gap_1()
        .group_hover("", |style| style.opacity(1.0))
        .opacity(0.0)
        .when_some(drag, |this, drag| this.child(drag_handle(drag, cx)))
        .when(is_sftp, |this| this.child(sftp_button(conn.clone(), cx)))
        .when(can_edit, |this| {
            this.child(duplicate_button(conn.clone(), cx))
                .child(edit_button(conn.clone(), cx))
                .child(delete_button(conn.clone(), cx))
        })
}

struct DragHandleData {
    button_id: i64,
    connection_id: i64,
    workspace_id: Option<i64>,
    name: SharedString,
}

fn drag_data(conn: &StoredConnection, data: &ConnectionCardActions) -> Option<DragHandleData> {
    if !data.reorder_enabled {
        return None;
    }

    conn.id.map(|connection_id| DragHandleData {
        button_id: conn.id.unwrap_or(0),
        connection_id,
        workspace_id: data.workspace_id,
        name: data.drag_name.clone(),
    })
}

fn drag_handle(data: DragHandleData, cx: &mut Context<HomePage>) -> impl IntoElement {
    let drag = DragConnectionCard::new(data.connection_id, data.workspace_id, data.name);
    div()
        .id(SharedString::from(format!("drag-conn-{}", data.button_id)))
        .w(px(28.0))
        .h(px(28.0))
        .flex()
        .items_center()
        .justify_center()
        .rounded(px(6.0))
        .bg(cx.theme().background.opacity(0.9))
        .border_1()
        .border_color(cx.theme().border)
        .cursor_grab()
        .on_drag(drag, |drag, _, window, cx| {
            window.prevent_default();
            cx.stop_propagation();
            cx.new(|_| drag.clone())
        })
        .child(
            Icon::new(IconName::Menu)
                .with_size(Size::Small)
                .text_color(cx.theme().muted_foreground),
        )
}

fn sftp_button(conn: StoredConnection, cx: &mut Context<HomePage>) -> impl IntoElement {
    Button::new(SharedString::from(format!(
        "sftp-conn-{}",
        conn.id.unwrap_or(0)
    )))
    .icon(IconName::Folder1.color())
    .with_size(Size::Small)
    .primary()
    .tooltip(t!("Home.open_sftp"))
    .on_click(cx.listener(move |this, _, window, cx| {
        cx.stop_propagation();
        this.open_sftp_view(conn.clone(), window, cx);
    }))
}

fn duplicate_button(conn: StoredConnection, cx: &mut Context<HomePage>) -> impl IntoElement {
    Button::new(SharedString::from(format!(
        "duplicate-conn-{}",
        conn.id.unwrap_or(0)
    )))
    .icon(IconName::Copy)
    .with_size(Size::Small)
    .primary()
    .tooltip(t!("Home.duplicate_connection"))
    .on_click(cx.listener(move |this, _, window, cx| {
        cx.stop_propagation();
        this.duplicate_connection(conn.clone(), window, cx);
    }))
}

fn edit_button(conn: StoredConnection, cx: &mut Context<HomePage>) -> impl IntoElement {
    let button_id = conn.id.unwrap_or(0);
    let edit_conn_type = conn.connection_type;
    let edit_conn_name = conn.name.clone();

    Button::new(SharedString::from(format!("edit-conn-{}", button_id)))
        .icon(IconName::Edit)
        .with_size(Size::Small)
        .primary()
        .tooltip(t!("Home.edit_connection"))
        .on_click(cx.listener(move |this, _, window, cx| {
            cx.stop_propagation();
            let Some(conn_id) = conn.id else {
                return;
            };
            let conn_name = edit_conn_name.clone();
            match edit_conn_type {
                ConnectionType::SshSftp => {
                    this.editing_connection_id = Some(conn_id);
                    this.show_ssh_form(window, cx);
                }
                ConnectionType::Database => {
                    let db_type = conn.to_db_connection().ok().map(|p| p.database_type);
                    this.confirm_edit_connection(conn_id, conn_name, db_type, window, cx);
                }
                ConnectionType::Redis => {
                    this.editing_connection_id = Some(conn_id);
                    this.show_redis_form(window, cx);
                }
                ConnectionType::MongoDB => {
                    this.editing_connection_id = Some(conn_id);
                    this.show_mongodb_form(window, cx);
                }
                ConnectionType::Serial => {
                    this.editing_connection_id = Some(conn_id);
                    this.show_serial_form(window, cx);
                }
                _ => {}
            }
        }))
}

fn delete_button(conn: StoredConnection, cx: &mut Context<HomePage>) -> impl IntoElement {
    let button_id = conn.id.unwrap_or(0);
    let delete_conn_id = conn.id;
    let delete_conn_name = conn.name.clone();

    Button::new(SharedString::from(format!("delete-conn-{}", button_id)))
        .icon(IconName::Remove)
        .with_size(Size::Small)
        .danger()
        .tooltip(t!("Home.delete_connection"))
        .on_click(cx.listener(move |this, _, window, cx| {
            cx.stop_propagation();
            if let Some(conn_id) = delete_conn_id {
                let conn_name = delete_conn_name.clone();
                this.confirm_delete_connection(conn_id, conn_name, window, cx);
            }
        }))
}
