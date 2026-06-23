use crate::home::home_layout::{CONTENT_PADDING, SECTION_GAP};
use crate::home_tab::HomePage;
use gpui::{
    AnyElement, Context, InteractiveElement as _, IntoElement, ParentElement as _,
    StatefulInteractiveElement as _, Styled as _, Window, div, px,
};
use gpui_component::{ActiveTheme, Icon, IconName, Sizable as _, v_flex};
use one_core::storage::{StoredConnection, Workspace};
use rust_i18n::t;

impl HomePage {
    pub(crate) fn render_content_area(
        &mut self,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let search_query = self.search_query.read(cx).to_lowercase();
        let selected_id = self.selected_connection_id;
        let reorder_enabled = self.cards_reorder_enabled(cx);
        self.render_workspace_view(&search_query, selected_id, reorder_enabled, window, cx)
            .into_any_element()
    }

    fn render_workspace_view(
        &self,
        search_query: &str,
        selected_id: Option<i64>,
        reorder_enabled: bool,
        window: &Window,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let workspaces = self.filtered_workspace_connections(search_query);
        let unassigned = self.filtered_unassigned_connections(search_query);

        div()
            .id("home-content")
            .size_full()
            .overflow_y_scroll()
            .child(div().w_full().min_w_0().p(px(CONTENT_PADDING)).child(
                self.render_workspace_body(
                    workspaces,
                    unassigned,
                    selected_id,
                    reorder_enabled,
                    window,
                    cx,
                ),
            ))
    }

    fn render_workspace_body(
        &self,
        workspaces: Vec<(Workspace, Vec<StoredConnection>)>,
        unassigned: Vec<StoredConnection>,
        selected_id: Option<i64>,
        reorder_enabled: bool,
        window: &Window,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        if self.connections.is_empty() {
            return self.render_home_empty_state(cx).into_any_element();
        }

        let mut container = v_flex().gap(px(SECTION_GAP)).w_full();
        let mut has_visible = false;
        for (workspace, connections) in workspaces {
            if connections.is_empty() {
                continue;
            }
            has_visible = true;
            container = container.child(self.render_workspace_section(
                workspace,
                connections,
                selected_id,
                reorder_enabled,
                window,
                cx,
            ));
        }

        let has_workspaces = self.workspaces.iter().any(|ws| ws.id.is_some());
        if !unassigned.is_empty() {
            has_visible = true;
            container = if has_workspaces {
                container.child(self.render_unassigned_section(
                    unassigned,
                    selected_id,
                    reorder_enabled,
                    window,
                    cx,
                ))
            } else {
                container.child(self.render_connections_grid(
                    unassigned,
                    None,
                    selected_id,
                    reorder_enabled,
                    window,
                    cx,
                ))
            };
        }

        if has_visible {
            container.into_any_element()
        } else {
            self.render_home_filtered_empty_state(cx).into_any_element()
        }
    }

    fn filtered_workspace_connections(
        &self,
        search_query: &str,
    ) -> Vec<(Workspace, Vec<StoredConnection>)> {
        self.workspaces
            .iter()
            .filter(|ws| {
                self.filtered_workspace_ids.is_empty()
                    || ws
                        .id
                        .map_or(true, |id| self.filtered_workspace_ids.contains(&id))
            })
            .map(|ws| {
                let conn_list = self
                    .connections
                    .iter()
                    .filter(|conn| conn.workspace_id == ws.id)
                    .filter(|conn| self.match_connection(conn, search_query))
                    .filter(|conn| self.match_connection_type(conn))
                    .cloned()
                    .collect();
                (ws.clone(), conn_list)
            })
            .collect()
    }

    fn filtered_unassigned_connections(&self, search_query: &str) -> Vec<StoredConnection> {
        self.connections
            .iter()
            .filter(|conn| conn.workspace_id.is_none())
            .filter(|conn| self.match_connection(conn, search_query))
            .filter(|conn| self.match_connection_type(conn))
            .cloned()
            .collect()
    }

    fn render_home_empty_state(&self, cx: &mut Context<Self>) -> impl IntoElement {
        v_flex()
            .size_full()
            .items_center()
            .justify_center()
            .gap_3()
            .text_color(cx.theme().muted_foreground)
            .child(Icon::new(IconName::Server).large())
            .child(
                div()
                    .text_sm()
                    .child(t!("Home.empty_command_home").to_string()),
            )
            .child(
                div()
                    .text_xs()
                    .child(t!("Home.empty_command_home_hint").to_string()),
            )
    }

    fn render_home_filtered_empty_state(&self, cx: &mut Context<Self>) -> impl IntoElement {
        v_flex()
            .size_full()
            .items_center()
            .justify_center()
            .gap_3()
            .text_color(cx.theme().muted_foreground)
            .child(Icon::new(IconName::Search).large())
            .child(div().text_sm().child(t!("Home.filtered_empty").to_string()))
            .child(
                div()
                    .text_xs()
                    .child(t!("Home.filtered_empty_hint").to_string()),
            )
    }
}
