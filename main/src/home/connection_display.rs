use std::collections::HashSet;

use gpui::Styled as _;
use gpui_component::{Icon, IconName};
use one_core::storage::{
    ConnectionType, DatabaseType, DbConnectionConfig, MongoDBParams, RedisMode, RedisParams,
    SerialParams, StoredConnection,
};

pub(crate) fn connection_subtitle(conn: &StoredConnection) -> Option<String> {
    match conn.connection_type {
        ConnectionType::Database => conn.to_db_connection().ok().map(database_subtitle),
        ConnectionType::SshSftp => conn
            .to_ssh_params()
            .ok()
            .map(|params| format!("{}@{}:{}", params.username, params.host, params.port)),
        ConnectionType::Redis => conn.to_redis_params().ok().map(redis_subtitle),
        ConnectionType::MongoDB => conn.to_mongodb_params().ok().map(mongodb_subtitle),
        ConnectionType::Serial => conn.to_serial_params().ok().map(serial_subtitle),
        _ => None,
    }
}

fn database_subtitle(params: DbConnectionConfig) -> String {
    if matches!(
        params.database_type,
        DatabaseType::SQLite | DatabaseType::DuckDB
    ) {
        return params.host;
    }

    let database = params
        .database
        .map(|database| format!("/{database}"))
        .unwrap_or_default();
    format!(
        "{}@{}:{}{}",
        params.username, params.host, params.port, database
    )
}

fn redis_subtitle(params: RedisParams) -> String {
    match params.mode {
        RedisMode::Standalone => format!("{}:{}/{}", params.host, params.port, params.db_index),
        RedisMode::Sentinel => {
            let (master_name, sentinel_count) = params
                .sentinel
                .as_ref()
                .map(|sentinel| (sentinel.master_name.as_str(), sentinel.sentinels.len()))
                .unwrap_or(("sentinel", 0));
            format!("{master_name} (sentinel:{sentinel_count})")
        }
        RedisMode::Cluster => {
            let node_count = params
                .cluster
                .as_ref()
                .map(|cluster| cluster.nodes.len())
                .unwrap_or(0);
            format!("cluster ({node_count} nodes)")
        }
    }
}

fn mongodb_subtitle(params: MongoDBParams) -> String {
    if !params.host.is_empty() {
        return params
            .port
            .map(|port| format!("{}:{port}", params.host))
            .unwrap_or(params.host);
    }

    if !params.connection_string.is_empty() {
        return params.connection_string;
    }

    "MongoDB".to_string()
}

fn serial_subtitle(params: SerialParams) -> String {
    let parity = match params.parity {
        one_core::storage::models::SerialParity::None => 'N',
        one_core::storage::models::SerialParity::Odd => 'O',
        one_core::storage::models::SerialParity::Even => 'E',
    };
    format!(
        "{} ({}, {}{}{})",
        params.port_name, params.baud_rate, params.data_bits, parity, params.stop_bits
    )
}

pub(crate) fn connection_icon(conn: &StoredConnection) -> Icon {
    match conn.connection_type {
        ConnectionType::Database => conn
            .to_db_connection()
            .map(|params| params.database_type.as_icon())
            .unwrap_or_else(|_| IconName::Database.color())
            .text_color(gpui::white()),
        ConnectionType::SshSftp => IconName::TerminalColor
            .color()
            .text_color(gpui::rgb(0x8b5cf6)),
        ConnectionType::Redis => IconName::Redis.color().text_color(gpui::white()),
        ConnectionType::MongoDB => IconName::MongoDB.color().text_color(gpui::white()),
        ConnectionType::Serial => IconName::SerialPort.color().text_color(gpui::white()),
        _ => IconName::Server.color().text_color(gpui::white()),
    }
}

pub(crate) fn has_team_badge(conn: &StoredConnection) -> bool {
    conn.team_id.is_some()
}

pub(crate) fn generate_duplicate_name(
    original_name: &str,
    existing_names: &HashSet<String>,
) -> String {
    let base_name = format!("{original_name} (副本)");

    if !existing_names.contains(&base_name) {
        return base_name;
    }

    for i in 2..100 {
        let name = format!("{original_name} (副本 {i})");
        if !existing_names.contains(&name) {
            return name;
        }
    }

    base_name
}

#[cfg(test)]
mod tests {
    use std::collections::HashSet;

    use one_core::storage::{ConnectionType, StoredConnection};

    use super::*;

    fn stored(connection_type: ConnectionType, params: serde_json::Value) -> StoredConnection {
        StoredConnection {
            id: Some(1),
            name: "Sample".to_string(),
            connection_type,
            params: params.to_string(),
            sort_order: 0,
            workspace_id: None,
            selected_databases: None,
            remark: None,
            sync_enabled: true,
            cloud_id: None,
            last_synced_at: None,
            created_at: None,
            updated_at: None,
            team_id: None,
            owner_id: None,
        }
    }

    #[test]
    fn database_subtitle_includes_database_path() {
        let conn = stored(
            ConnectionType::Database,
            serde_json::json!({
                "database_type": "PostgreSQL",
                "host": "db.local",
                "port": 5432,
                "username": "alice",
                "password": "",
                "database": "app",
                "service_name": null,
                "sid": null,
                "extra_params": {}
            }),
        );

        assert_eq!(
            Some("alice@db.local:5432/app".to_string()),
            connection_subtitle(&conn)
        );
    }

    #[test]
    fn ssh_subtitle_uses_user_host_port() {
        let conn = stored(
            ConnectionType::SshSftp,
            serde_json::json!({
                "host": "box.local",
                "port": 22,
                "username": "deploy",
                "auth_method": "Agent",
                "connect_timeout": null,
                "keepalive_interval": null,
                "keepalive_max": null,
                "default_directory": null,
                "init_script": null,
                "jump_server": null,
                "proxy": null
            }),
        );

        assert_eq!(
            Some("deploy@box.local:22".to_string()),
            connection_subtitle(&conn)
        );
    }

    #[test]
    fn duplicate_name_uses_incrementing_suffix() {
        let existing: HashSet<String> = ["Prod (副本)", "Prod (副本 2)"]
            .into_iter()
            .map(String::from)
            .collect();

        assert_eq!("Prod (副本 3)", generate_duplicate_name("Prod", &existing));
    }
}
