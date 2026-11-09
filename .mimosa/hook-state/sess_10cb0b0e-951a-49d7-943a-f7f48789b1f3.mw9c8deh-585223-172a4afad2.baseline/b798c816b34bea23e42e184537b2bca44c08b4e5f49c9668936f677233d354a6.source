//! db.* 工具契约测试:stub provider + 注册表行为(照 redis_tools.rs 模式)。

use public_mcp::permissions::PermissionMode;
use public_mcp::tools::{
    DatabaseRuntimeProvider, DatabaseToolProvider, PublicMcpToolContext, PublicMcpToolRegistry,
    ToolRuntimeMcpProvider,
};
use serde_json::{Value, json};
use std::sync::Arc;
use tool_runtime::{ResourceKind, ResourcePool, ResourceRef, ToolRegistry};

#[tokio::test]
async fn database_provider_lists_connections() {
    let registry = db_registry(FakeDbRuntime::default());

    let result = registry
        .call_tool("db.list_connections", None, tool_context())
        .await
        .expect("db.list_connections should run");

    // 契约:输出顺序由实现侧决定(stub 原样;真实实现按 connection_id 排序)。
    assert_eq!(
        Some(json!({
            "connections": [
                { "connection_id": "pg-b", "driver": "postgres", "database": Value::Null },
                { "connection_id": "pg-a", "driver": "postgres", "database": "app" }
            ]
        })),
        result.structured_content
    );
}

#[tokio::test]
async fn database_provider_lists_tables() {
    let registry = db_registry(FakeDbRuntime::default());

    let result = registry
        .call_tool(
            "db.list_tables",
            Some(serde_json::Map::from_iter([(
                "target".to_string(),
                json!("pg-a"),
            )])),
            tool_context(),
        )
        .await
        .expect("db.list_tables should run");

    assert_eq!(
        Some(json!({
            "tables": [
                { "name": "orders", "table_type": "TABLE" },
                { "name": "orders_view", "table_type": "VIEW" }
            ]
        })),
        result.structured_content
    );
}

#[tokio::test]
async fn database_provider_runs_query() {
    let registry = db_registry(FakeDbRuntime::default());

    let result = registry
        .call_tool(
            "db.query",
            Some(serde_json::Map::from_iter([
                ("target".to_string(), json!("pg-a")),
                ("sql".to_string(), json!("SELECT 1")),
            ])),
            tool_context(),
        )
        .await
        .expect("db.query should run");

    assert_eq!(
        Some(json!({
            "columns": ["id", "val"],
            "rows": [[1, "via-ok"]],
            "truncated": false
        })),
        result.structured_content
    );
}

#[tokio::test]
async fn database_query_rejects_unknown_connection() {
    let registry = db_registry(FakeDbRuntime::default());

    let error = registry
        .call_tool(
            "db.query",
            Some(serde_json::Map::from_iter([
                ("target".to_string(), json!("nope")),
                ("sql".to_string(), json!("SELECT 1")),
            ])),
            tool_context(),
        )
        .await
        .expect_err("unknown connection should fail");

    assert!(error.to_string().contains("unknown database connection: nope"));
}

#[tokio::test]
async fn database_query_requires_sql_field() {
    let registry = db_registry(FakeDbRuntime::default());

    let error = registry
        .call_tool(
            "db.query",
            Some(serde_json::Map::from_iter([(
                "target".to_string(),
                json!("pg-a"),
            )])),
            tool_context(),
        )
        .await
        .expect_err("missing sql field should fail");

    assert!(error.to_string().contains("missing required string field `sql`"));
}

#[test]
fn database_query_is_registered_as_mutating() {
    let registry = ToolRegistry::new(DatabaseToolProvider::empty());
    let tool = registry
        .get("db.query", tool_runtime::ToolAdapter::Mcp)
        .expect("db.query tool should be registered");

    assert_eq!(json!(["connection", "sql"]), tool.input_schema["required"]);
    assert_eq!("db.query", tool.id);
    assert!(!tool.annotations.read_only);
    assert!(tool.annotations.destructive);
}

#[tokio::test]
async fn database_target_resolves_with_database_resource_kind() {
    let runtime_registry =
        ToolRegistry::new(DatabaseToolProvider::handlers(Arc::new(FakeDbRuntime::default())));
    let resource_pool = ResourcePool::new()
        .with_resource(ResourceRef::new("redis-a", ResourceKind::Redis, "cache"))
        .with_resource(ResourceRef::new("pg-a", ResourceKind::Postgres, "db"));
    let registry = PublicMcpToolRegistry::new(vec![Arc::new(
        ToolRuntimeMcpProvider::new(runtime_registry).with_resource_pool(resource_pool),
    )]);

    let result = registry
        .call_tool(
            "db.list_tables",
            Some(serde_json::Map::from_iter([(
                "target".to_string(),
                json!("db"),
            )])),
            tool_context(),
        )
        .await
        .expect("target label should disambiguate to the postgres resource");

    assert_eq!(
        Some(json!({
            "tables": [
                { "name": "orders", "table_type": "TABLE" },
                { "name": "orders_view", "table_type": "VIEW" }
            ]
        })),
        result.structured_content
    );
}

fn db_registry(runtime: FakeDbRuntime) -> PublicMcpToolRegistry {
    PublicMcpToolRegistry::new(vec![Arc::new(ToolRuntimeMcpProvider::new(
        ToolRegistry::new(DatabaseToolProvider::handlers(Arc::new(runtime))),
    ))])
}

fn tool_context() -> PublicMcpToolContext {
    PublicMcpToolContext {
        permission_mode: PermissionMode::Allow,
        approver: Default::default(),
    }
}

/// 契约参考实现:按 database.rs 文档的 JSON 形状返回固定数据。
struct FakeDbRuntime;

impl Default for FakeDbRuntime {
    fn default() -> Self {
        Self
    }
}

impl DatabaseRuntimeProvider for FakeDbRuntime {
    fn list_connections(&self) -> tool_runtime::ToolFuture {
        Box::pin(async {
            Ok(tool_runtime::ToolResult::structured(json!({
                "connections": [
                    { "connection_id": "pg-b", "driver": "postgres", "database": Value::Null },
                    { "connection_id": "pg-a", "driver": "postgres", "database": "app" }
                ]
            })))
        })
    }

    fn list_tables(&self, connection: &str) -> tool_runtime::ToolFuture {
        let connection = connection.to_string();
        Box::pin(async move {
            if connection != "pg-a" {
                return Err(tool_runtime::ToolError::Failed {
                    message: format!("unknown database connection: {connection}"),
                });
            }
            Ok(tool_runtime::ToolResult::structured(json!({
                "tables": [
                    { "name": "orders", "table_type": "TABLE" },
                    { "name": "orders_view", "table_type": "VIEW" }
                ]
            })))
        })
    }

    fn query(&self, connection: &str, _sql: &str) -> tool_runtime::ToolFuture {
        let connection = connection.to_string();
        Box::pin(async move {
            if connection != "pg-a" {
                return Err(tool_runtime::ToolError::Failed {
                    message: format!("unknown database connection: {connection}"),
                });
            }
            Ok(tool_runtime::ToolResult::structured(json!({
                "columns": ["id", "val"],
                "rows": [[1, "via-ok"]],
                "truncated": false
            })))
        })
    }
}
