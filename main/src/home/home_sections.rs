use crate::home::connection_card::ConnectionCardRenderData;
use crate::home::home_layout::{
    CARD_GAP, CARD_HEIGHT, CARD_MIN_WIDTH, CARD_WIDTH, CONTENT_MAX_WIDTH, CONTENT_PADDING,
    GRID_ROW_GAP, SECTION_HEADER_HEIGHT, SECTION_ICON_SLOT, SECTION_INNER_GAP, SIDEBAR_WIDTH,
};
use crate::home_tab::HomePage;
use gpui::{
    Context, ElementId, FontWeight, InteractiveElement as _, IntoElement, ParentElement as _,
    SharedString, Styled as _, Window, div, px,
};
use gpui_component::{
    ActiveTheme, Icon, IconName, Sizable as _, Size, chrome::APP_RAIL_WIDTH, h_flex, v_flex,
};
use one_core::storage::{StoredConnection, Workspace};
use rust_i18n::t;

impl HomePage {
    pub(crate) fn render_workspace_section(
        &self,
        workspace: Workspace,
        connections: Vec<StoredConnection>,
        selected_id: Option<i64>,
        reorder_enabled: bool,
        window: &Window,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let workspace_id = workspace.id;
        v_flex()
            .gap(px(SECTION_INNER_GAP))
            .child(self.render_section_title(workspace_id, workspace.name, connections.len(), cx))
            .child(self.render_connections_grid(
                connections,
                workspace_id,
                selected_id,
                reorder_enabled,
                window,
                cx,
            ))
    }

    pub(crate) fn render_connections_grid(
        &self,
        connections: Vec<StoredConnection>,
        workspace_id: Option<i64>,
        selected_id: Option<i64>,
        reorder_enabled: bool,
        window: &Window,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let min_height = grid_min_height(connections.len(), window);
        let mut container = div()
            .flex()
            .flex_row()
            .flex_wrap()
            .items_start()
            .w_full()
            .min_w_0()
            .max_w(px(CONTENT_MAX_WIDTH))
            .min_h(px(min_height))
            .gap_x(px(CARD_GAP))
            .gap_y(px(GRID_ROW_GAP));
        for conn in connections {
            container = container.child(
                div()
                    .flex_basis(px(CARD_WIDTH))
                    .min_w(px(CARD_MIN_WIDTH))
                    .max_w(px(CARD_WIDTH))
                    .flex_grow()
                    .flex_shrink()
                    .child(self.render_connection_card(
                        ConnectionCardRenderData {
                            conn,
                            workspace_id,
                            selected_id,
                            reorder_enabled,
                        },
                        cx,
                    )),
            );
        }
        container
    }

    pub(crate) fn render_unassigned_section(
        &self,
        connections: Vec<StoredConnection>,
        selected_id: Option<i64>,
        reorder_enabled: bool,
        window: &Window,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        v_flex()
            .gap(px(SECTION_INNER_GAP))
            .child(self.render_unassigned_title(connections.len(), cx))
            .child(self.render_connections_grid(
                connections,
                None,
                selected_id,
                reorder_enabled,
                window,
                cx,
            ))
    }

    fn render_section_title(
        &self,
        workspace_id: Option<i64>,
        title: String,
        count: usize,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        h_flex()
            .h(px(SECTION_HEADER_HEIGHT))
            .items_center()
            .gap_2()
            .child(section_icon_slot(
                Icon::new(IconName::AppsColor)
                    .color()
                    .with_size(Size::Medium),
            ))
            .child(
                div()
                    .id(ElementId::Name(SharedString::from(format!(
                        "workspace-name-{}",
                        workspace_id.unwrap_or(0)
                    ))))
                    .text_base()
                    .font_weight(FontWeight::SEMIBOLD)
                    .text_color(cx.theme().foreground)
                    .child(title),
            )
            .child(connection_count(count, cx))
            .child(div().flex_1())
    }

    fn render_unassigned_title(&self, count: usize, cx: &mut Context<Self>) -> impl IntoElement {
        h_flex()
            .h(px(SECTION_HEADER_HEIGHT))
            .items_center()
            .gap_2()
            .child(section_icon_slot(
                Icon::new(IconName::Folder1).text_color(cx.theme().muted_foreground),
            ))
            .child(
                div()
                    .text_base()
                    .font_weight(FontWeight::SEMIBOLD)
                    .text_color(cx.theme().foreground)
                    .child(
                        t!("Home.unassigned_workspace")
                            .to_string()
                            .into_any_element(),
                    ),
            )
            .child(connection_count(count, cx))
    }
}

fn section_icon_slot(icon: Icon) -> impl IntoElement {
    div()
        .w(px(SECTION_ICON_SLOT))
        .h_full()
        .flex()
        .items_center()
        .justify_center()
        .child(icon)
}

fn connection_count(count: usize, cx: &mut Context<HomePage>) -> impl IntoElement {
    div()
        .text_xs()
        .text_color(cx.theme().muted_foreground)
        .child(t!("Home.connection_count", count = count).to_string())
}

fn grid_min_height(count: usize, window: &Window) -> f32 {
    let rows = grid_row_count(count, window);
    rows as f32 * CARD_HEIGHT + rows.saturating_sub(1) as f32 * GRID_ROW_GAP
}

fn grid_row_count(count: usize, window: &Window) -> usize {
    if count == 0 {
        return 0;
    }

    let columns = grid_column_count(window);
    (count + columns - 1) / columns
}

fn grid_column_count(window: &Window) -> usize {
    let viewport_width = f32::from(window.viewport_size().width);
    let content_width = (viewport_width - APP_RAIL_WIDTH - SIDEBAR_WIDTH - CONTENT_PADDING * 2.0)
        .max(CARD_MIN_WIDTH);
    let grid_width = content_width.min(CONTENT_MAX_WIDTH);
    let columns = ((grid_width + CARD_GAP) / (CARD_MIN_WIDTH + CARD_GAP)).floor() as usize;
    columns.clamp(1, 3)
}
