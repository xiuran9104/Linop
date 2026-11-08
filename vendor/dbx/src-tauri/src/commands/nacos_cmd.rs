use std::path::Path;
use std::sync::Arc;

use tauri::ipc::Channel;
use tauri::State;

use crate::commands::connection::AppState;

#[tauri::command]
pub async fn nacos_test_connection(
    state: State<'_, Arc<AppState>>,
    connection_id: String,
    force_refresh: Option<bool>,
) -> Result<linop_db_core::nacos::NacosConnectionInfo, String> {
    linop_db_core::nacos::service::nacos_test_connection_core(&state, &connection_id, force_refresh.unwrap_or(false))
        .await
}

#[tauri::command]
pub async fn nacos_list_namespaces(
    state: State<'_, Arc<AppState>>,
    connection_id: String,
) -> Result<Vec<linop_db_core::nacos::NacosNamespaceInfo>, String> {
    linop_db_core::nacos::service::nacos_list_namespaces_core(&state, &connection_id).await
}

#[tauri::command]
pub async fn nacos_sidebar_snapshot(
    state: State<'_, Arc<AppState>>,
    connection_id: String,
) -> Result<linop_db_core::nacos::NacosNamespaceSidebarSnapshot, String> {
    linop_db_core::nacos::service::nacos_sidebar_snapshot_core(&state, &connection_id).await
}

#[tauri::command]
pub async fn nacos_create_namespace(
    state: State<'_, Arc<AppState>>,
    connection_id: String,
    req: linop_db_core::nacos::NacosNamespaceCreate,
) -> Result<(), String> {
    linop_db_core::nacos::service::nacos_create_namespace_core(&state, &connection_id, req).await
}

#[tauri::command]
pub async fn nacos_update_namespace(
    state: State<'_, Arc<AppState>>,
    connection_id: String,
    req: linop_db_core::nacos::NacosNamespaceUpdate,
) -> Result<(), String> {
    linop_db_core::nacos::service::nacos_update_namespace_core(&state, &connection_id, req).await
}

#[tauri::command]
pub async fn nacos_delete_namespace(
    state: State<'_, Arc<AppState>>,
    connection_id: String,
    namespace_id: String,
) -> Result<(), String> {
    linop_db_core::nacos::service::nacos_delete_namespace_core(&state, &connection_id, namespace_id).await
}

#[tauri::command]
pub async fn nacos_list_configs(
    state: State<'_, Arc<AppState>>,
    connection_id: String,
    query: linop_db_core::nacos::NacosConfigQuery,
) -> Result<linop_db_core::nacos::NacosConfigList, String> {
    linop_db_core::nacos::service::nacos_list_configs_core(&state, &connection_id, query).await
}

#[tauri::command]
pub async fn nacos_get_config(
    state: State<'_, Arc<AppState>>,
    connection_id: String,
    key: linop_db_core::nacos::NacosConfigKey,
) -> Result<linop_db_core::nacos::NacosConfigItem, String> {
    linop_db_core::nacos::service::nacos_get_config_core(&state, &connection_id, key).await
}

#[tauri::command]
pub async fn nacos_publish_config(
    state: State<'_, Arc<AppState>>,
    connection_id: String,
    req: linop_db_core::nacos::NacosConfigUpsert,
) -> Result<(), String> {
    linop_db_core::nacos::service::nacos_publish_config_core(&state, &connection_id, req).await
}

#[tauri::command]
pub async fn nacos_delete_config(
    state: State<'_, Arc<AppState>>,
    connection_id: String,
    key: linop_db_core::nacos::NacosConfigKey,
) -> Result<(), String> {
    linop_db_core::nacos::service::nacos_delete_config_core(&state, &connection_id, key).await
}

#[tauri::command]
pub async fn nacos_list_config_history(
    state: State<'_, Arc<AppState>>,
    connection_id: String,
    query: linop_db_core::nacos::NacosConfigHistoryQuery,
) -> Result<linop_db_core::nacos::NacosConfigHistoryList, String> {
    linop_db_core::nacos::service::nacos_list_config_history_core(&state, &connection_id, query).await
}

#[tauri::command]
pub async fn nacos_get_config_history(
    state: State<'_, Arc<AppState>>,
    connection_id: String,
    key: linop_db_core::nacos::NacosConfigHistoryKey,
) -> Result<linop_db_core::nacos::NacosConfigItem, String> {
    linop_db_core::nacos::service::nacos_get_config_history_core(&state, &connection_id, key).await
}

#[tauri::command]
pub async fn nacos_rollback_config(
    state: State<'_, Arc<AppState>>,
    connection_id: String,
    req: linop_db_core::nacos::NacosConfigRollbackRequest,
) -> Result<(), String> {
    linop_db_core::nacos::service::nacos_rollback_config_core(&state, &connection_id, req).await
}

#[tauri::command]
pub async fn nacos_get_rnacos_console_captcha(
    state: State<'_, Arc<AppState>>,
    connection_id: String,
) -> Result<linop_db_core::nacos::NacosRNacosConsoleCaptcha, String> {
    linop_db_core::nacos::service::nacos_get_rnacos_console_captcha_core(&state, &connection_id).await
}

#[tauri::command]
pub async fn nacos_login_rnacos_console(
    state: State<'_, Arc<AppState>>,
    connection_id: String,
    captcha: Option<String>,
) -> Result<(), String> {
    linop_db_core::nacos::service::nacos_login_rnacos_console_core(&state, &connection_id, captcha).await
}

#[tauri::command]
pub async fn nacos_list_users(
    state: State<'_, Arc<AppState>>,
    connection_id: String,
    query: linop_db_core::nacos::NacosUserQuery,
) -> Result<linop_db_core::nacos::NacosUserList, String> {
    linop_db_core::nacos::service::nacos_list_users_core(&state, &connection_id, query).await
}

#[tauri::command]
pub async fn nacos_create_user(
    state: State<'_, Arc<AppState>>,
    connection_id: String,
    req: linop_db_core::nacos::NacosUserCreate,
) -> Result<(), String> {
    linop_db_core::nacos::service::nacos_create_user_core(&state, &connection_id, req).await
}

#[tauri::command]
pub async fn nacos_update_user(
    state: State<'_, Arc<AppState>>,
    connection_id: String,
    req: linop_db_core::nacos::NacosUserUpdate,
) -> Result<(), String> {
    linop_db_core::nacos::service::nacos_update_user_core(&state, &connection_id, req).await
}

#[tauri::command]
pub async fn nacos_delete_user(
    state: State<'_, Arc<AppState>>,
    connection_id: String,
    username: String,
) -> Result<(), String> {
    linop_db_core::nacos::service::nacos_delete_user_core(&state, &connection_id, username).await
}

#[tauri::command]
pub async fn nacos_list_role_bindings(
    state: State<'_, Arc<AppState>>,
    connection_id: String,
    query: linop_db_core::nacos::NacosRoleQuery,
) -> Result<linop_db_core::nacos::NacosRoleList, String> {
    linop_db_core::nacos::service::nacos_list_role_bindings_core(&state, &connection_id, query).await
}

#[tauri::command]
pub async fn nacos_assign_role(
    state: State<'_, Arc<AppState>>,
    connection_id: String,
    binding: linop_db_core::nacos::NacosRoleBinding,
) -> Result<(), String> {
    linop_db_core::nacos::service::nacos_assign_role_core(&state, &connection_id, binding).await
}

#[tauri::command]
pub async fn nacos_remove_role(
    state: State<'_, Arc<AppState>>,
    connection_id: String,
    binding: linop_db_core::nacos::NacosRoleBinding,
) -> Result<(), String> {
    linop_db_core::nacos::service::nacos_remove_role_core(&state, &connection_id, binding).await
}

#[tauri::command]
pub async fn nacos_access_snapshot(
    state: State<'_, Arc<AppState>>,
    connection_id: String,
) -> Result<linop_db_core::nacos::NacosAccessControlSnapshot, String> {
    linop_db_core::nacos::service::nacos_access_snapshot_core(&state, &connection_id).await
}

#[tauri::command]
pub async fn nacos_start_access_operation(
    state: State<'_, Arc<AppState>>,
    connection_id: String,
    req: linop_db_core::nacos::NacosAccessOperationRequest,
) -> Result<linop_db_core::nacos::NacosAccessOperationResult, String> {
    linop_db_core::nacos::service::nacos_start_access_operation_core(&state, &connection_id, req).await
}

#[tauri::command]
pub async fn nacos_get_access_operation(
    state: State<'_, Arc<AppState>>,
    connection_id: String,
    operation_id: String,
) -> Result<linop_db_core::nacos::NacosAccessOperationResult, String> {
    linop_db_core::nacos::service::nacos_get_access_operation_core(&state, &connection_id, &operation_id).await
}

#[tauri::command]
pub async fn nacos_retry_access_operation(
    state: State<'_, Arc<AppState>>,
    connection_id: String,
    retry: linop_db_core::nacos::NacosAccessOperationRetry,
) -> Result<linop_db_core::nacos::NacosAccessOperationResult, String> {
    linop_db_core::nacos::service::nacos_retry_access_operation_core(&state, &connection_id, retry).await
}

#[tauri::command]
pub async fn nacos_undo_access_operation(
    state: State<'_, Arc<AppState>>,
    connection_id: String,
    operation_id: String,
) -> Result<linop_db_core::nacos::NacosAccessOperationResult, String> {
    linop_db_core::nacos::service::nacos_undo_access_operation_core(&state, &connection_id, &operation_id).await
}

#[tauri::command]
pub async fn nacos_list_services(
    state: State<'_, Arc<AppState>>,
    connection_id: String,
    query: linop_db_core::nacos::NacosServiceQuery,
) -> Result<linop_db_core::nacos::NacosServiceList, String> {
    linop_db_core::nacos::service::nacos_list_services_core(&state, &connection_id, query).await
}

#[tauri::command]
pub async fn nacos_get_service(
    state: State<'_, Arc<AppState>>,
    connection_id: String,
    query: linop_db_core::nacos::NacosServiceQuery,
) -> Result<linop_db_core::nacos::NacosServiceDetail, String> {
    linop_db_core::nacos::service::nacos_get_service_core(&state, &connection_id, query).await
}

#[tauri::command]
pub async fn nacos_create_service(
    state: State<'_, Arc<AppState>>,
    connection_id: String,
    req: linop_db_core::nacos::NacosServiceUpsert,
) -> Result<(), String> {
    linop_db_core::nacos::service::nacos_create_service_core(&state, &connection_id, req).await
}

#[tauri::command]
pub async fn nacos_update_service(
    state: State<'_, Arc<AppState>>,
    connection_id: String,
    req: linop_db_core::nacos::NacosServiceUpsert,
) -> Result<(), String> {
    linop_db_core::nacos::service::nacos_update_service_core(&state, &connection_id, req).await
}

#[tauri::command]
pub async fn nacos_delete_service(
    state: State<'_, Arc<AppState>>,
    connection_id: String,
    query: linop_db_core::nacos::NacosServiceQuery,
) -> Result<(), String> {
    linop_db_core::nacos::service::nacos_delete_service_core(&state, &connection_id, query).await
}

#[tauri::command]
pub async fn nacos_list_instances(
    state: State<'_, Arc<AppState>>,
    connection_id: String,
    query: linop_db_core::nacos::NacosInstanceQuery,
) -> Result<Vec<linop_db_core::nacos::NacosInstanceInfo>, String> {
    linop_db_core::nacos::service::nacos_list_instances_core(&state, &connection_id, query).await
}

#[tauri::command]
pub async fn nacos_update_instance(
    state: State<'_, Arc<AppState>>,
    connection_id: String,
    req: linop_db_core::nacos::NacosInstanceUpdateRequest,
) -> Result<(), String> {
    linop_db_core::nacos::service::nacos_update_instance_core(&state, &connection_id, req).await
}

#[tauri::command]
pub async fn nacos_register_instance(
    state: State<'_, Arc<AppState>>,
    connection_id: String,
    req: linop_db_core::nacos::NacosInstanceRegistration,
) -> Result<(), String> {
    linop_db_core::nacos::service::nacos_register_instance_core(&state, &connection_id, req).await
}

#[tauri::command]
pub async fn nacos_deregister_instance(
    state: State<'_, Arc<AppState>>,
    connection_id: String,
    req: linop_db_core::nacos::NacosInstanceRef,
) -> Result<(), String> {
    linop_db_core::nacos::service::nacos_deregister_instance_core(&state, &connection_id, req).await
}

#[tauri::command]
pub async fn nacos_get_dashboard(
    state: State<'_, Arc<AppState>>,
    connection_id: String,
    query: linop_db_core::nacos::NacosDashboardQuery,
) -> Result<linop_db_core::nacos::NacosDashboardSnapshot, String> {
    linop_db_core::nacos::service::nacos_get_dashboard_core(&state, &connection_id, query).await
}

#[tauri::command]
pub async fn nacos_raw_request(
    state: State<'_, Arc<AppState>>,
    connection_id: String,
    req: linop_db_core::nacos::NacosRawRequest,
) -> Result<linop_db_core::nacos::NacosRawResponse, String> {
    linop_db_core::nacos::service::nacos_raw_request_core(&state, &connection_id, req).await
}

#[tauri::command]
pub async fn nacos_search_config_content(
    state: State<'_, Arc<AppState>>,
    connection_id: String,
    req: linop_db_core::nacos::NacosContentSearchRequest,
    on_progress: Channel<linop_db_core::nacos::NacosSearchProgress>,
) -> Result<linop_db_core::nacos::NacosContentSearchResult, String> {
    linop_db_core::nacos::service::nacos_search_config_content_core(&state, &connection_id, req, move |progress| {
        let _ = on_progress.send(progress);
        std::future::ready(())
    })
    .await
}

#[tauri::command]
pub async fn nacos_cancel_operation(operation_id: String) -> Result<bool, String> {
    Ok(linop_db_core::nacos::service::nacos_cancel_operation_core(&operation_id))
}

#[tauri::command]
pub async fn nacos_export_configs(
    state: State<'_, Arc<AppState>>,
    connection_id: String,
    selector: linop_db_core::nacos::NacosConfigSelector,
    destination: String,
) -> Result<(), String> {
    linop_db_core::nacos::batch::nacos_export_config_archive_core(
        &state,
        &connection_id,
        selector,
        Path::new(&destination),
    )
    .await
    .map(|_| ())
}

#[tauri::command]
pub async fn nacos_preview_config_import(
    state: State<'_, Arc<AppState>>,
    connection_id: String,
    target_namespace: String,
    archive_path: String,
) -> Result<linop_db_core::nacos::NacosBatchPreview, String> {
    linop_db_core::nacos::batch::nacos_preview_config_import_core(
        &state,
        &connection_id,
        &target_namespace,
        Path::new(&archive_path),
    )
    .await
}

#[tauri::command]
pub async fn nacos_apply_config_import(
    state: State<'_, Arc<AppState>>,
    connection_id: String,
    operation_id: String,
    target_namespace: String,
    archive_path: String,
    plan_hash: String,
    conflict_policy: linop_db_core::nacos::NacosConflictPolicy,
) -> Result<linop_db_core::nacos::NacosBatchReport, String> {
    linop_db_core::nacos::batch::nacos_apply_config_import_core(
        &state,
        &connection_id,
        &target_namespace,
        Path::new(&archive_path),
        &operation_id,
        &plan_hash,
        &conflict_policy,
    )
    .await
}

#[tauri::command]
pub async fn nacos_preview_config_transfer(
    state: State<'_, Arc<AppState>>,
    req: linop_db_core::nacos::NacosConfigTransferRequest,
) -> Result<linop_db_core::nacos::NacosBatchPreview, String> {
    linop_db_core::nacos::batch::nacos_preview_config_transfer_core(&state, &req).await
}

#[tauri::command]
pub async fn nacos_apply_config_transfer(
    state: State<'_, Arc<AppState>>,
    req: linop_db_core::nacos::NacosConfigTransferRequest,
    plan_hash: String,
) -> Result<linop_db_core::nacos::NacosBatchReport, String> {
    linop_db_core::nacos::batch::nacos_apply_config_transfer_core(&state, &req, &plan_hash).await
}
