//! 数据库工具集(db.*):连接发现、表清单、SQL 查询。
//!
//! 执行归 Linop 的 linop-dbx 运行时(DBX 驱动执行层,via 跳板隧道);
//! 本模块只定义工具契约(provider trait 全异步,实现侧负责 JSON 成形):

use serde_json::{Value, json};
use std::sync::Arc;
use tool_runtime::{
    ResourceKind, RiskLevel, ToolAdapter, ToolAnnotations, ToolContext, ToolDescriptor, ToolError,
    ToolHandler, ToolMode, ToolResult, ToolTargetSpec,
};

const DB_LIST_CONNECTIONS_TOOL: &str = "db.list_connections";
const DB_LIST_TABLES_TOOL: &str = "db.list_tables";
const DB_QUERY_TOOL: &str = "db.query";

/// 数据库工具的运行时契约(实现侧:linop_db_view 的 mcp.rs,桥接 linop-dbx)。
///
/// 返回值 JSON 契约(实现侧负责成形):
/// - `list_connections`:`{"connections":[{"connection_id","driver","database"}]}`
/// - `list_tables`:`{"tables":[{"name","table_type"}]}`
/// - `query`:`{"columns":[..],"rows":[[..]],"truncated":bool}`
pub trait DatabaseRuntimeProvider: Send + Sync + 'static {
    fn list_connections(&self) -> tool_runtime::ToolFuture;
    fn list_tables(&self, connection: &str) -> tool_runtime::ToolFuture;
    fn query(&self, connection: &str, sql: &str) -> tool_runtime::ToolFuture;
}

/// 以给定运行时构造 db.* 工具集注册表。
pub fn database_tool_registry(runtime: Arc<dyn DatabaseRuntimeProvider>) -> tool_runtime::ToolRegistry {
    tool_runtime::ToolRegistry::new(DatabaseToolProvider::handlers(runtime))
}

#[derive(Clone)]
pub struct DatabaseToolProvider {
    runtime: Arc<dyn DatabaseRuntimeProvider>,
    tool: DatabaseTool,
}

#[derive(Clone, Copy)]
enum DatabaseTool {
    ListConnections,
    ListTables,
    Query,
}

impl DatabaseToolProvider {
    pub fn handlers(runtime: Arc<dyn DatabaseRuntimeProvider>) -> Vec<Arc<dyn ToolHandler>> {
        vec![
            Arc::new(Self { runtime: runtime.clone(), tool: DatabaseTool::ListConnections }),
            Arc::new(Self { runtime: runtime.clone(), tool: DatabaseTool::ListTables }),
            Arc::new(Self { runtime, tool: DatabaseTool::Query }),
        ]
    }

    pub fn empty() -> Vec<Arc<dyn ToolHandler>> {
        Self::handlers(Arc::new(EmptyDatabaseRuntime))
    }
}

impl ToolHandler for DatabaseToolProvider {
    fn descriptor(&self) -> ToolDescriptor {
        ToolDescriptor {
            id: self.tool.id().to_string(),
            title: self.tool.title().to_string(),
            description: self.tool.description().to_string(),
            input_schema: self.tool.input_schema(),
            output_schema: json!({ "type": "object" }),
            permissions: Vec::new(),
            mode: ToolMode::Deterministic,
            adapters: vec![ToolAdapter::Mcp, ToolAdapter::FunctionCalling],
            annotations: self.tool.annotations(),
        }
    }

    fn call(&self, input: Value, _context: ToolContext) -> tool_runtime::ToolFuture {
        let handler = self.clone();
        Box::pin(async move {
            match handler.tool {
                DatabaseTool::ListConnections => handler.runtime.list_connections().await,
                DatabaseTool::ListTables => {
                    let connection = required_string(&input, "connection")?;
                    handler.runtime.list_tables(&connection).await
                }
                DatabaseTool::Query => {
                    let connection = required_string(&input, "connection")?;
                    let sql = required_string(&input, "sql")?;
                    handler.runtime.query(&connection, &sql).await
                }
            }
        })
    }

    fn target_spec(&self) -> ToolTargetSpec {
        if matches!(self.tool, DatabaseTool::ListConnections) {
            return ToolTargetSpec::none();
        }
        ToolTargetSpec::required(vec![
            ResourceKind::Postgres,
            ResourceKind::Mysql,
            ResourceKind::Sqlite,
            ResourceKind::Redis,
            ResourceKind::Mongo,
            // 已编译其余驱动经 resource_pool 映射为 Other(<dbType>)。
            ResourceKind::Other("sqlserver".into()),
            ResourceKind::Other("elasticsearch".into()),
            ResourceKind::Other("clickhouse".into()),
            ResourceKind::Other("doris".into()),
            ResourceKind::Other("starrocks".into()),
            ResourceKind::Other("manticoresearch".into()),
            ResourceKind::Other("questdb".into()),
            ResourceKind::Other("influxdb".into()),
            ResourceKind::Other("influxdb3".into()),
            ResourceKind::Other("victoriametrics".into()),
            ResourceKind::Other("solr".into()),
            ResourceKind::Other("meilisearch".into()),
            ResourceKind::Other("easysearch".into()),
            ResourceKind::Other("couchdb".into()),
            ResourceKind::Other("hbase".into()),
            ResourceKind::Other("rqlite".into()),
            ResourceKind::Other("turso".into()),
            ResourceKind::Other("cloudflare-d1".into()),
            ResourceKind::Other("dynamodb".into()),
        ])
    }
}

impl DatabaseTool {
    fn id(self) -> &'static str {
        match self {
            DatabaseTool::ListConnections => DB_LIST_CONNECTIONS_TOOL,
            DatabaseTool::ListTables => DB_LIST_TABLES_TOOL,
            DatabaseTool::Query => DB_QUERY_TOOL,
        }
    }

    fn title(self) -> &'static str {
        match self {
            DatabaseTool::ListConnections => "List database connections",
            DatabaseTool::ListTables => "List database tables",
            DatabaseTool::Query => "Execute SQL",
        }
    }

    fn description(self) -> &'static str {
        match self {
            DatabaseTool::ListConnections => {
                "List database connections loaded by the running Linop app (databases.toml, driver postgres/mysql/sqlite/redis; connections with a via jump host go through SSH tunnels). Returns connection_id values for db.list_tables and db.query. This does not list saved connection profiles; use connections.list for that."
            }
            DatabaseTool::ListTables => {
                "List tables and views of one database connection in the running Linop app. Use db.list_connections first to discover connection_id. Returns name and table_type for each object."
            }
            DatabaseTool::Query => {
                "Execute one SQL statement against a database connection in the running Linop app (postgres/mysql/sqlite drivers). Use db.list_connections first to discover connection_id, and db.list_tables to discover table names. Returns columns, rows and a truncated flag. The statement may mutate data and therefore requires write approval."
            }
        }
    }

    fn input_schema(self) -> Value {
        match self {
            DatabaseTool::ListConnections => json!({
                "type": "object",
                "properties": {}
            }),
            DatabaseTool::ListTables => json!({
                "type": "object",
                "properties": {
                    "connection": connection_property()
                },
                "required": ["connection"]
            }),
            DatabaseTool::Query => json!({
                "type": "object",
                "properties": {
                    "connection": connection_property(),
                    "sql": {
                        "type": "string",
                        "description": "Single SQL statement, for example `SELECT * FROM users LIMIT 10`."
                    }
                },
                "required": ["connection", "sql"]
            }),
        }
    }

    fn annotations(self) -> ToolAnnotations {
        match self {
            DatabaseTool::ListConnections => {
                ToolAnnotations::read_only(self.title()).with_risk(RiskLevel::Low)
            }
            DatabaseTool::ListTables => {
                ToolAnnotations::read_only(self.title()).with_risk(RiskLevel::Low)
            }
            DatabaseTool::Query => ToolAnnotations::mutating(self.title()),
        }
    }
}

fn connection_property() -> Value {
    json!({
        "type": "string",
        "description": "Database connection id from db.list_connections."
    })
}

fn required_string(input: &Value, field: &'static str) -> Result<String, ToolError> {
    input
        .get(field)
        .and_then(Value::as_str)
        .filter(|value| !value.trim().is_empty())
        .map(str::to_string)
        .ok_or_else(|| ToolError::Failed {
            message: format!("missing required string field `{field}`"),
        })
}

struct EmptyDatabaseRuntime;

impl DatabaseRuntimeProvider for EmptyDatabaseRuntime {
    fn list_connections(&self) -> tool_runtime::ToolFuture {
        Box::pin(async {
            Ok(ToolResult::structured(
                json!({ "connections": [] }),
            ))
        })
    }

    fn list_tables(&self, connection: &str) -> tool_runtime::ToolFuture {
        let connection = connection.to_string();
        Box::pin(async move {
            Err(ToolError::Failed {
                message: format!("unknown database connection: {connection}"),
            })
        })
    }

    fn query(&self, connection: &str, _sql: &str) -> tool_runtime::ToolFuture {
        let connection = connection.to_string();
        Box::pin(async move {
            Err(ToolError::Failed {
                message: format!("unknown database connection: {connection}"),
            })
        })
    }
}
