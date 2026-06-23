use crate::home::connection_card_actions::{
    ConnectionCardActions, DragConnectionCard, connection_card_actions,
};
use crate::home::connection_card_parts::{active_dot, connection_card_body};
use crate::home::connection_display::has_team_badge;
use crate::home::home_layout::{CARD_HEIGHT, CARD_PADDING_X, CARD_PADDING_Y, CARD_RADIUS};
use crate::home::home_strategy::build_connection_open_strategy;
use crate::home_tab::HomePage;
use gpui::{
    AnyElement, Context, Div, InteractiveElement as _, IntoElement, ParentElement as _,
    SharedString, Stateful, StatefulInteractiveElement as _, Styled as _, div,
    prelude::FluentBuilder as _, px,
};
use gpui_component::{ActiveTheme, InteractiveElementExt as _, v_flex};
use one_core::storage::{ActiveConnections, StoredConnection, Workspace};

pub(crate) struct ConnectionCardRenderData {
    pub(crate) conn: StoredConnection,
    pub(crate) workspace_id: Option<i64>,
    pub(crate) selected_id: Option<i64>,
    pub(crate) reorder_enabled: bool,
}

struct ConnectionCardRuntime {
    conn_id: Option<i64>,
    card_conn_id: i64,
    workspace: Option<Workspace>,
    drag_name: SharedString,
    is_selected: bool,
    is_active: bool,
    has_team: bool,
}

struct ConnectionCardInteractions {
    conn_id: Option<i64>,
    card_conn_id: i64,
    workspace_id: Option<i64>,
    reorder_enabled: bool,
    open_conn: StoredConnection,
    workspace: Option<Workspace>,
}

impl HomePage {
    pub(crate) fn render_connection_card(
        &self,
        data: ConnectionCardRenderData,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let runtime = self.connection_card_runtime(&data, cx);
        let card = connection_card_shell(runtime.card_conn_id, runtime.is_selected, cx);
        let card = connection_card_interactions(card, runtime.interactions(&data), cx)
            .when(runtime.is_active, |this| this.child(active_dot(cx)))
            .child(connection_card_actions(
                ConnectionCardActions {
                    conn: data.conn.clone(),
                    workspace_id: data.workspace_id,
                    drag_name: runtime.drag_name,
                    reorder_enabled: data.reorder_enabled,
                },
                cx,
            ))
            .child(connection_card_body(data.conn, runtime.has_team, cx));

        card.into_any_element()
    }

    fn connection_card_runtime(
        &self,
        data: &ConnectionCardRenderData,
        cx: &mut Context<Self>,
    ) -> ConnectionCardRuntime {
        let workspace = data
            .workspace_id
            .and_then(|id| self.workspaces.iter().find(|w| w.id == Some(id)).cloned());

        ConnectionCardRuntime {
            conn_id: data.conn.id,
            card_conn_id: data.conn.id.unwrap_or(0),
            workspace,
            drag_name: data.conn.name.clone().into(),
            is_selected: data.selected_id == data.conn.id,
            is_active: data
                .conn
                .id
                .map_or(false, |id| cx.global::<ActiveConnections>().is_active(id)),
            has_team: has_team_badge(&data.conn),
        }
    }
}

impl ConnectionCardRuntime {
    fn interactions(&self, data: &ConnectionCardRenderData) -> ConnectionCardInteractions {
        ConnectionCardInteractions {
            conn_id: self.conn_id,
            card_conn_id: self.card_conn_id,
            workspace_id: data.workspace_id,
            reorder_enabled: data.reorder_enabled,
            open_conn: data.conn.clone(),
            workspace: self.workspace.clone(),
        }
    }
}

fn connection_card_shell(
    card_conn_id: i64,
    is_selected: bool,
    cx: &mut Context<HomePage>,
) -> Stateful<Div> {
    v_flex()
        .justify_center()
        .id(SharedString::from(format!("conn-card-{}", card_conn_id)))
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
        .group("")
        .when(is_selected, |this| {
            this.border_color(cx.theme().blue.opacity(0.55))
                .bg(cx.theme().background)
                .shadow_md()
                .child(selected_card_accent(cx))
        })
        .when(!is_selected, |this| this.border_color(cx.theme().border))
        .cursor_pointer()
        .hover(|style| {
            style
                .shadow_md()
                .border_color(cx.theme().blue.opacity(0.45))
        })
}

fn selected_card_accent(cx: &mut Context<HomePage>) -> impl IntoElement {
    div()
        .absolute()
        .left_0()
        .top(px(18.0))
        .bottom(px(18.0))
        .w(px(3.0))
        .rounded_r(px(3.0))
        .bg(cx.theme().blue)
}

fn connection_card_interactions(
    card: Stateful<Div>,
    data: ConnectionCardInteractions,
    cx: &mut Context<HomePage>,
) -> Stateful<Div> {
    let workspace_id = data.workspace_id;
    let card_conn_id = data.card_conn_id;
    let reorder_enabled = data.reorder_enabled;
    let conn_id = data.conn_id;
    let open_conn = data.open_conn;
    let workspace = data.workspace;

    card.drag_over::<DragConnectionCard>(move |el, drag, _, cx| {
        if drag.workspace_id == workspace_id && drag.connection_id != card_conn_id {
            el.border_t_2().border_color(cx.theme().drag_border)
        } else {
            el
        }
    })
    .on_drop(cx.listener(move |this, drag: &DragConnectionCard, _, cx| {
        if !reorder_enabled
            || drag.workspace_id != workspace_id
            || drag.connection_id == card_conn_id
        {
            return;
        }
        this.move_connection_card(drag.connection_id, workspace_id, card_conn_id, cx);
    }))
    .on_double_click(cx.listener(move |this, _, w, cx| {
        let strategy = build_connection_open_strategy(open_conn.clone(), workspace.clone());
        strategy.open(this, w, cx);
        cx.notify()
    }))
    .on_click(cx.listener(move |this, _, _, cx| {
        this.selected_connection_id = conn_id;
        cx.notify();
    }))
}
