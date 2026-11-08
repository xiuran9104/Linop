use std::any::TypeId;

fn assert_same_type<Legacy: 'static, Extracted: 'static>() {
    assert_eq!(TypeId::of::<Legacy>(), TypeId::of::<Extracted>());
}

#[test]
fn extracted_crates_preserve_legacy_type_identity() {
    assert_same_type::<linop_db_core::models::connection::ConnectionConfig, linop_db_types::models::connection::ConnectionConfig>(
    );
    assert_same_type::<linop_db_core::models::connection::DatabaseType, linop_db_types::models::connection::DatabaseType>();
    assert_same_type::<linop_db_core::types::QueryResult, linop_db_types::types::QueryResult>();
    assert_same_type::<linop_db_core::db::mysql::MySqlPool, linop_db_drivers::db::mysql::MySqlPool>();
    assert_same_type::<linop_db_core::db::agent_driver::AgentDriverClient, linop_db_drivers::db::agent_driver::AgentDriverClient>(
    );
    assert_same_type::<linop_db_core::schema_diff::SchemaDiffPreparation, linop_db_sql::schema_diff::SchemaDiffPreparation>();
    assert_same_type::<
        linop_db_core::table_structure_sql::TableStructureSqlOptions,
        linop_db_sql::table_structure_sql::TableStructureSqlOptions,
    >();
    assert_same_type::<linop_db_core::csv_export::CsvQuoteMode, linop_db_formats::csv_export::CsvQuoteMode>();
    assert_same_type::<linop_db_core::plugins::PluginManifest, linop_db_plugin_runtime::plugins::PluginManifest>();
    assert_same_type::<linop_db_core::ai::AiProvider, linop_db_ai_provider::ai::AiProvider>();
    assert_same_type::<linop_db_core::db::ssh_prompt::SshPromptRequest, linop_db_platform::ssh_prompt::SshPromptRequest>();
    assert_same_type::<linop_db_core::db::ssh_prompt::UserInputRequest, linop_db_platform::ssh_prompt::UserInputRequest>();
}

#[test]
fn business_directories_preserve_legacy_namespaces() {
    assert_same_type::<linop_db_core::storage::Storage, linop_db_core::persistence::storage::Storage>();
    assert_same_type::<linop_db_core::query_cancel::RunningTaskMetadata, linop_db_core::query::query_cancel::RunningTaskMetadata>(
    );
}
