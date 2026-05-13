use crate::database_view_plugin::{
    ContextMenuEvent, ContextMenuItem, ToolbarButtonType, build_context_menu_for,
    build_toolbar_buttons_for,
};
use crate::db_tree_view::{DbTreeViewEvent, SqlDumpMode, get_icon_for_node_type};
use db::{DbNode, DbNodeType, GlobalDbState, ObjectView};
use gpui::prelude::FluentBuilder;
use gpui::{
    AnyElement, App, AppContext, AsyncApp, Context, DragMoveEvent, Empty, Entity, EntityId,
    EventEmitter, FocusHandle, Focusable, InteractiveElement, IntoElement, ListSizingBehavior,
    MouseButton, MouseDownEvent, ParentElement, Pixels, Render, ScrollWheelEvent, SharedString,
    StatefulInteractiveElement, Styled, Subscription, WeakEntity, Window, div, px, uniform_list,
};
use gpui_component::button::Button;
use gpui_component::input::{Input, InputEvent, InputState};
use gpui_component::label::Label;
use gpui_component::menu::{ContextMenuExt, PopupMenu, PopupMenuItem};
use gpui_component::notification::Notification;
use gpui_component::scroll::ScrollableElement;
use gpui_component::{
    ActiveTheme, Icon, IconName, Sizable, Size, h_flex, table::Column, tooltip::Tooltip, v_flex,
};
use gpui_component::{InteractiveElementExt, WindowExt};
use one_core::storage::manager::get_queries_dir;
use one_core::storage::{
    ActiveConnections, ConnectionRepository, DatabaseType, DbConnectionConfig, GlobalStorageState,
    StorageManager, Workspace,
};
use one_core::tab_container::{TabContent, TabContentEvent};
use one_core::utils::debouncer::Debouncer;
use rust_i18n::t;
use std::collections::{HashMap, HashSet};
use std::ops::Range;
use std::sync::Arc;
use std::time::Duration;

const DB_OBJECTS_ROW_NUMBER_WIDTH: Pixels = px(48.0);
const DB_OBJECTS_COLUMN_RESIZE_HANDLE_WIDTH: Pixels = px(6.0);
const DB_OBJECTS_MIN_COLUMN_WIDTH: Pixels = px(64.0);

#[derive(Clone)]
struct DbObjectsResizeColumn {
    entity_id: EntityId,
    col_ix: usize,
}

impl Render for DbObjectsResizeColumn {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        Empty
    }
}

#[derive(Clone, Copy, Debug)]
struct ColumnResizeState {
    col_ix: usize,
    start_x: Pixels,
    start_width: Pixels,
}

fn format_timestamp(ts: i64) -> String {
    use chrono::{DateTime, Local};
    if let Some(dt) = DateTime::from_timestamp_millis(ts) {
        let local: DateTime<Local> = dt.into();
        local.format("%Y-%m-%d %H:%M:%S").to_string()
    } else {
        "".to_string()
    }
}

/// 数据库对象面板事件 - 统一的表格交互事件
#[derive(Clone, Debug)]
pub enum DatabaseObjectsEvent {
    /// 刷新当前视图
    Refresh { node: DbNode },

    /// 将数据库添加到树视图并展开
    AddDatabaseToTree { node: DbNode },

    /// 新建数据库
    CreateDatabase { node: DbNode },

    /// 打开数据库或 Schema 的 ER 图
    OpenErDiagram { node: DbNode },

    /// 编辑数据库
    EditDatabase { node: DbNode },

    /// 关闭数据库
    CloseDatabase { node: DbNode },

    /// 删除数据库
    DeleteDatabase { node: DbNode },

    /// 删除连接
    DeleteConnection { node: DbNode },

    /// 关闭连接
    CloseConnection { node: DbNode },

    /// 打开表数据
    OpenTableData { node: DbNode },

    /// 设计表（新建或编辑）
    DesignTable { node: DbNode },

    /// 重命名表
    RenameTable { node: DbNode },

    /// 创建备份表
    CopyTable { node: DbNode },

    /// 清空表
    TruncateTable { node: DbNode },

    /// 删除表
    DeleteTable { node: DbNode },

    /// 导入数据
    ImportData { node: DbNode },

    /// 导出表
    ExportData { node: DbNode },

    /// 转储 SQL 文件
    DumpSqlFile { node: DbNode, mode: SqlDumpMode },

    /// 打开视图数据
    OpenViewData { node: DbNode },

    /// 删除视图
    DeleteView { node: DbNode },

    /// 新建查询
    CreateNewQuery { node: DbNode },

    /// 打开命名查询
    OpenNamedQuery { node: DbNode },

    /// 重命名查询
    RenameQuery { node: DbNode },

    /// 删除查询
    DeleteQuery { node: DbNode },

    /// 运行 SQL 文件
    RunSqlFile { node: DbNode },

    /// 删除模式/Schema
    DeleteSchema { node: DbNode },

    /// 新建模式/Schema
    CreateSchema { node: DbNode },

    /// 批量操作
    Batch {
        action: DatabaseObjectsBatchAction,
        nodes: Vec<DbNode>,
    },
}

#[derive(Clone, Debug)]
pub enum DatabaseObjectsBatchAction {
    DeleteConnection,
    DeleteDatabase,
    DeleteSchema,
    DeleteTable,
    DeleteView,
    DeleteQuery,
}

pub struct DatabaseObjects {
    loaded_data: Entity<ObjectView>,
    // 直接管理表格数据
    columns: Vec<Column>,
    rows: Vec<Vec<String>>,
    filtered_rows: Vec<usize>,
    db_node_type: DbNodeType,
    focus_handle: FocusHandle,
    workspace: Option<Workspace>,
    search_input: Entity<InputState>,
    search_query: String,
    search_seq: u64,
    search_debouncer: Arc<Debouncer>,
    current_node: Option<DbNode>,
    selected_indices: HashSet<usize>,
    ddl_preview_content: SharedString,
    ddl_preview_loading: bool,
    ddl_preview_table_name: Option<String>,
    ddl_preview_request_seq: u64,
    resizing_column: Option<ColumnResizeState>,
    _subscriptions: Vec<Subscription>,
}

impl DatabaseObjects {
    fn stop_vertical_scroll_bubble(
        &mut self,
        event: &ScrollWheelEvent,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let delta = event.delta.pixel_delta(window.line_height());
        if delta.y != Pixels::ZERO && delta.y.abs() >= delta.x.abs() {
            cx.stop_propagation();
        }
    }

    pub fn new(workspace: Option<Workspace>, window: &mut Window, cx: &mut Context<Self>) -> Self {
        let loaded_data = cx.new(|_| ObjectView::default());
        let focus_handle = cx.focus_handle();
        let search_input = cx.new(|cx| {
            InputState::new(window, cx)
                .placeholder(t!("Common.search"))
                .clean_on_escape()
        });
        let search_debouncer = Arc::new(Debouncer::new(Duration::from_millis(250)));

        let search_sub = cx.subscribe_in(
            &search_input,
            window,
            |this: &mut Self,
             input: &Entity<InputState>,
             event: &InputEvent,
             _window,
             cx: &mut Context<Self>| {
                if let InputEvent::Change = event {
                    let query = input.read(cx).text().to_string();

                    this.search_seq += 1;
                    let current_seq = this.search_seq;
                    let debouncer = Arc::clone(&this.search_debouncer);
                    let query_for_task = query.clone();

                    cx.spawn(async move |view, cx| {
                        if debouncer.debounce(cx).await {
                            _ = view.update(cx, |this, cx| {
                                if this.search_seq == current_seq {
                                    this.search_query = query_for_task.to_lowercase();
                                    this.selected_indices.clear();
                                    this.apply_filter();
                                    cx.notify();
                                }
                            });
                        }
                    })
                    .detach();
                }
            },
        );

        let storage_manager = cx.global::<GlobalStorageState>().storage.clone();
        let clone_workspace = workspace.clone();
        cx.spawn(async move |entity: WeakEntity<Self>, cx: &mut AsyncApp| {
            let result = Self::load_connection_list_view(storage_manager, clone_workspace);
            if let Some(view) = result {
                let columns = view.columns.clone();
                let rows = view.rows.clone();
                let db_node_type = view.db_node_type;
                entity
                    .update(cx, move |this, cx| {
                        this.loaded_data.update(cx, |data, _cx| {
                            *data = view;
                        });
                        this.columns = columns;
                        this.rows = rows;
                        this.filtered_rows = (0..this.rows.len()).collect();
                        this.db_node_type = db_node_type;
                        this.selected_indices.clear();
                        cx.notify();
                    })
                    .ok();
            }
        })
        .detach();

        Self {
            loaded_data,
            columns: vec![],
            rows: vec![],
            filtered_rows: vec![],
            db_node_type: DbNodeType::default(),
            focus_handle,
            workspace,
            search_input,
            search_query: "".to_string(),
            search_seq: 0,
            search_debouncer,
            current_node: None,
            selected_indices: HashSet::new(),
            ddl_preview_content: SharedString::new_static(""),
            ddl_preview_loading: false,
            ddl_preview_table_name: None,
            ddl_preview_request_seq: 0,
            resizing_column: None,
            _subscriptions: vec![search_sub],
        }
    }

    fn handle_row_double_click(&self, row: usize, cx: &mut Context<Self>) {
        let Some(node) = self.build_node_for_row(row) else {
            return;
        };

        let event = match node.node_type {
            DbNodeType::Table => DatabaseObjectsEvent::OpenTableData { node },
            DbNodeType::View => DatabaseObjectsEvent::OpenViewData { node },
            DbNodeType::NamedQuery => DatabaseObjectsEvent::OpenNamedQuery { node },
            DbNodeType::Database => DatabaseObjectsEvent::AddDatabaseToTree { node },
            _ => return,
        };

        cx.emit(event);
    }

    pub fn handle_node_selected(
        &mut self,
        node: DbNode,
        _config: DbConnectionConfig,
        cx: &mut Context<Self>,
    ) {
        match node.node_type {
            DbNodeType::Connection
            | DbNodeType::Database
            | DbNodeType::Schema
            | DbNodeType::TablesFolder
            | DbNodeType::Table
            | DbNodeType::ViewsFolder
            | DbNodeType::View
            | DbNodeType::QueriesFolder
            | DbNodeType::NamedQuery => {}
            _ => return,
        }

        if !node.children_loaded && node.node_type != DbNodeType::Connection {
            return;
        }

        self.current_node = Some(node.clone());
        self.selected_indices.clear();
        self.clear_ddl_preview(cx);
        let node_clone = node.clone();
        let storage_manager = cx.global::<GlobalStorageState>().storage.clone();
        let global_state = cx.global::<GlobalDbState>().clone();
        let workspace = self.workspace.clone();
        let connection_id = node.connection_id.clone();
        cx.spawn(async move |entity: WeakEntity<Self>, cx: &mut AsyncApp| {
            let result: Option<ObjectView> =
                if !node_clone.children_loaded && node_clone.node_type == DbNodeType::Connection {
                    Self::load_connection_list_view(storage_manager, workspace)
                } else if node_clone.node_type == DbNodeType::QueriesFolder
                    || node_clone.node_type == DbNodeType::NamedQuery
                {
                    Self::load_queries_list_view(node_clone.clone()).await
                } else {
                    global_state
                        .load_object_view(cx, connection_id, node_clone)
                        .await
                        .ok()
                        .flatten()
                };

            if let Some(view) = result {
                let columns = view.columns.clone();
                let rows = view.rows.clone();
                let db_node_type = view.db_node_type;
                entity
                    .update(cx, move |this, cx| {
                        let search_query = this.search_query.clone();
                        this.loaded_data.update(cx, |data, _cx| {
                            *data = view;
                        });
                        this.columns = columns;
                        this.rows = rows;
                        this.db_node_type = db_node_type;
                        if !search_query.is_empty() {
                            this.apply_filter();
                        } else {
                            this.filtered_rows = (0..this.rows.len()).collect();
                        }
                        this.selected_indices.clear();
                        cx.notify();
                    })
                    .ok();
            }
        })
        .detach();
    }

    fn toggle_selection(&mut self, row_ix: usize, multi_select: bool) {
        if multi_select {
            if self.selected_indices.contains(&row_ix) {
                self.selected_indices.remove(&row_ix);
            } else {
                self.selected_indices.insert(row_ix);
            }
        } else if !self.selected_indices.contains(&row_ix) {
            self.selected_indices.clear();
            self.selected_indices.insert(row_ix);
        }
    }

    fn select_context_row(&mut self, row_ix: usize) {
        let is_single_selected =
            self.selected_indices.len() == 1 && self.selected_indices.contains(&row_ix);
        if !is_single_selected {
            self.selected_indices.clear();
            self.selected_indices.insert(row_ix);
        }
    }

    fn clear_ddl_preview(&mut self, cx: &mut Context<Self>) {
        self.ddl_preview_loading = false;
        self.ddl_preview_table_name = None;
        self.ddl_preview_request_seq = self.ddl_preview_request_seq.saturating_add(1);
        self.ddl_preview_content = SharedString::new_static("");
        cx.notify();
    }

    fn set_ddl_preview_content(
        &mut self,
        table_name: Option<String>,
        content: impl Into<String>,
        cx: &mut Context<Self>,
    ) {
        self.ddl_preview_table_name = table_name;
        self.ddl_preview_content = content.into().into();
        cx.notify();
    }

    fn refresh_ddl_preview_for_selection(&mut self, cx: &mut Context<Self>) {
        if self.selected_indices.len() != 1 {
            self.clear_ddl_preview(cx);
            return;
        }

        let Some(row_ix) = self.selected_indices.iter().next().copied() else {
            self.clear_ddl_preview(cx);
            return;
        };
        let Some(node) = self.build_node_for_row(row_ix) else {
            self.clear_ddl_preview(cx);
            return;
        };
        if node.node_type != DbNodeType::Table {
            self.clear_ddl_preview(cx);
            return;
        }

        let Some(database) = node.get_database_name() else {
            self.clear_ddl_preview(cx);
            return;
        };
        let Some(table) = node.get_table_name() else {
            self.clear_ddl_preview(cx);
            return;
        };

        self.ddl_preview_request_seq = self.ddl_preview_request_seq.saturating_add(1);
        let request_seq = self.ddl_preview_request_seq;
        let schema = node.get_schema_name();
        let connection_id = node.connection_id.clone();
        let table_name = table.clone();
        let global_state = cx.global::<GlobalDbState>().clone();
        self.ddl_preview_loading = true;
        self.ddl_preview_table_name = Some(table_name.clone());
        self.ddl_preview_content = "Loading DDL...".into();

        cx.spawn(async move |entity: WeakEntity<Self>, cx: &mut AsyncApp| {
            let result = global_state
                .export_table_create_sql(cx, connection_id, database, schema, table)
                .await;

            let _ = entity.update(cx, |this, cx| {
                if this.ddl_preview_request_seq != request_seq {
                    return;
                }
                this.ddl_preview_loading = false;
                match result {
                    Ok(ddl) if !ddl.trim().is_empty() => {
                        this.set_ddl_preview_content(Some(table_name), ddl, cx);
                    }
                    Ok(_) => {
                        this.set_ddl_preview_content(
                            Some(table_name),
                            "DDL preview is not available for this table.",
                            cx,
                        );
                    }
                    Err(err) => {
                        this.set_ddl_preview_content(
                            Some(table_name),
                            format!("Failed to load DDL:\n{err}"),
                            cx,
                        );
                    }
                }
                cx.notify();
            });
        })
        .detach();
    }

    fn apply_filter(&mut self) {
        if self.search_query.is_empty() {
            self.filtered_rows = (0..self.rows.len()).collect();
        } else {
            self.filtered_rows = self
                .rows
                .iter()
                .enumerate()
                .filter(|(_, row)| {
                    row.iter()
                        .any(|cell| cell.to_lowercase().contains(&self.search_query))
                })
                .map(|(idx, _)| idx)
                .collect();
        }
    }

    fn load_connection_list_view(
        storage_manager: StorageManager,
        workspace: Option<Workspace>,
    ) -> Option<ObjectView> {
        let conn_repo = storage_manager.get::<ConnectionRepository>()?;
        let w = workspace?;
        let connections = conn_repo.list_by_workspace(w.id).ok()?;

        let rows = connections
            .iter()
            .map(|stored_conn| {
                let created = stored_conn
                    .created_at
                    .map(|ts| format_timestamp(ts))
                    .unwrap_or_default();
                let updated = stored_conn
                    .updated_at
                    .map(|ts| format_timestamp(ts))
                    .unwrap_or_default();
                let remark = stored_conn.remark.clone().unwrap_or_default();
                let db_type = stored_conn
                    .to_db_connection()
                    .map(|c| c.database_type)
                    .unwrap_or(DatabaseType::MySQL);
                let connection_id = stored_conn.id.map(|id| id.to_string()).unwrap_or_default();
                vec![
                    stored_conn.name.clone(),
                    connection_id,
                    db_type.as_str().into(),
                    created,
                    updated,
                    remark,
                ]
            })
            .collect();

        Some(ObjectView {
            db_node_type: DbNodeType::Connection,
            columns: vec![
                Column::new("name", t!("ConnectionForm.connection_name")).width(200.0),
                Column::new("id", "ID").width(80.0),
                Column::new("type", t!("Common.type")),
                Column::new("created_at", t!("Table.created_at")).width(200.0),
                Column::new("updated_at", t!("Table.updated_at")).width(200.0),
                Column::new("remark", t!("ConnectionForm.remark")).width(250.0),
            ],
            rows,
            title: t!("Connection.connection_list").to_string(),
        })
    }

    async fn load_queries_list_view(node: DbNode) -> Option<ObjectView> {
        use std::time::UNIX_EPOCH;

        let database_name = node.get_database_name().unwrap_or_default();
        let database_type = node.database_type.as_str();
        let connection_id = node.connection_id.clone();

        let queries_dir = get_queries_dir().ok()?;
        let query_path = queries_dir
            .join(database_type)
            .join(&connection_id)
            .join(&database_name);

        if !query_path.exists() {
            return Some(ObjectView {
                db_node_type: DbNodeType::NamedQuery,
                columns: vec![
                    Column::new("name", t!("Query.query_name")).width(200.0),
                    Column::new("created_at", t!("Table.created_at")).width(180.0),
                    Column::new("updated_at", t!("Table.updated_at")).width(180.0),
                ],
                rows: vec![],
                title: t!("Query.query_list").to_string(),
            });
        }

        let entries = std::fs::read_dir(&query_path).ok()?;
        let mut rows: Vec<Vec<String>> = Vec::new();

        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().map_or(false, |ext| ext == "sql") {
                let file_name = path
                    .file_stem()
                    .and_then(|s| s.to_str())
                    .unwrap_or("unknown")
                    .to_string();

                let (created, modified) = if let Ok(metadata) = std::fs::metadata(&path) {
                    let created_time = metadata
                        .created()
                        .ok()
                        .and_then(|t| t.duration_since(UNIX_EPOCH).ok())
                        .map(|d| format_timestamp(d.as_millis() as i64))
                        .unwrap_or_default();
                    let modified_time = metadata
                        .modified()
                        .ok()
                        .and_then(|t| t.duration_since(UNIX_EPOCH).ok())
                        .map(|d| format_timestamp(d.as_millis() as i64))
                        .unwrap_or_default();
                    (created_time, modified_time)
                } else {
                    (String::new(), String::new())
                };

                rows.push(vec![file_name, created, modified]);
            }
        }

        rows.sort_by(|a, b| a[0].cmp(&b[0]));

        Some(ObjectView {
            db_node_type: DbNodeType::NamedQuery,
            columns: vec![
                Column::new("name", t!("Query.query_name")).width(200.0),
                Column::new("created_at", t!("Table.created_at")).width(180.0),
                Column::new("updated_at", t!("Table.updated_at")).width(180.0),
            ],
            rows,
            title: t!("Query.query_list").to_string(),
        })
    }

    fn build_node_for_row(&self, row_ix: usize) -> Option<DbNode> {
        let db_node_type = self.db_node_type;

        let original_row = self.filtered_rows.get(row_ix).copied()?;
        let row_data = self.rows.get(original_row)?;
        let name = row_data.first().cloned()?;

        // 特殊处理：当 current_node 为 None 且显示连接列表时
        if self.current_node.is_none() && db_node_type == DbNodeType::Connection {
            let connection_id = row_data.get(1).cloned().unwrap_or_default();
            let db_type_str = row_data.get(2).cloned().unwrap_or_default();
            let database_type = DatabaseType::from_str(&db_type_str).unwrap_or(DatabaseType::MySQL);

            return Some(DbNode::new(
                connection_id.clone(),
                name,
                DbNodeType::Connection,
                connection_id,
                database_type,
            ));
        }

        let current_node = self.current_node.as_ref()?;
        let connection_id = current_node.connection_id.clone();
        let database_type = current_node.database_type;

        let mut metadata: HashMap<String, String> = current_node.metadata.clone();
        let database = metadata.get("database").cloned().unwrap_or_default();

        let (node_id, target_node_type) = match db_node_type {
            DbNodeType::Connection => {
                if current_node.children_loaded {
                    metadata.insert("database".to_string(), name.clone());
                    (format!("{}:{}", connection_id, name), DbNodeType::Database)
                } else {
                    // 连接未展开时，从行数据获取 connection_id
                    let row_connection_id = row_data.get(1).cloned().unwrap_or_default();
                    let db_type_str = row_data.get(2).cloned().unwrap_or_default();
                    let row_database_type =
                        DatabaseType::from_str(&db_type_str).unwrap_or(DatabaseType::MySQL);
                    return Some(DbNode::new(
                        row_connection_id.clone(),
                        name,
                        DbNodeType::Connection,
                        row_connection_id,
                        row_database_type,
                    ));
                }
            }
            DbNodeType::Database => {
                if current_node.node_type == DbNodeType::Connection {
                    metadata.insert("database".to_string(), name.clone());
                    (format!("{}:{}", connection_id, name), DbNodeType::Database)
                } else {
                    let db = if database.is_empty() {
                        current_node.name.clone()
                    } else {
                        database.clone()
                    };
                    metadata.insert("database".to_string(), db.clone());
                    metadata.insert("table".to_string(), name.clone());
                    (
                        format!("{}:{}:table_folder:{}", connection_id, db, name),
                        DbNodeType::Table,
                    )
                }
            }
            DbNodeType::TablesFolder | DbNodeType::Table => {
                let db = if database.is_empty() {
                    current_node.name.clone()
                } else {
                    database.clone()
                };
                metadata.insert("database".to_string(), db.clone());
                metadata.insert("table".to_string(), name.clone());
                (
                    format!("{}:{}:table_folder:{}", connection_id, db, name),
                    DbNodeType::Table,
                )
            }
            DbNodeType::Schema => {
                if current_node.node_type == DbNodeType::Connection {
                    metadata.insert("schema".to_string(), name.clone());
                    (format!("{}:{}", connection_id, name), DbNodeType::Schema)
                } else {
                    let db = metadata
                        .get("database")
                        .cloned()
                        .unwrap_or_else(|| current_node.name.clone());
                    let schema = current_node.name.clone();
                    metadata.insert("database".to_string(), db.clone());
                    metadata.insert("schema".to_string(), schema.clone());
                    metadata.insert("table".to_string(), name.clone());
                    (
                        format!("{}:{}:{}:table_folder:{}", connection_id, db, schema, name),
                        DbNodeType::Table,
                    )
                }
            }
            DbNodeType::ViewsFolder | DbNodeType::View => {
                let db = if database.is_empty() {
                    current_node.name.clone()
                } else {
                    database.clone()
                };
                metadata.insert("database".to_string(), db.clone());
                metadata.insert("view".to_string(), name.clone());
                (
                    format!("{}:{}:views_folder:{}", connection_id, db, name),
                    DbNodeType::View,
                )
            }
            DbNodeType::QueriesFolder | DbNodeType::NamedQuery => {
                let query_id = row_data.get(1).cloned().unwrap_or_default();
                metadata.insert("query_name".to_string(), name.clone());
                metadata.insert("query_id".to_string(), query_id.clone());
                (
                    format!("{}:queries:{}", connection_id, query_id),
                    DbNodeType::NamedQuery,
                )
            }
            _ => return None,
        };

        Some(
            DbNode::new(
                node_id,
                name,
                target_node_type,
                connection_id,
                database_type,
            )
            .with_metadata(metadata),
        )
    }

    fn build_nodes_for_selected_rows(&self) -> Vec<DbNode> {
        let mut selected_rows: Vec<usize> = self.selected_indices.iter().copied().collect();
        selected_rows.sort_unstable();
        selected_rows
            .into_iter()
            .filter_map(|row_ix| self.build_node_for_row(row_ix))
            .collect()
    }

    fn event_for_tree_event(event: &DbTreeViewEvent, node: DbNode) -> Option<DatabaseObjectsEvent> {
        match event {
            DbTreeViewEvent::OpenTableData { .. } => {
                Some(DatabaseObjectsEvent::OpenTableData { node })
            }
            DbTreeViewEvent::DesignTable { .. } => Some(DatabaseObjectsEvent::DesignTable { node }),
            DbTreeViewEvent::RenameTable { .. } => Some(DatabaseObjectsEvent::RenameTable { node }),
            DbTreeViewEvent::CopyTable { .. } => Some(DatabaseObjectsEvent::CopyTable { node }),
            DbTreeViewEvent::TruncateTable { .. } => {
                Some(DatabaseObjectsEvent::TruncateTable { node })
            }
            DbTreeViewEvent::DeleteTable { .. } => Some(DatabaseObjectsEvent::DeleteTable { node }),
            DbTreeViewEvent::ImportData { .. } => Some(DatabaseObjectsEvent::ImportData { node }),
            DbTreeViewEvent::ExportData { .. } => Some(DatabaseObjectsEvent::ExportData { node }),
            DbTreeViewEvent::DumpSqlFile { mode, .. } => {
                Some(DatabaseObjectsEvent::DumpSqlFile { node, mode: *mode })
            }
            DbTreeViewEvent::OpenViewData { .. } => {
                Some(DatabaseObjectsEvent::OpenViewData { node })
            }
            DbTreeViewEvent::DeleteView { .. } => Some(DatabaseObjectsEvent::DeleteView { node }),
            DbTreeViewEvent::CreateNewQuery { .. } => {
                Some(DatabaseObjectsEvent::CreateNewQuery { node })
            }
            DbTreeViewEvent::OpenNamedQuery { .. } => {
                Some(DatabaseObjectsEvent::OpenNamedQuery { node })
            }
            DbTreeViewEvent::RenameQuery { .. } => Some(DatabaseObjectsEvent::RenameQuery { node }),
            DbTreeViewEvent::DeleteQuery { .. } => Some(DatabaseObjectsEvent::DeleteQuery { node }),
            DbTreeViewEvent::CloseConnection { .. } => {
                Some(DatabaseObjectsEvent::CloseConnection { node })
            }
            DbTreeViewEvent::DeleteConnection { .. } => {
                Some(DatabaseObjectsEvent::DeleteConnection { node })
            }
            DbTreeViewEvent::CreateDatabase { .. } => {
                Some(DatabaseObjectsEvent::CreateDatabase { node })
            }
            DbTreeViewEvent::OpenErDiagram { .. } => {
                Some(DatabaseObjectsEvent::OpenErDiagram { node })
            }
            DbTreeViewEvent::EditDatabase { .. } => {
                Some(DatabaseObjectsEvent::EditDatabase { node })
            }
            DbTreeViewEvent::CloseDatabase { .. } => {
                Some(DatabaseObjectsEvent::CloseDatabase { node })
            }
            DbTreeViewEvent::DeleteDatabase { .. } => {
                Some(DatabaseObjectsEvent::DeleteDatabase { node })
            }
            DbTreeViewEvent::CreateSchema { .. } => {
                Some(DatabaseObjectsEvent::CreateSchema { node })
            }
            DbTreeViewEvent::DeleteSchema { .. } => {
                Some(DatabaseObjectsEvent::DeleteSchema { node })
            }
            DbTreeViewEvent::RunSqlFile { .. } => Some(DatabaseObjectsEvent::RunSqlFile { node }),
            _ => None,
        }
    }

    fn context_menu_has_action(items: &[ContextMenuItem], node: &DbNode) -> bool {
        items.iter().any(|item| match item {
            ContextMenuItem::Item { event, .. } => match event {
                ContextMenuEvent::TreeEvent(tree_event) => {
                    Self::event_for_tree_event(tree_event, node.clone()).is_some()
                }
                ContextMenuEvent::Custom(_) => false,
            },
            ContextMenuItem::Submenu { items, .. } => Self::context_menu_has_action(items, node),
            ContextMenuItem::Separator => false,
        })
    }

    fn push_context_menu_item(
        menu: PopupMenu,
        item: PopupMenuItem,
        pending_separator: &mut bool,
    ) -> PopupMenu {
        let menu = if *pending_separator {
            *pending_separator = false;
            menu.separator()
        } else {
            menu
        };
        menu.item(item)
    }

    fn context_menu_action_item(
        label: String,
        tree_event: DbTreeViewEvent,
        requires_active: bool,
        is_active: bool,
        node: &DbNode,
        view: &Entity<Self>,
        window: &mut Window,
    ) -> Option<PopupMenuItem> {
        let objects_event = Self::event_for_tree_event(&tree_event, node.clone())?;
        let view_ref = view.clone();
        Some(
            PopupMenuItem::new(label)
                .disabled(requires_active && !is_active)
                .on_click(window.listener_for(&view_ref, move |_this, _, _, cx| {
                    cx.emit(objects_event.clone());
                })),
        )
    }

    fn context_menu_submenu_item(
        label: String,
        items: Vec<ContextMenuItem>,
        is_active: bool,
        node: DbNode,
        view: &Entity<Self>,
        window: &mut Window,
        cx: &mut Context<PopupMenu>,
        requires_active: bool,
    ) -> Option<PopupMenuItem> {
        if !Self::context_menu_has_action(&items, &node) {
            return None;
        }
        let view_submenu = view.clone();
        let submenu_node = node.clone();
        let submenu_entity = PopupMenu::build(window, cx, move |submenu, window, cx| {
            Self::render_context_menu_items(
                submenu,
                items.clone(),
                is_active,
                submenu_node.clone(),
                &view_submenu,
                window,
                cx,
            )
        });
        Some(PopupMenuItem::submenu(label, submenu_entity).disabled(requires_active && !is_active))
    }

    fn render_context_menu_items(
        mut menu: PopupMenu,
        items: Vec<ContextMenuItem>,
        is_active: bool,
        node: DbNode,
        view: &Entity<Self>,
        window: &mut Window,
        cx: &mut Context<PopupMenu>,
    ) -> PopupMenu {
        let mut has_item = false;
        let mut pending_separator = false;

        for item in items {
            let menu_item = match item {
                ContextMenuItem::Item {
                    label,
                    event: ContextMenuEvent::TreeEvent(tree_event),
                    requires_active,
                } => Self::context_menu_action_item(
                    label,
                    tree_event,
                    requires_active,
                    is_active,
                    &node,
                    view,
                    window,
                ),
                ContextMenuItem::Item { .. } => None,
                ContextMenuItem::Separator => {
                    if has_item {
                        pending_separator = true;
                    }
                    continue;
                }
                ContextMenuItem::Submenu {
                    label,
                    items: sub_items,
                    requires_active,
                } => Self::context_menu_submenu_item(
                    label,
                    sub_items,
                    is_active,
                    node.clone(),
                    view,
                    window,
                    cx,
                    requires_active,
                ),
            };

            let Some(menu_item) = menu_item else { continue };
            menu = Self::push_context_menu_item(menu, menu_item, &mut pending_separator);
            has_item = true;
        }

        menu
    }

    fn build_context_menu_for_row(
        mut menu: PopupMenu,
        row_ix: usize,
        view: &Entity<Self>,
        window: &mut Window,
        cx: &mut Context<PopupMenu>,
    ) -> PopupMenu {
        let Some(node) = view.read(cx).build_node_for_row(row_ix) else {
            return menu;
        };
        let refresh_node = view
            .read(cx)
            .current_node
            .clone()
            .unwrap_or_else(|| node.clone());
        let is_active = node
            .connection_id
            .parse::<i64>()
            .ok()
            .map(|conn_id| cx.global::<ActiveConnections>().is_active(conn_id))
            .unwrap_or(false);

        let _ = view.update(cx, |this, cx| {
            this.select_context_row(row_ix);
            this.refresh_ddl_preview_for_selection(cx);
            cx.notify();
        });

        let menu_items = build_context_menu_for(node.database_type, &node.id, node.node_type, cx);
        menu = Self::render_context_menu_items(menu, menu_items, is_active, node, view, window, cx);

        let view_ref = view.clone();
        menu.item(
            PopupMenuItem::new(t!("Common.refresh")).on_click(window.listener_for(
                &view_ref,
                move |_this, _, _, cx| {
                    cx.emit(DatabaseObjectsEvent::Refresh {
                        node: refresh_node.clone(),
                    });
                },
            )),
        )
    }

    fn batch_action_for_event(event: &DatabaseObjectsEvent) -> Option<DatabaseObjectsBatchAction> {
        match event {
            DatabaseObjectsEvent::DeleteConnection { .. } => {
                Some(DatabaseObjectsBatchAction::DeleteConnection)
            }
            DatabaseObjectsEvent::DeleteDatabase { .. } => {
                Some(DatabaseObjectsBatchAction::DeleteDatabase)
            }
            DatabaseObjectsEvent::DeleteSchema { .. } => {
                Some(DatabaseObjectsBatchAction::DeleteSchema)
            }
            DatabaseObjectsEvent::DeleteTable { .. } => {
                Some(DatabaseObjectsBatchAction::DeleteTable)
            }
            DatabaseObjectsEvent::DeleteView { .. } => Some(DatabaseObjectsBatchAction::DeleteView),
            DatabaseObjectsEvent::DeleteQuery { .. } => {
                Some(DatabaseObjectsBatchAction::DeleteQuery)
            }
            _ => None,
        }
    }

    fn allow_multi_event(event: &DatabaseObjectsEvent) -> bool {
        matches!(
            event,
            DatabaseObjectsEvent::OpenTableData { .. }
                | DatabaseObjectsEvent::OpenViewData { .. }
                | DatabaseObjectsEvent::OpenNamedQuery { .. }
                | DatabaseObjectsEvent::DesignTable { .. }
                | DatabaseObjectsEvent::CloseConnection { .. }
        )
    }

    fn begin_column_resize(&mut self, col_ix: usize, start_x: Pixels) {
        let Some(column) = self.columns.get(col_ix) else {
            return;
        };
        if !column.resizable {
            return;
        }

        self.resizing_column = Some(ColumnResizeState {
            col_ix,
            start_x,
            start_width: column.width,
        });
    }

    fn resize_column(&mut self, col_ix: usize, pointer_x: Pixels, cx: &mut Context<Self>) {
        let Some(resizing) = self.resizing_column else {
            return;
        };
        if resizing.col_ix != col_ix {
            return;
        }

        let Some(column) = self.columns.get_mut(col_ix) else {
            return;
        };

        let min_width = column.min_width.max(DB_OBJECTS_MIN_COLUMN_WIDTH);
        let mut next_width = resizing.start_width + pointer_x - resizing.start_x;
        if next_width < min_width {
            next_width = min_width;
        }
        if next_width > column.max_width {
            next_width = column.max_width;
        }

        if column.width != next_width {
            column.width = next_width;
            cx.notify();
        }
    }

    fn finish_column_resize(&mut self, cx: &mut Context<Self>) {
        if self.resizing_column.take().is_some() {
            cx.notify();
        }
    }

    fn drag_resize_column(
        &mut self,
        col_ix: usize,
        event: &DragMoveEvent<DbObjectsResizeColumn>,
        cx: &mut Context<Self>,
    ) {
        let drag = event.drag(cx);
        if drag.entity_id != cx.entity_id() || drag.col_ix != col_ix {
            return;
        }
        self.resize_column(col_ix, event.event.position.x, cx);
    }

    fn render_column_resize_indicator(
        &self,
        group_id: &SharedString,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        div()
            .w(px(1.0))
            .h_full()
            .bg(cx.theme().table_row_border)
            .group_hover(group_id, |this| this.bg(cx.theme().border))
            .into_any_element()
    }

    fn render_column_resize_handle(
        &self,
        col_ix: usize,
        column: &Column,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        if !column.resizable {
            return div().into_any_element();
        }

        let group_id = SharedString::from(format!("database-object-column-resize-{col_ix}"));

        h_flex()
            .id(("database-object-column-resize", col_ix))
            .group(group_id.clone())
            .occlude()
            .cursor_col_resize()
            .h_full()
            .w(DB_OBJECTS_COLUMN_RESIZE_HANDLE_WIDTH)
            .ml(-(DB_OBJECTS_COLUMN_RESIZE_HANDLE_WIDTH))
            .items_center()
            .justify_center()
            .child(self.render_column_resize_indicator(&group_id, cx))
            .on_mouse_down(
                MouseButton::Left,
                cx.listener(move |this, event: &MouseDownEvent, _window, cx| {
                    this.begin_column_resize(col_ix, event.position.x);
                    cx.stop_propagation();
                }),
            )
            .on_drag_move(cx.listener(
                move |this, event: &DragMoveEvent<DbObjectsResizeColumn>, _window, cx| {
                    this.drag_resize_column(col_ix, event, cx);
                },
            ))
            .on_drag(
                DbObjectsResizeColumn {
                    entity_id: cx.entity_id(),
                    col_ix,
                },
                |drag, _, _, cx| {
                    cx.stop_propagation();
                    cx.new(|_| drag.clone())
                },
            )
            .on_mouse_up(
                MouseButton::Left,
                cx.listener(|this, _event, _window, cx| {
                    this.finish_column_resize(cx);
                    cx.stop_propagation();
                }),
            )
            .on_mouse_up_out(
                MouseButton::Left,
                cx.listener(|this, _event, _window, cx| {
                    this.finish_column_resize(cx);
                }),
            )
            .into_any_element()
    }

    fn render_header(
        &self,
        columns: &[Column],
        show_row_number: bool,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let mut header = h_flex()
            .h(px(32.))
            .px_2()
            .items_center()
            .border_b_1()
            .border_color(cx.theme().border)
            .text_color(cx.theme().table_head_foreground)
            .bg(cx.theme().table_head);

        if show_row_number {
            header = header.child(
                div()
                    .w(DB_OBJECTS_ROW_NUMBER_WIDTH)
                    .px_2()
                    .text_sm()
                    .text_color(cx.theme().table_head_foreground)
                    .child(
                        div()
                            .size_full()
                            .flex()
                            .items_center()
                            .justify_end()
                            .child("#"),
                    ),
            );
        }

        for (col_ix, column) in columns.iter().enumerate() {
            header = header.child(
                div()
                    .w(column.width)
                    .h_full()
                    .flex()
                    .items_center()
                    .text_sm()
                    .text_color(cx.theme().table_head_foreground)
                    .child(
                        div()
                            .flex_1()
                            .min_w_0()
                            .h_full()
                            .px_2()
                            .flex()
                            .items_center()
                            .overflow_hidden()
                            .text_ellipsis()
                            .whitespace_nowrap()
                            .child(column.name.clone()),
                    )
                    .child(self.render_column_resize_handle(col_ix, column, cx)),
            );
        }

        header.into_any_element()
    }

    fn render_row(
        &self,
        row_ix: usize,
        row_values: &[String],
        columns: &[Column],
        show_row_number: bool,
        is_selected: bool,
        search_query: &str,
        db_node_type: DbNodeType,
        cx: &App,
    ) -> impl IntoElement {
        let mut row = h_flex()
            .h(px(44.))
            .px_2()
            .items_center()
            .when(is_selected, |el| el.bg(cx.theme().selection));

        if show_row_number {
            row = row.child(
                div()
                    .w(DB_OBJECTS_ROW_NUMBER_WIDTH)
                    .px_2()
                    .text_sm()
                    .text_color(cx.theme().muted_foreground)
                    .child((row_ix + 1).to_string()),
            );
        }

        for (col_ix, column) in columns.iter().enumerate() {
            let cell_value = row_values.get(col_ix).cloned().unwrap_or_default();
            let tooltip_text = cell_value.clone();
            let cell = if col_ix == 0 {
                let icon = get_icon_for_node_type(&db_node_type, cx.theme()).color();
                let label = if search_query.is_empty() {
                    Label::new(cell_value)
                } else {
                    Label::new(cell_value).highlights(search_query.to_string())
                };
                h_flex()
                    .gap_2()
                    .items_center()
                    .child(icon)
                    .child(label)
                    .into_any_element()
            } else {
                div().child(cell_value).into_any_element()
            };

            let cell_id = SharedString::from(format!("cell-{}-{}", row_ix, col_ix));
            row = row.child(
                div()
                    .id(cell_id)
                    .w(column.width)
                    .px_2()
                    .overflow_hidden()
                    .text_ellipsis()
                    .whitespace_nowrap()
                    .when(!tooltip_text.is_empty(), |el| {
                        el.tooltip(move |window, cx| {
                            Tooltip::new(tooltip_text.clone()).build(window, cx)
                        })
                    })
                    .child(cell),
            );
        }

        row
    }

    fn render_toolbar_buttons(
        &self,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Vec<AnyElement> {
        let mut buttons: Vec<AnyElement> = vec![];
        let current_node = self.current_node.clone();
        let data_db_node_type = self.db_node_type;
        let node_type = current_node
            .as_ref()
            .map(|n| n.node_type)
            .unwrap_or(DbNodeType::Connection);
        let database_type = current_node
            .as_ref()
            .map(|n| n.database_type)
            .unwrap_or(DatabaseType::MySQL);

        buttons.push({
            let node = current_node.clone();
            Button::new("refresh-data")
                .with_size(Size::Medium)
                .icon(IconName::Refresh)
                .tooltip(t!("Common.refresh"))
                .on_click(window.listener_for(&cx.entity(), move |_this, _, _, cx| {
                    if let Some(ref node) = node {
                        cx.emit(DatabaseObjectsEvent::Refresh { node: node.clone() });
                    }
                }))
                .into_any_element()
        });

        let toolbar_buttons =
            build_toolbar_buttons_for(database_type, node_type, data_db_node_type, cx);

        for btn_config in toolbar_buttons {
            let button = match btn_config.button_type {
                ToolbarButtonType::CurrentNode => {
                    let node = current_node.clone();
                    let event_fn = btn_config.event_fn;
                    Button::new(btn_config.id)
                        .with_size(Size::Medium)
                        .icon(btn_config.icon)
                        .tooltip(btn_config.tooltip)
                        .on_click(window.listener_for(&cx.entity(), move |_this, _, _, cx| {
                            if let Some(ref node) = node {
                                let event = event_fn(node.clone());
                                cx.emit(event);
                            }
                        }))
                        .into_any_element()
                }
                ToolbarButtonType::SelectedRow => {
                    let event_fn = btn_config.event_fn;
                    Button::new(btn_config.id)
                        .with_size(Size::Medium)
                        .icon(btn_config.icon)
                        .tooltip(btn_config.tooltip)
                        .on_click(
                            window.listener_for(&cx.entity(), move |this, _, window, cx| {
                                let nodes = this.build_nodes_for_selected_rows();
                                if nodes.is_empty() {
                                    window.push_notification(
                                        Notification::warning(t!("Common.select_row")),
                                        cx,
                                    );
                                    return;
                                }
                                if nodes.len() == 1 {
                                    let event = event_fn(nodes[0].clone());
                                    cx.emit(event);
                                    return;
                                }

                                let sample_event = event_fn(nodes[0].clone());
                                if let Some(action) = Self::batch_action_for_event(&sample_event) {
                                    cx.emit(DatabaseObjectsEvent::Batch { action, nodes });
                                    return;
                                }

                                if !Self::allow_multi_event(&sample_event) {
                                    window.push_notification(
                                        Notification::warning(
                                            t!("DatabaseObjects.batch_not_supported").to_string(),
                                        ),
                                        cx,
                                    );
                                    return;
                                }

                                for node in nodes {
                                    let event = event_fn(node);
                                    cx.emit(event);
                                }
                            }),
                        )
                        .into_any_element()
                }
            };
            buttons.push(button);
        }

        buttons
    }

    fn table_content_width(&self, columns: &[Column], show_row_number: bool) -> Pixels {
        let row_number_width = if show_row_number {
            DB_OBJECTS_ROW_NUMBER_WIDTH
        } else {
            px(0.0)
        };
        let columns_width = columns
            .iter()
            .fold(px(0.0), |acc, column| acc + column.width);
        row_number_width + columns_width + px(16.0)
    }
}

impl Render for DatabaseObjects {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let loaded_data = self.loaded_data.read(cx);
        let title = loaded_data.title.clone();
        let toolbar_buttons = self.render_toolbar_buttons(window, cx);
        let columns = self.columns.clone();
        let row_count = self.filtered_rows.len();
        let show_row_number = true;
        let search_query = self.search_query.clone();
        let header = self.render_header(&columns, show_row_number, cx);
        let list_columns = columns.clone();
        let list_search_query = search_query.clone();
        let table_width = self.table_content_width(&columns, show_row_number);
        let view = cx.entity();

        v_flex()
            .size_full()
            .child(
                h_flex()
                    .gap_1()
                    .items_center()
                    .px_2()
                    .py_1()
                    .border_b_1()
                    .border_color(cx.theme().border)
                    .bg(cx.theme().background)
                    .children(toolbar_buttons)
                    .child({
                        div().flex_1().min_w(px(220.0)).child(
                            Input::new(&self.search_input)
                                .prefix(
                                    Icon::new(IconName::Search)
                                        .text_color(cx.theme().muted_foreground),
                                )
                                .cleanable(true)
                                .small()
                                .w_full(),
                        )
                    })
                    .into_any_element(),
            )
            .child(
                div()
                    .size_full()
                    .overflow_x_scrollbar_masked()
                    .child(
                        v_flex()
                            .w(table_width)
                            .min_w(table_width)
                            .h_full()
                            .flex_shrink_0()
                            .gap_2()
                            .child(header)
                            .child(
                                div()
                                    .flex_1()
                                    .overflow_y_scrollbar()
                                    .on_scroll_wheel(cx.listener(Self::stop_vertical_scroll_bubble))
                                    .child(
                                        uniform_list("database-objects-list", row_count, {
                                            cx.processor(
                                                move |state: &mut Self,
                                                      range: Range<usize>,
                                                      _window,
                                                      cx| {
                                                    let db_node_type = state.db_node_type;
                                                    let show_row_number = true;
                                                    range
                                                        .map(|list_ix| {
                                                            let Some(original_row) = state
                                                                .filtered_rows
                                                                .get(list_ix)
                                                                .copied()
                                                            else {
                                                                return div()
                                                                    .id(list_ix)
                                                                    .into_any_element();
                                                            };
                                                            let Some(row_values) =
                                                                state.rows.get(original_row)
                                                            else {
                                                                return div()
                                                                    .id(list_ix)
                                                                    .into_any_element();
                                                            };

                                                            let is_selected = state
                                                                .selected_indices
                                                                .contains(&list_ix);
                                                            let row_ix = list_ix;
                                                            let row_view = view.clone();
                                                            div()
                                                                .id(list_ix)
                                                                .cursor_pointer()
                                                                .on_mouse_down(
                                                                    MouseButton::Left,
                                                                    cx.listener(
                                                                        move |this,
                                                                              event: &MouseDownEvent,
                                                                              _window,
                                                                              cx| {
                                                                            let multi_select =
                                                                                event.modifiers.secondary();
                                                                            this.toggle_selection(
                                                                                row_ix,
                                                                                multi_select,
                                                                            );
                                                                            this.refresh_ddl_preview_for_selection(cx);
                                                                            cx.notify();
                                                                        },
                                                                    ),
                                                                )
                                                                .on_double_click(cx.listener(
                                                                    move |this, _, _window, cx| {
                                                                        this.handle_row_double_click(row_ix, cx);
                                                                    },
                                                                ))
                                                                .context_menu(
                                                                    move |menu, window, cx| {
                                                                        Self::build_context_menu_for_row(
                                                                            menu,
                                                                            row_ix,
                                                                            &row_view,
                                                                            window,
                                                                            cx,
                                                                        )
                                                                    },
                                                                )
                                                                .child(state.render_row(
                                                                    row_ix,
                                                                    row_values,
                                                                    &list_columns,
                                                                    show_row_number,
                                                                    is_selected,
                                                                    &list_search_query,
                                                                    db_node_type,
                                                                    cx,
                                                                ))
                                                                .into_any_element()
                                                        })
                                                        .collect()
                                                },
                                            )
                                        })
                                        .flex_grow()
                                        .size_full()
                                        .with_sizing_behavior(ListSizingBehavior::Auto),
                                    ),
                            ),
                    ),
            )
            .child(div().p_2().text_sm().child(title))
    }
}

impl Clone for DatabaseObjects {
    fn clone(&self) -> Self {
        Self {
            loaded_data: self.loaded_data.clone(),
            columns: self.columns.clone(),
            rows: self.rows.clone(),
            filtered_rows: self.filtered_rows.clone(),
            db_node_type: self.db_node_type,
            focus_handle: self.focus_handle.clone(),
            workspace: self.workspace.clone(),
            search_input: self.search_input.clone(),
            search_seq: self.search_seq,
            search_query: self.search_query.clone(),
            search_debouncer: self.search_debouncer.clone(),
            current_node: self.current_node.clone(),
            selected_indices: self.selected_indices.clone(),
            ddl_preview_content: self.ddl_preview_content.clone(),
            ddl_preview_loading: self.ddl_preview_loading,
            ddl_preview_table_name: self.ddl_preview_table_name.clone(),
            ddl_preview_request_seq: self.ddl_preview_request_seq,
            resizing_column: self.resizing_column,
            _subscriptions: vec![],
        }
    }
}

impl EventEmitter<DatabaseObjectsEvent> for DatabaseObjects {}

impl DatabaseObjects {
    pub fn ddl_preview_content(&self) -> &SharedString {
        &self.ddl_preview_content
    }

    pub fn ddl_preview_table_name(&self) -> Option<&str> {
        self.ddl_preview_table_name.as_deref()
    }

    pub fn ddl_preview_loading(&self) -> bool {
        self.ddl_preview_loading
    }
}

impl Focusable for DatabaseObjects {
    fn focus_handle(&self, _cx: &App) -> FocusHandle {
        self.focus_handle.clone()
    }
}

pub struct DatabaseObjectsPanel {
    database_objects: Entity<DatabaseObjects>,
}

impl DatabaseObjectsPanel {
    pub fn new(workspace: Option<Workspace>, window: &mut Window, cx: &mut Context<Self>) -> Self {
        let database_objects = cx.new(|cx| DatabaseObjects::new(workspace, window, cx));
        Self { database_objects }
    }

    pub fn database_objects(&self) -> &Entity<DatabaseObjects> {
        &self.database_objects
    }

    pub fn handle_node_selected(&self, node: DbNode, config: DbConnectionConfig, cx: &mut App) {
        self.database_objects.update(cx, |database_objects, cx| {
            database_objects.handle_node_selected(node, config, cx);
        })
    }

    pub fn refresh(&self, global_state: GlobalDbState, cx: &mut App) {
        self.database_objects.update(cx, |database_objects, cx| {
            if let Some(node) = database_objects.current_node.clone() {
                let connection_id = node.connection_id.clone();
                if let Some(config) = global_state.get_config(&connection_id) {
                    database_objects.handle_node_selected(node, config, cx);
                }
            }
        });
    }
}

impl EventEmitter<TabContentEvent> for DatabaseObjectsPanel {}

impl Render for DatabaseObjectsPanel {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        self.database_objects.clone()
    }
}

impl Focusable for DatabaseObjectsPanel {
    fn focus_handle(&self, cx: &App) -> FocusHandle {
        self.database_objects.focus_handle(cx)
    }
}

impl TabContent for DatabaseObjectsPanel {
    fn content_key(&self) -> &'static str {
        "DatabaseObjects"
    }

    fn title(&self, _cx: &App) -> SharedString {
        SharedString::from(t!("DatabaseObjects.title"))
    }

    fn closeable(&self, _cx: &App) -> bool {
        false
    }

    fn width_size(&self, _cx: &App) -> Option<Size> {
        Some(Size::XSmall)
    }
}

impl Clone for DatabaseObjectsPanel {
    fn clone(&self) -> Self {
        Self {
            database_objects: self.database_objects.clone(),
        }
    }
}
