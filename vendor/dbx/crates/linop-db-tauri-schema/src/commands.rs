use std::future::Future;
use std::sync::Arc;
use tauri::State;

use linop_db_core::connection::AppState;
use linop_db_core::db;
use linop_db_core::native_jobs::{
    NativeJobContext, NativeJobDetails, NativeJobList, NativeJobListRequest, NativeJobOperationStatus,
    NativeJobPreparedPlan, NativeJobRef, NativeJobReferenceCatalog, NativeJobReferenceListRequest, NativeJobRequest,
};
use linop_db_core::schema::native_jobs as jobs;

#[tauri::command]
pub async fn native_job_open_context(
    state: State<'_, Arc<AppState>>,
    connection_id: String,
    database: String,
) -> Result<NativeJobContext, String> {
    jobs::native_job_open_context_core(&state, &connection_id, &database).await
}

#[tauri::command]
pub async fn native_job_list(
    state: State<'_, Arc<AppState>>,
    request: NativeJobListRequest,
) -> Result<NativeJobList, String> {
    jobs::native_job_list_core(&state, &request).await
}

#[tauri::command]
pub async fn native_job_references(
    state: State<'_, Arc<AppState>>,
    request: NativeJobReferenceListRequest,
) -> Result<NativeJobReferenceCatalog, String> {
    jobs::native_job_references_core(&state, &request).await
}

#[tauri::command]
pub async fn native_job_read(
    state: State<'_, Arc<AppState>>,
    target: NativeJobRef,
) -> Result<NativeJobDetails, String> {
    jobs::native_job_read_core(&state, &target).await
}

#[tauri::command]
pub async fn native_job_prepare(
    state: State<'_, Arc<AppState>>,
    mutation: NativeJobRequest,
) -> Result<NativeJobPreparedPlan, String> {
    jobs::native_job_prepare_core(&state, mutation).await
}

#[tauri::command]
pub async fn native_job_apply(
    state: State<'_, Arc<AppState>>,
    plan_id: String,
) -> Result<NativeJobOperationStatus, String> {
    jobs::native_job_apply_core(state.inner().clone(), &plan_id)
}

#[tauri::command]
pub async fn native_job_status(
    state: State<'_, Arc<AppState>>,
    operation_id: String,
    observe: Option<bool>,
) -> Result<NativeJobOperationStatus, String> {
    Ok(jobs::native_job_status_observed_core(&state, &operation_id, observe.unwrap_or(false)).await)
}

#[tauri::command]
pub async fn native_job_cancel(
    state: State<'_, Arc<AppState>>,
    operation_id: String,
) -> Result<NativeJobOperationStatus, String> {
    Ok(jobs::native_job_cancel(&state, &operation_id))
}

async fn run_cancellable<T, F>(state: &Arc<AppState>, execution_id: Option<String>, future: F) -> Result<T, String>
where
    F: Future<Output = Result<T, String>>,
{
    let registered =
        execution_id.as_ref().filter(|id| !id.trim().is_empty()).map(|id| state.running_queries.register(id.clone()));
    if let Some(query) = registered.as_ref() {
        let token = query.token();
        tokio::select! {
            biased;
            _ = token.cancelled() => Err(linop_db_core::query::canceled_error()),
            result = future => result,
        }
    } else {
        future.await
    }
}

/// Resolve a non-internal catalog for dispatch to the Doris multi-catalog path.
/// Thin wrapper around the shared linop-db-core resolver so the Tauri and HTTP
/// backends stay in sync.
async fn external_doris_catalog(state: &AppState, connection_id: &str, catalog: Option<&str>) -> Option<String> {
    linop_db_core::schema::resolve_external_doris_catalog(state, connection_id, catalog).await
}

#[tauri::command]
pub async fn list_databases(
    state: State<'_, Arc<AppState>>,
    connection_id: String,
) -> Result<Vec<db::DatabaseInfo>, String> {
    linop_db_core::schema::list_databases_core(&state, &connection_id).await
}

#[tauri::command]
pub async fn list_database_metadata(
    state: State<'_, Arc<AppState>>,
    connection_id: String,
) -> Result<Vec<db::DatabaseInfo>, String> {
    linop_db_core::schema::list_database_metadata_core(&state, &connection_id).await
}

#[tauri::command]
pub async fn list_database_storage(
    state: State<'_, Arc<AppState>>,
    connection_id: String,
    databases: Vec<String>,
) -> Result<Vec<db::DatabaseStorageInfo>, String> {
    linop_db_core::schema::list_database_storage_core(&state, &connection_id, &databases).await
}

#[tauri::command]
pub async fn list_xugu_tablespaces(
    state: State<'_, Arc<AppState>>,
    connection_id: String,
    database: Option<String>,
) -> Result<Vec<db::XuguTablespaceInfo>, String> {
    linop_db_core::schema::list_xugu_tablespaces_core(&state, &connection_id, database.as_deref()).await
}

#[tauri::command]
pub async fn get_sqlserver_completion_context(
    state: State<'_, Arc<AppState>>,
    connection_id: String,
    database: String,
) -> Result<db::sqlserver::SqlServerCompletionContext, String> {
    linop_db_core::schema::get_sqlserver_completion_context_core(&state, &connection_id, &database).await
}

#[tauri::command]
pub async fn list_doris_catalogs(
    state: State<'_, Arc<AppState>>,
    connection_id: String,
) -> Result<Vec<db::CatalogInfo>, String> {
    linop_db_core::schema::list_doris_catalogs_core(&state, &connection_id).await
}

#[tauri::command]
pub async fn list_doris_catalog_databases(
    state: State<'_, Arc<AppState>>,
    connection_id: String,
    catalog: String,
) -> Result<Vec<db::DatabaseInfo>, String> {
    linop_db_core::schema::list_doris_catalog_databases_core(&state, &connection_id, &catalog).await
}

#[tauri::command]
pub async fn list_sqlserver_linked_servers(
    state: State<'_, Arc<AppState>>,
    connection_id: String,
) -> Result<Vec<db::LinkedServerInfo>, String> {
    linop_db_core::schema::list_sqlserver_linked_servers_core(&state, &connection_id).await
}

#[tauri::command]
pub async fn list_sqlserver_linked_server_catalogs(
    state: State<'_, Arc<AppState>>,
    connection_id: String,
    server: String,
) -> Result<Vec<db::DatabaseInfo>, String> {
    linop_db_core::schema::list_sqlserver_linked_server_catalogs_core(&state, &connection_id, &server).await
}

#[tauri::command]
pub async fn list_sqlserver_linked_server_schemas(
    state: State<'_, Arc<AppState>>,
    connection_id: String,
    server: String,
    catalog: String,
) -> Result<Vec<String>, String> {
    linop_db_core::schema::list_sqlserver_linked_server_schemas_core(&state, &connection_id, &server, &catalog).await
}

#[tauri::command]
pub async fn list_sqlserver_linked_server_tables(
    state: State<'_, Arc<AppState>>,
    connection_id: String,
    server: String,
    catalog: String,
    schema: String,
    filter: Option<String>,
    limit: Option<usize>,
    offset: Option<usize>,
) -> Result<Vec<db::TableInfo>, String> {
    linop_db_core::schema::list_sqlserver_linked_server_tables_core(
        &state,
        &connection_id,
        &server,
        &catalog,
        &schema,
        filter.as_deref(),
        limit,
        offset,
    )
    .await
}

#[tauri::command]
pub async fn list_schemas(
    state: State<'_, Arc<AppState>>,
    connection_id: String,
    database: String,
    apply_visible_filter: Option<bool>,
) -> Result<Vec<String>, String> {
    linop_db_core::schema::list_schemas_core_with_visible_filter(
        &state,
        &connection_id,
        &database,
        apply_visible_filter.unwrap_or(false),
    )
    .await
}

#[tauri::command]
pub async fn list_schema_infos(
    state: State<'_, Arc<AppState>>,
    connection_id: String,
    database: String,
) -> Result<Vec<db::SchemaInfo>, String> {
    linop_db_core::schema::list_schema_infos_core(&state, &connection_id, &database).await
}

#[tauri::command]
pub async fn list_data_types(
    state: State<'_, Arc<AppState>>,
    connection_id: String,
    database: String,
) -> Result<Vec<String>, String> {
    linop_db_core::schema::list_data_types_core(&state, &connection_id, &database).await
}

#[tauri::command]
#[allow(clippy::too_many_arguments)]
pub async fn list_tables(
    state: State<'_, Arc<AppState>>,
    connection_id: String,
    database: String,
    schema: String,
    filter: Option<String>,
    limit: Option<usize>,
    offset: Option<usize>,
    object_types: Option<Vec<String>>,
    catalog: Option<String>,
    table_name_filter: Option<linop_db_core::schema::TableNameFilter>,
    exact_identity: Option<bool>,
    page_table_roots: Option<bool>,
) -> Result<Vec<db::TableInfo>, String> {
    if exact_identity.unwrap_or(false) {
        return linop_db_core::schema::exact_metadata::tables(
            &state,
            &connection_id,
            &database,
            &schema,
            filter.as_deref(),
            limit,
            offset,
            object_types.as_deref(),
            table_name_filter.as_ref(),
        )
        .await;
    }
    if let Some(catalog) = external_doris_catalog(&state, &connection_id, catalog.as_deref()).await {
        return linop_db_core::schema::list_doris_catalog_tables_core(
            &state,
            &connection_id,
            &catalog,
            &database,
            filter.as_deref(),
            limit,
            offset,
            object_types.as_deref(),
            table_name_filter.as_ref(),
        )
        .await;
    }
    if page_table_roots.unwrap_or(false) {
        return linop_db_core::schema::list_table_roots_page_core(
            &state,
            &connection_id,
            &database,
            &schema,
            filter.as_deref(),
            limit,
            offset,
            object_types.as_deref(),
            table_name_filter.as_ref(),
        )
        .await;
    }
    linop_db_core::schema::list_tables_core(
        &state,
        &connection_id,
        &database,
        &schema,
        filter.as_deref(),
        limit,
        offset,
        object_types.as_deref(),
        table_name_filter.as_ref(),
    )
    .await
}

#[tauri::command]
pub async fn get_table_comment(
    state: State<'_, Arc<AppState>>,
    connection_id: String,
    database: String,
    schema: String,
    table: String,
    catalog: Option<String>,
    exact_identity: Option<bool>,
) -> Result<Option<String>, String> {
    if exact_identity.unwrap_or(false) {
        return linop_db_core::schema::exact_metadata::comment(&state, &connection_id, &database, &schema, &table)
            .await;
    }
    if let Some(catalog) = external_doris_catalog(&state, &connection_id, catalog.as_deref()).await {
        return linop_db_core::schema::get_doris_catalog_table_comment_core(
            &state,
            &connection_id,
            &catalog,
            &database,
            &table,
        )
        .await;
    }
    linop_db_core::schema::get_table_comment_core(&state, &connection_id, &database, &schema, &table).await
}

#[tauri::command]
pub async fn get_mysql_table_auto_increment(
    state: State<'_, Arc<AppState>>,
    connection_id: String,
    database: String,
    table: String,
) -> Result<Option<String>, String> {
    linop_db_core::schema::get_mysql_table_auto_increment_core(&state, &connection_id, &database, &table).await
}

#[tauri::command]
#[allow(clippy::too_many_arguments)]
pub async fn list_objects(
    state: State<'_, Arc<AppState>>,
    connection_id: String,
    database: String,
    schema: String,
    filter: Option<String>,
    limit: Option<usize>,
    offset: Option<usize>,
    object_types: Option<Vec<String>>,
    catalog: Option<String>,
    table_name_filter: Option<linop_db_core::schema::TableNameFilter>,
    execution_id: Option<String>,
    identity_mode: Option<linop_db_core::schema::MetadataIdentityMode>,
    exact_identity: Option<bool>,
    lightweight: Option<bool>,
) -> Result<Vec<db::ObjectInfo>, String> {
    let app = Arc::clone(state.inner());
    let operation_app = Arc::clone(&app);
    run_cancellable(&app, execution_id, async move {
        if exact_identity.unwrap_or(false) {
            return linop_db_core::schema::exact_metadata::objects(
                &operation_app,
                &connection_id,
                &database,
                &schema,
                filter.as_deref(),
                limit,
                offset,
                object_types.as_deref(),
                table_name_filter.as_ref(),
            )
            .await;
        }
        if let Some(catalog) = external_doris_catalog(&operation_app, &connection_id, catalog.as_deref()).await {
            let tables = linop_db_core::schema::list_doris_catalog_tables_core(
                &operation_app,
                &connection_id,
                &catalog,
                &database,
                filter.as_deref(),
                limit,
                offset,
                object_types.as_deref(),
                table_name_filter.as_ref(),
            )
            .await?;
            return Ok(tables
                .into_iter()
                .map(|table| db::ObjectInfo {
                    name: table.name,
                    object_type: table.table_type,
                    schema: Some(database.clone()),
                    valid: None,
                    signature: None,
                    comment: table.comment,
                    created_at: None,
                    updated_at: None,
                    parent_schema: table.parent_schema,
                    parent_name: table.parent_name,
                    custom_type_kind: None,
                    has_members: None,
                    trigger: None,
                    xugu_type_members_expandable: None,
                })
                .collect());
        }
        linop_db_core::schema::list_objects_core_with_loading_options(
            &operation_app,
            &connection_id,
            &database,
            &schema,
            filter.as_deref(),
            limit,
            offset,
            object_types.as_deref(),
            table_name_filter.as_ref(),
            &linop_db_core::schema::MetadataReadOptions { identity_mode, catalog },
            lightweight,
        )
        .await
    })
    .await
}

#[tauri::command]
pub async fn list_object_statistics(
    state: State<'_, Arc<AppState>>,
    connection_id: String,
    database: String,
    schema: String,
) -> Result<Vec<db::ObjectStatistics>, String> {
    linop_db_core::schema::list_object_statistics_core(&state, &connection_id, &database, &schema).await
}

#[tauri::command]
pub async fn list_completion_objects(
    state: State<'_, Arc<AppState>>,
    connection_id: String,
    database: String,
    schema: String,
) -> Result<Vec<db::ObjectInfo>, String> {
    linop_db_core::schema::list_completion_objects_core(&state, &connection_id, &database, &schema).await
}

#[tauri::command]
pub async fn completion_assistant_search(
    state: State<'_, Arc<AppState>>,
    request: db::CompletionAssistantRequest,
) -> Result<db::CompletionAssistantResponse, String> {
    linop_db_core::schema::completion_assistant_search_core(&state, request).await
}

#[tauri::command]
pub async fn get_object_source(
    state: State<'_, Arc<AppState>>,
    connection_id: String,
    database: String,
    schema: String,
    name: String,
    object_type: db::ObjectSourceKind,
    signature: Option<String>,
    relation_name: Option<String>,
    identity_mode: Option<linop_db_core::schema::MetadataIdentityMode>,
    catalog: Option<String>,
) -> Result<db::ObjectSource, String> {
    linop_db_core::schema::get_object_source_core_with_options(
        &state,
        &connection_id,
        &database,
        &schema,
        &name,
        object_type,
        signature.as_deref(),
        relation_name.as_deref(),
        &linop_db_core::schema::MetadataReadOptions { identity_mode, catalog },
    )
    .await
}

#[tauri::command]
pub async fn get_event_info(
    state: State<'_, Arc<AppState>>,
    connection_id: String,
    database: String,
    schema: String,
    name: String,
) -> Result<db::MysqlEventInfo, String> {
    linop_db_core::schema::get_event_info_core(&state, &connection_id, &database, &schema, &name).await
}

#[tauri::command]
pub async fn get_custom_type_details(
    state: State<'_, Arc<AppState>>,
    connection_id: String,
    database: String,
    schema: String,
    name: String,
) -> Result<db::CustomTypeDetails, String> {
    linop_db_core::schema::get_custom_type_details_core(&state, &connection_id, &database, &schema, &name).await
}

#[tauri::command]
pub async fn get_oracle_type_management_snapshot(
    state: State<'_, Arc<AppState>>,
    connection_id: String,
    database: String,
    owner: String,
    name: String,
    known_objects: Option<Vec<db::OracleTypeManagementObjectKey>>,
) -> Result<db::OracleTypeManagementSnapshot, String> {
    linop_db_core::schema::get_oracle_type_management_snapshot_with_objects_core(
        &state,
        &connection_id,
        &database,
        &owner,
        &name,
        known_objects.as_deref().unwrap_or_default(),
    )
    .await
}

#[tauri::command]
pub fn build_oracle_type_management_sql(
    input: db::OracleTypeManagementSqlInput,
) -> Result<db::OracleTypeManagementSqlResult, String> {
    linop_db_core::schema::build_oracle_type_management_sql(input)
}

#[tauri::command]
pub async fn verify_oracle_type_data(
    state: State<'_, Arc<AppState>>,
    input: db::OracleTypeDataRequest,
) -> Result<db::OracleTypeDataResult, String> {
    linop_db_core::schema::verify_oracle_type_data_core(&state, input).await
}

#[tauri::command]
pub async fn prepare_oracle_type_evolution(
    state: State<'_, Arc<AppState>>,
    input: db::OracleTypeEvolutionRequest,
) -> Result<db::OracleTypeEvolutionManifest, String> {
    linop_db_core::schema::prepare_oracle_type_evolution_core(&state, input).await
}

#[tauri::command]
pub async fn get_columns(
    state: State<'_, Arc<AppState>>,
    connection_id: String,
    database: String,
    schema: String,
    table: String,
    catalog: Option<String>,
    client_session_id: Option<String>,
    current_schema: Option<String>,
    exact_identity: Option<bool>,
) -> Result<Vec<db::ColumnInfo>, String> {
    if exact_identity.unwrap_or(false) {
        return linop_db_core::schema::exact_metadata::columns(
            &state,
            &connection_id,
            &database,
            &schema,
            &table,
            client_session_id.as_deref(),
        )
        .await;
    }
    if let Some(catalog) = external_doris_catalog(&state, &connection_id, catalog.as_deref()).await {
        return linop_db_core::schema::get_doris_catalog_columns_core(
            &state,
            &connection_id,
            &catalog,
            &database,
            &table,
        )
        .await;
    }
    linop_db_core::schema::get_columns_core_for_session_in_context(
        &state,
        &connection_id,
        &database,
        &schema,
        &table,
        client_session_id.as_deref(),
        current_schema.as_deref(),
    )
    .await
}

/// Read-only Plugin Host API over the existing connection. The core function
/// performs the open-connection gate before entering the ordinary metadata path.
#[tauri::command]
pub async fn get_plugin_table_metadata(
    state: State<'_, Arc<AppState>>,
    request: linop_db_core::schema::plugin_metadata::PluginTableContext,
) -> Result<linop_db_core::schema::plugin_metadata::PluginTableMetadata, String> {
    linop_db_core::schema::plugin_metadata::get_table_metadata(&state, request).await
}

#[tauri::command]
pub async fn get_all_columns(
    state: State<'_, Arc<AppState>>,
    connection_id: String,
    database: String,
    schema: String,
) -> Result<Vec<db::TableColumnsResult>, String> {
    linop_db_core::schema::get_all_columns_core(&state, &connection_id, &database, &schema).await
}

#[tauri::command]
pub async fn get_sqlserver_column_metadata(
    state: State<'_, Arc<AppState>>,
    connection_id: String,
    database: String,
    schema: String,
    table: String,
) -> Result<Vec<db::sqlserver::SqlServerColumnMetadata>, String> {
    linop_db_core::schema::get_sqlserver_column_metadata_core(&state, &connection_id, &database, &schema, &table).await
}

#[tauri::command]
pub async fn list_indexes(
    state: State<'_, Arc<AppState>>,
    connection_id: String,
    database: String,
    schema: String,
    table: String,
    catalog: Option<String>,
    exact_identity: Option<bool>,
) -> Result<Vec<db::IndexInfo>, String> {
    if exact_identity.unwrap_or(false) {
        return linop_db_core::schema::exact_metadata::indexes(&state, &connection_id, &database, &schema, &table)
            .await;
    }
    if let Some(catalog) = external_doris_catalog(&state, &connection_id, catalog.as_deref()).await {
        return linop_db_core::schema::list_doris_catalog_indexes_core(
            &state,
            &connection_id,
            &catalog,
            &database,
            &table,
        )
        .await;
    }
    linop_db_core::schema::list_indexes_core(&state, &connection_id, &database, &schema, &table).await
}

#[tauri::command]
pub async fn list_reference_key_columns(
    state: State<'_, Arc<AppState>>,
    connection_id: String,
    database: String,
    schema: String,
    table: String,
    catalog: Option<String>,
) -> Result<Vec<String>, String> {
    if let Some(catalog) = external_doris_catalog(&state, &connection_id, catalog.as_deref()).await {
        let indexes =
            linop_db_core::schema::list_doris_catalog_indexes_core(&state, &connection_id, &catalog, &database, &table)
                .await?;
        return Ok(linop_db_core::schema::reference_key_columns_from_indexes(&indexes));
    }
    linop_db_core::schema::list_reference_key_columns_core(&state, &connection_id, &database, &schema, &table).await
}

#[tauri::command]
pub async fn list_reference_keys(
    state: State<'_, Arc<AppState>>,
    connection_id: String,
    database: String,
    schema: String,
    table: String,
    catalog: Option<String>,
) -> Result<Vec<linop_db_core::schema::ReferenceKeyInfo>, String> {
    if let Some(catalog) = external_doris_catalog(&state, &connection_id, catalog.as_deref()).await {
        let indexes =
            linop_db_core::schema::list_doris_catalog_indexes_core(&state, &connection_id, &catalog, &database, &table)
                .await?;
        return Ok(linop_db_core::schema::reference_keys_from_indexes(&indexes));
    }
    linop_db_core::schema::list_reference_keys_core(&state, &connection_id, &database, &schema, &table).await
}

#[tauri::command]
pub async fn list_foreign_keys(
    state: State<'_, Arc<AppState>>,
    connection_id: String,
    database: String,
    schema: String,
    table: String,
    catalog: Option<String>,
    exact_identity: Option<bool>,
) -> Result<Vec<db::ForeignKeyInfo>, String> {
    if exact_identity.unwrap_or(false) {
        return linop_db_core::schema::exact_metadata::foreign_keys(&state, &connection_id, &database, &schema, &table)
            .await;
    }
    if let Some(catalog) = external_doris_catalog(&state, &connection_id, catalog.as_deref()).await {
        return linop_db_core::schema::list_doris_catalog_foreign_keys_core(
            &state,
            &connection_id,
            &catalog,
            &database,
            &table,
        )
        .await;
    }
    linop_db_core::schema::list_foreign_keys_core(&state, &connection_id, &database, &schema, &table).await
}

#[tauri::command]
pub async fn list_foreign_keys_for_database(
    state: State<'_, Arc<AppState>>,
    connection_id: String,
    database: String,
    schema: String,
    catalog: Option<String>,
    execution_id: Option<String>,
) -> Result<std::collections::HashMap<String, Vec<db::ForeignKeyInfo>>, String> {
    let app = Arc::clone(state.inner());
    let operation_app = Arc::clone(&app);
    run_cancellable(&app, execution_id, async move {
        if external_doris_catalog(&operation_app, &connection_id, catalog.as_deref()).await.is_some() {
            return Ok(std::collections::HashMap::new());
        }
        linop_db_core::schema::list_foreign_keys_for_database_core(&operation_app, &connection_id, &database, &schema)
            .await
    })
    .await
}

#[tauri::command]
pub async fn list_triggers(
    state: State<'_, Arc<AppState>>,
    connection_id: String,
    database: String,
    schema: String,
    table: String,
    catalog: Option<String>,
    identity_mode: Option<linop_db_core::schema::MetadataIdentityMode>,
    exact_identity: Option<bool>,
) -> Result<Vec<db::TriggerInfo>, String> {
    if exact_identity.unwrap_or(false) {
        return linop_db_core::schema::exact_metadata::triggers(&state, &connection_id, &database, &schema, &table)
            .await;
    }
    if let Some(catalog) = external_doris_catalog(&state, &connection_id, catalog.as_deref()).await {
        return linop_db_core::schema::list_doris_catalog_triggers_core(
            &state,
            &connection_id,
            &catalog,
            &database,
            &table,
        )
        .await;
    }
    linop_db_core::schema::list_triggers_core_with_options(
        &state,
        &connection_id,
        &database,
        &schema,
        &table,
        &linop_db_core::schema::MetadataReadOptions { identity_mode, catalog },
    )
    .await
}

#[tauri::command]
pub async fn list_constraints(
    state: State<'_, Arc<AppState>>,
    connection_id: String,
    database: String,
    schema: String,
    table: String,
    catalog: Option<String>,
    exact_identity: Option<bool>,
) -> Result<Vec<linop_db_core::db::ConstraintInfo>, String> {
    if exact_identity.unwrap_or(false) {
        return linop_db_core::schema::exact_metadata::constraints(&state, &connection_id, &database, &schema, &table)
            .await;
    }
    let _ = catalog;
    linop_db_core::schema::list_constraints_core(&state, &connection_id, &database, &schema, &table).await
}

#[tauri::command]
pub async fn list_partitions(
    state: State<'_, Arc<AppState>>,
    connection_id: String,
    database: String,
    schema: String,
    table: String,
) -> Result<Vec<linop_db_core::db::PartitionInfo>, String> {
    linop_db_core::schema::list_partitions_core(&state, &connection_id, &database, &schema, &table).await
}

#[tauri::command]
pub async fn get_table_partition_status(
    state: State<'_, Arc<AppState>>,
    connection_id: String,
    database: String,
    schema: String,
    table: String,
) -> Result<linop_db_core::schema::TablePartitionStatus, String> {
    linop_db_core::schema::table_partition_status_core(&state, &connection_id, &database, &schema, &table).await
}

#[tauri::command]
pub async fn list_invalid_indexes(
    state: State<'_, Arc<AppState>>,
    connection_id: String,
    database: String,
    schema: String,
    table: String,
) -> Result<Vec<String>, String> {
    linop_db_core::schema::list_invalid_indexes_core(&state, &connection_id, &database, &schema, &table).await
}

#[tauri::command]
pub async fn get_table_partitioning(
    state: State<'_, Arc<AppState>>,
    connection_id: String,
    database: String,
    schema: String,
    table: String,
) -> Result<linop_db_core::db::PgTablePartitioning, String> {
    linop_db_core::schema::get_table_partitioning_core(&state, &connection_id, &database, &schema, &table).await
}

#[tauri::command]
pub async fn list_subpartitions(
    state: State<'_, Arc<AppState>>,
    connection_id: String,
    database: String,
    schema: String,
    table: String,
) -> Result<Vec<linop_db_core::db::SubpartitionInfo>, String> {
    linop_db_core::schema::list_subpartitions_core(&state, &connection_id, &database, &schema, &table).await
}

#[tauri::command]
#[allow(clippy::too_many_arguments)]
pub async fn get_table_ddl(
    state: State<'_, Arc<AppState>>,
    connection_id: String,
    database: String,
    schema: String,
    table: String,
    object_type: Option<db::ObjectSourceKind>,
    catalog: Option<String>,
    include_postgres_access: Option<bool>,
    portable: Option<bool>,
    exact_identity: Option<bool>,
    separate_indexes: Option<bool>,
    explicit_owner: Option<String>,
) -> Result<String, String> {
    let separate_indexes = separate_indexes.unwrap_or(false) && object_type.is_none();
    let result = if exact_identity.unwrap_or(false) {
        linop_db_core::schema::exact_metadata::ddl(
            &state,
            &connection_id,
            &database,
            &schema,
            &table,
            object_type,
            portable.unwrap_or(false),
        )
        .await?
    } else if let Some(catalog) = external_doris_catalog(&state, &connection_id, catalog.as_deref()).await {
        linop_db_core::schema::get_doris_catalog_table_ddl_core(&state, &connection_id, &catalog, &database, &table)
            .await?
    } else if portable.unwrap_or(false) {
        linop_db_core::schema::get_table_export_ddl_core(
            &state,
            &connection_id,
            &database,
            &schema,
            &table,
            object_type,
        )
        .await?
    } else if include_postgres_access.unwrap_or(false) {
        linop_db_core::schema::get_table_display_ddl_core(
            &state,
            &connection_id,
            &database,
            &schema,
            &table,
            object_type,
        )
        .await?
    } else {
        linop_db_core::schema::get_table_ddl_core(&state, &connection_id, &database, &schema, &table, object_type)
            .await?
    };
    if separate_indexes {
        linop_db_core::schema::separate_table_ddl_indexes_core(
            &state,
            &connection_id,
            &database,
            &schema,
            &table,
            &result,
            explicit_owner.as_deref().or_else(|| exact_identity.unwrap_or(false).then_some(schema.as_str())),
        )
        .await
    } else {
        Ok(result)
    }
}

#[tauri::command]
pub async fn list_functions(
    state: State<'_, Arc<AppState>>,
    connection_id: String,
    database: String,
    schema: String,
) -> Result<Vec<db::FunctionInfo>, String> {
    linop_db_core::schema::list_functions_core(&state, &connection_id, &database, &schema).await
}

#[tauri::command]
pub async fn load_schema_diff_routines(
    state: State<'_, Arc<AppState>>,
    connection_id: String,
    database: String,
    schema: String,
    selected_routines: Option<Vec<String>>,
    object_kinds: Option<Vec<db::ObjectSourceKind>>,
    selected_objects: Option<Vec<linop_db_core::schema::oracle_routines::RoutineSelection>>,
) -> Result<linop_db_core::schema::oracle_routines::RoutineSnapshot, String> {
    linop_db_core::schema::oracle_routines::load_schema_diff_routines_with_options_core(
        &state,
        &connection_id,
        &database,
        &schema,
        linop_db_core::schema::oracle_routines::RoutineReadOptions {
            object_kinds,
            selected_objects,
            selected_routines,
        },
    )
    .await
}

#[tauri::command]
pub async fn prepare_oracle_routine_save(
    state: State<'_, Arc<AppState>>,
    input: linop_db_core::schema::oracle_routines::OracleRoutineSaveInput,
) -> Result<linop_db_core::schema::oracle_routines::RoutineOperationRecord, String> {
    linop_db_core::schema::oracle_routines::prepare_oracle_routine_save_core(&state, input).await
}

#[tauri::command]
pub async fn prepare_oracle_routine_plan(
    state: State<'_, Arc<AppState>>,
    input: linop_db_core::schema::oracle_routines::OracleRoutinePlanInput,
) -> Result<linop_db_core::schema::oracle_routines::OracleRoutinePlan, String> {
    linop_db_core::schema::oracle_routines::prepare_oracle_routine_plan_core(&state, input).await
}

#[tauri::command]
pub async fn prepare_oracle_routine_recovery(
    state: State<'_, Arc<AppState>>,
    operation_id: String,
) -> Result<linop_db_core::schema::oracle_routines::OracleRoutinePlan, String> {
    linop_db_core::schema::oracle_routines::prepare_oracle_routine_recovery_core(&state, &operation_id).await
}

#[tauri::command]
pub async fn execute_oracle_routine_operation(
    state: State<'_, Arc<AppState>>,
    operation_id: String,
    approved_sql: String,
) -> Result<linop_db_core::schema::oracle_routines::RoutineOperationRecord, String> {
    linop_db_core::schema::oracle_routines::execute_oracle_routine_operation_core(
        state.inner().clone(),
        operation_id,
        approved_sql,
    )
    .await
}

#[tauri::command]
pub async fn observe_oracle_routine_operation(
    state: State<'_, Arc<AppState>>,
    operation_id: String,
    refresh_readback: Option<bool>,
) -> Result<linop_db_core::schema::oracle_routines::RoutineOperationObservation, String> {
    Ok(linop_db_core::schema::oracle_routines::observe_oracle_routine_operation_core(
        &state,
        &operation_id,
        refresh_readback.unwrap_or(false),
    )
    .await)
}

#[tauri::command]
pub fn update_oracle_routine_operation_references(
    state: State<'_, Arc<AppState>>,
    retain_operation_ids: Vec<String>,
    release_operation_ids: Vec<String>,
) -> Result<(), String> {
    state.routine_operations.update_references(&retain_operation_ids, &release_operation_ids)
}

#[tauri::command]
pub async fn list_sequences(
    state: State<'_, Arc<AppState>>,
    connection_id: String,
    database: String,
    schema: String,
    with_last_values: bool,
) -> Result<Vec<db::SequenceInfo>, String> {
    linop_db_core::schema::list_sequences_core(&state, &connection_id, &database, &schema, with_last_values).await
}

#[tauri::command]
pub async fn list_rules(
    state: State<'_, Arc<AppState>>,
    connection_id: String,
    database: String,
    schema: String,
) -> Result<Vec<db::RuleInfo>, String> {
    linop_db_core::schema::list_rules_core(&state, &connection_id, &database, &schema).await
}

#[tauri::command]
pub async fn list_owners(
    state: State<'_, Arc<AppState>>,
    connection_id: String,
    database: String,
    schema: String,
) -> Result<Vec<db::OwnerInfo>, String> {
    linop_db_core::schema::list_owners_core(&state, &connection_id, &database, &schema).await
}

#[tauri::command]
pub async fn get_table_owner(
    state: State<'_, Arc<AppState>>,
    connection_id: String,
    database: String,
    schema: String,
    table: String,
) -> Result<Option<String>, String> {
    linop_db_core::schema::get_table_owner_core(&state, &connection_id, &database, &schema, &table).await
}

#[tauri::command]
pub async fn list_extensions(
    state: State<'_, Arc<AppState>>,
    connection_id: String,
    database: String,
    schema: Option<String>,
) -> Result<Vec<db::ExtensionInfo>, String> {
    linop_db_core::schema::list_extensions_core(&state, &connection_id, &database, schema.as_deref()).await
}

#[tauri::command]
pub async fn list_available_extensions(
    state: State<'_, Arc<AppState>>,
    connection_id: String,
    database: String,
) -> Result<Vec<db::ExtensionInfo>, String> {
    linop_db_core::schema::list_available_extensions_core(&state, &connection_id, &database).await
}

#[tauri::command]
pub async fn list_event_triggers(
    state: State<'_, Arc<AppState>>,
    connection_id: String,
    database: String,
) -> Result<Vec<db::EventTriggerInfo>, String> {
    linop_db_core::schema::list_event_triggers_core(&state, &connection_id, &database).await
}
