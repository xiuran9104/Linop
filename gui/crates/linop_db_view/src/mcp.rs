//! Public MCP Database 工具集接线:db.* → linop-dbx 共享运行时。
//!
//! 工具契约见 public_mcp::tools::database;这里按契约成形 JSON:
//! 经由 `DbxRuntime::shared()`(via 跳板隧道在首次初始化时建立)。

use std::sync::Arc;

use gpui::App;
use public_mcp::tools::{DatabaseRuntimeProvider, database_tool_registry};
use serde_json::json;
use tool_runtime::{ToolError, ToolResult};

use crate::DbxRuntime;

struct DbxToolRuntime;

impl DatabaseRuntimeProvider for DbxToolRuntime {
    fn list_connections(&self) -> tool_runtime::ToolFuture {
        Box::pin(async move {
            let runtime = DbxRuntime::shared().await.map_err(failed)?;
            let mut connections = runtime.0.connections().await;
            connections.sort_by(|left, right| left.name.cmp(&right.name));
            Ok(ToolResult::structured(json!({
                "connections": connections
                    .into_iter()
                    .map(|c| json!({
                        "connection_id": c.name,
                        "driver": c.driver,
                        "database": c.database,
                    }))
                    .collect::<Vec<_>>()
            })))
        })
    }

    fn list_tables(&self, connection: &str) -> tool_runtime::ToolFuture {
        let connection = connection.to_string();
        Box::pin(async move {
            let runtime = DbxRuntime::shared().await.map_err(failed)?;
            let tables = runtime.0.list_tables(&connection).await.map_err(failed)?;
            Ok(ToolResult::structured(json!({
                "tables": tables
                    .into_iter()
                    .map(|t| json!({
                        "name": t.name,
                        "table_type": t.table_type,
                    }))
                    .collect::<Vec<_>>()
            })))
        })
    }

    fn query(&self, connection: &str, sql: &str) -> tool_runtime::ToolFuture {
        let connection = connection.to_string();
        let sql = sql.to_string();
        Box::pin(async move {
            let runtime = DbxRuntime::shared().await.map_err(failed)?;
            let result = runtime.0.query(&connection, &sql).await.map_err(failed)?;
            Ok(ToolResult::structured(json!({
                "columns": result.columns,
                "rows": result.rows,
                "truncated": result.truncated,
            })))
        })
    }
}

fn failed(error: impl std::fmt::Display) -> ToolError {
    ToolError::Failed { message: error.to_string() }
}

/// 注册 Database 工具集(DbxDomain::register 调用一次)。
pub fn register_mcp_toolsets() {
    mcp_host::register_toolset(
        mcp_host::McpToolset::Database,
        Arc::new(|cx: &App| {
            // 预热共享运行时:与工具调用同一 Tokio runtime,via 隧道提前建立。
            one_core::gpui_tokio::Tokio::spawn(cx, async {
                let _ = DbxRuntime::shared().await;
            })
            .detach();
            vec![mcp_host::runtime_provider(database_tool_registry(Arc::new(
                DbxToolRuntime,
            )))]
        }),
    );
}
