use super::agent_runtime_tool_registry;
use agent_runtime::{ResourceContext, RiskLevel, ToolName};
use gpui::TestAppContext;
use one_core::settings::{AppSettings, ToolExposureToolsetSettings};
use one_core::storage::connection::SqliteConnection;
use one_core::storage::migration::run_migrations;
use one_core::storage::{
    ConnectionRepository, GlobalStorageState, StorageManager, WorkspaceRepository,
};

#[gpui::test]
fn agent_runtime_tool_registry_uses_native_database_tools(cx: &mut TestAppContext) {
    let registry = cx.update(|cx| {
        register_connection_repository(cx);
        let mut settings = AppSettings::default();
        settings.tool_exposure.agent = ToolExposureToolsetSettings {
            terminal: false,
            connections: false,
            database: true,
            ..Default::default()
        };
        cx.set_global(settings);

        agent_runtime_tool_registry(cx).expect("agent registry should build")
    });
    let names = registry
        .names()
        .into_iter()
        .map(|name| name.to_string())
        .collect::<Vec<_>>();

    // dbx 支持的 db.* 工具集(经共享 linop-dbx 运行时)。
    assert!(names.contains(&"db_list_connections".to_string()));
    assert!(names.contains(&"db_list_tables".to_string()));
    assert!(names.contains(&"db_query".to_string()));

    let db_query = registry
        .get(&ToolName::new("db.query"))
        .expect("runtime-backed db.query should be registered");
    let spec = db_query.spec(&ResourceContext::new());
    // SQL 可能变更数据:标注 mutating(高风险),走审批门。
    assert_eq!(RiskLevel::High, spec.risk);
    assert!(
        spec.description.contains("Execute one SQL statement"),
        "db_query should be backed by tool_runtime db.query descriptor"
    );
    assert_agent_target_schema(&registry, "db.query", serde_json::json!(["target", "sql"]));
    assert_tool_risk(&registry, "db.list_connections", RiskLevel::Read);
    assert_tool_risk(&registry, "db.list_tables", RiskLevel::Read);
}

#[gpui::test]
fn agent_runtime_tool_registry_ignores_public_mcp_toolset_exposure(cx: &mut TestAppContext) {
    let registry = cx.update(|cx| {
        register_connection_repository(cx);
        let mut settings = AppSettings::default();
        settings.tool_exposure.mcp = ToolExposureToolsetSettings {
            terminal: false,
            terminal_ssh_exec: false,
            terminal_exec: false,
            connections: false,
            sftp: false,
            database: false,
            redis: false,
            mongo: false,
            internal_functions: false,
        };
        cx.set_global(settings);

        agent_runtime_tool_registry(cx).expect("agent registry should build")
    });
    let names = registry
        .names()
        .into_iter()
        .map(|name| name.to_string())
        .collect::<Vec<_>>();

    // agent 默认工具集与 public MCP 开关无关;redis/mongo 随旧数据域删除
    // (Redis 经 db.* driver=redis 承载)。
    assert!(names.contains(&"connections_save".to_string()));
    assert!(names.contains(&"internal_functions_call".to_string()));
    assert!(names.contains(&"db_query".to_string()));
    assert!(names.contains(&"db_list_connections".to_string()));
    assert!(names.contains(&"sftp_read".to_string()));
}

#[gpui::test]
fn agent_runtime_tool_registry_respects_agent_tool_exposure(cx: &mut TestAppContext) {
    let registry = cx.update(|cx| {
        register_connection_repository(cx);
        let mut settings = AppSettings::default();
        settings.tool_exposure.agent = ToolExposureToolsetSettings {
            terminal: false,
            connections: false,
            sftp: false,
            database: false,
            redis: false,
            mongo: false,
            internal_functions: false,
            ..Default::default()
        };
        cx.set_global(settings);

        agent_runtime_tool_registry(cx).expect("agent registry should build")
    });
    let names = registry
        .names()
        .into_iter()
        .map(|name| name.to_string())
        .collect::<Vec<_>>();

    assert!(!names.contains(&"connections_save".to_string()));
    assert!(!names.contains(&"internal_functions_call".to_string()));
    assert!(!names.contains(&"db_query".to_string()));
    assert!(!names.contains(&"redis_get".to_string()));
    assert!(!names.contains(&"mongo_find".to_string()));
    assert!(!names.contains(&"sftp_read".to_string()));
}

// 注:redis/mongo 独立工具集随旧数据域删除;Redis 访问经 db.*(driver=redis)承载。

#[gpui::test]
fn agent_runtime_tool_registry_uses_native_ssh_sftp_tools(cx: &mut TestAppContext) {
    let registry = cx.update(|cx| {
        register_connection_repository(cx);
        let mut settings = AppSettings::default();
        settings.tool_exposure.agent = ToolExposureToolsetSettings {
            terminal: false,
            connections: false,
            sftp: true,
            ..Default::default()
        };
        cx.set_global(settings);

        agent_runtime_tool_registry(cx).expect("agent registry should build")
    });
    let names = registry
        .names()
        .into_iter()
        .map(|name| name.to_string())
        .collect::<Vec<_>>();

    assert!(names.contains(&"sftp_list".to_string()));
    assert!(names.contains(&"sftp_read".to_string()));
    assert!(names.contains(&"sftp_write".to_string()));
    assert!(names.contains(&"sftp_stat".to_string()));
    assert!(names.contains(&"sftp_upload".to_string()));
    assert!(names.contains(&"sftp_download".to_string()));
    assert!(!names.contains(&"ssh_list_dir".to_string()));
    assert!(!names.contains(&"ssh_read_file".to_string()));
    assert!(!names.contains(&"ssh_file_stat".to_string()));
    assert!(!names.contains(&"ssh_write_file".to_string()));
    assert!(!names.contains(&"sftp.write".to_string()));
    assert_agent_target_schema(&registry, "sftp.list", serde_json::json!(["target"]));
    assert_tool_risk(&registry, "sftp.list", RiskLevel::Read);
    assert_tool_risk(&registry, "sftp.read", RiskLevel::Read);
    assert_tool_risk(&registry, "sftp.write", RiskLevel::High);
    assert_tool_risk(&registry, "sftp.stat", RiskLevel::Read);
    assert_tool_risk(&registry, "sftp.upload", RiskLevel::High);
    assert_tool_risk(&registry, "sftp.download", RiskLevel::High);
}

fn register_connection_repository(cx: &mut gpui::App) {
    let storage = StorageManager::new_with_connection(test_connection());
    storage.register(ConnectionRepository::new(storage.connection()));
    storage.register(WorkspaceRepository::new(storage.connection()));
    cx.set_global(GlobalStorageState { storage });
}

/// 所有暴露给模型的工具 schema 必须能被 function-calling 接受。
///
/// `connections.save` 曾在顶层用 `oneOf` 表达"创建 or 更新"，provider 会拒绝整个模型
/// 请求（`Invalid schema for function 'connections_save'`），表现为任务直接失败。这里把
/// "已暴露工具都能转换成 llm-connector 工具定义"固化为 LLM 边界契约。
#[gpui::test]
fn agent_runtime_tool_schemas_convert_to_function_calling(cx: &mut TestAppContext) {
    let specs = cx.update(|cx| {
        register_connection_repository(cx);
        cx.set_global(AppSettings::default());
        agent_runtime_tool_registry(cx)
            .expect("agent registry should build")
            .specs(&ResourceContext::new())
    });

    assert!(!specs.is_empty(), "agent registry should expose tools");
    for spec in specs {
        spec.to_llm_tool().unwrap_or_else(|error| panic!("{error}"));
    }
}

fn assert_tool_risk(registry: &agent_runtime::ToolRegistry, name: &str, risk: RiskLevel) {
    let tool = registry.get(&ToolName::new(name)).expect(name);
    assert_eq!(risk, tool.spec(&ResourceContext::new()).risk, "{name} risk");
}

fn assert_agent_target_schema(
    registry: &agent_runtime::ToolRegistry,
    name: &str,
    required: serde_json::Value,
) {
    let tool = registry.get(&ToolName::new(name)).expect(name);
    let spec = tool.spec(&ResourceContext::new());
    let properties = spec.parameters["properties"].as_object().unwrap();
    assert_eq!(required, spec.parameters["required"], "{name} required");
    assert!(properties.contains_key("target"), "{name} target");
    assert!(!properties.contains_key("connection"), "{name} connection");
    assert!(
        !properties.contains_key("connection_id"),
        "{name} connection_id"
    );
    assert!(!properties.contains_key("session_id"), "{name} session_id");
}

fn test_connection() -> SqliteConnection {
    let conn = SqliteConnection::open_with_pool_size(":memory:", 1)
        .expect("sqlite connection should open");
    conn.with_connection(|db| {
        run_migrations(db)?;
        Ok(())
    })
    .expect("migrations should run");
    conn
}
