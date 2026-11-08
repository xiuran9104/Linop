export type McpExecutionMode = "read_only" | "safe_write" | "high_risk_write";

export interface McpCapabilityRow {
  labelKey: string;
  read_only: boolean;
  safe_write: boolean;
  high_risk_write: boolean;
}

export const MCP_EXECUTION_MODE_COLUMNS: ReadonlyArray<{ mode: McpExecutionMode; labelKey: string }> = [
  { mode: "read_only", labelKey: "settings.mcpExecutionModeReadOnly" },
  { mode: "safe_write", labelKey: "settings.mcpExecutionModeSafeWrite" },
  { mode: "high_risk_write", labelKey: "settings.mcpExecutionModeHighRiskWrite" },
];

export const MCP_CAPABILITY_ROWS: readonly McpCapabilityRow[] = [
  { labelKey: "settings.mcpCapabilityRead", read_only: true, safe_write: true, high_risk_write: true },
  { labelKey: "settings.mcpCapabilityScopedMutation", read_only: false, safe_write: true, high_risk_write: true },
  { labelKey: "settings.mcpCapabilityBroadMutation", read_only: false, safe_write: false, high_risk_write: true },
  { labelKey: "settings.mcpCapabilitySchemaAdmin", read_only: false, safe_write: false, high_risk_write: true },
  { labelKey: "settings.mcpCapabilityConnectionCreate", read_only: false, safe_write: true, high_risk_write: true },
  { labelKey: "settings.mcpCapabilityConnectionRemove", read_only: false, safe_write: false, high_risk_write: true },
];

export const MCP_TOOL_OPTIONS = [
  { name: "linop_db_list_connections", labelKey: "settings.mcpToolListConnections" },
  { name: "linop_db_list_databases", labelKey: "settings.mcpToolListDatabases" },
  { name: "linop_db_list_tables", labelKey: "settings.mcpToolListTables" },
  { name: "linop_db_describe_table", labelKey: "settings.mcpToolDescribeTable" },
  { name: "linop_db_list_routines", labelKey: "settings.mcpToolListRoutines" },
  { name: "linop_db_get_routine_source", labelKey: "settings.mcpToolGetRoutineSource" },
  { name: "linop_db_get_schema_context", labelKey: "settings.mcpToolGetSchemaContext" },
  { name: "linop_db_execute_query", labelKey: "settings.mcpToolExecuteQuery" },
  { name: "linop_db_execute_batch", labelKey: "settings.mcpToolExecuteBatch" },
  { name: "linop_db_open_session", labelKey: "settings.mcpToolOpenSession" },
  { name: "linop_db_begin_transaction", labelKey: "settings.mcpToolBeginTransaction" },
  { name: "linop_db_commit_transaction", labelKey: "settings.mcpToolCommitTransaction" },
  { name: "linop_db_rollback_transaction", labelKey: "settings.mcpToolRollbackTransaction" },
  { name: "linop_db_close_session", labelKey: "settings.mcpToolCloseSession" },
  { name: "linop_db_execute_redis_command", labelKey: "settings.mcpToolExecuteRedisCommand" },
  { name: "linop_db_salesforce_current_user", labelKey: "settings.mcpToolSalesforceCurrentUser" },
  { name: "linop_db_salesforce_prepare_write", labelKey: "settings.mcpToolSalesforcePrepareWrite" },
  { name: "linop_db_salesforce_apply_write", labelKey: "settings.mcpToolSalesforceApplyWrite" },
  { name: "linop_db_nacos_list_namespaces", labelKey: "settings.mcpToolNacosListNamespaces" },
  { name: "linop_db_nacos_list_configs", labelKey: "settings.mcpToolNacosListConfigs" },
  { name: "linop_db_nacos_search_configs", labelKey: "settings.mcpToolNacosSearchConfigs" },
  { name: "linop_db_nacos_get_config", labelKey: "settings.mcpToolNacosGetConfig" },
  { name: "linop_db_nacos_config_history", labelKey: "settings.mcpToolNacosConfigHistory" },
  { name: "linop_db_nacos_publish_config", labelKey: "settings.mcpToolNacosPublishConfig" },
  { name: "linop_db_nacos_rollback_config", labelKey: "settings.mcpToolNacosRollbackConfig" },
  { name: "linop_db_nacos_delete_config", labelKey: "settings.mcpToolNacosDeleteConfig" },
  { name: "linop_db_nacos_list_services", labelKey: "settings.mcpToolNacosListServices" },
  { name: "linop_db_nacos_get_service", labelKey: "settings.mcpToolNacosGetService" },
  { name: "linop_db_nacos_list_instances", labelKey: "settings.mcpToolNacosListInstances" },
  { name: "linop_db_nacos_update_instance", labelKey: "settings.mcpToolNacosUpdateInstance" },
  { name: "linop_db_nacos_register_instance", labelKey: "settings.mcpToolNacosRegisterInstance" },
  { name: "linop_db_nacos_deregister_instance", labelKey: "settings.mcpToolNacosDeregisterInstance" },
  { name: "linop_db_consul_kv_list", labelKey: "settings.mcpToolConsulKvList" },
  { name: "linop_db_consul_kv_get", labelKey: "settings.mcpToolConsulKvGet" },
  { name: "linop_db_consul_kv_put", labelKey: "settings.mcpToolConsulKvPut" },
  { name: "linop_db_consul_kv_delete", labelKey: "settings.mcpToolConsulKvDelete" },
  { name: "linop_db_consul_list_services", labelKey: "settings.mcpToolConsulListServices" },
  { name: "linop_db_consul_list_nodes", labelKey: "settings.mcpToolConsulListNodes" },
  { name: "linop_db_consul_service_health", labelKey: "settings.mcpToolConsulServiceHealth" },
  { name: "linop_db_consul_set_maintenance", labelKey: "settings.mcpToolConsulSetMaintenance" },
  { name: "linop_db_peek_messages", labelKey: "settings.mcpToolPeekMessages" },
  { name: "linop_db_send_message", labelKey: "settings.mcpToolSendMessage" },
  { name: "linop_db_add_connection", labelKey: "settings.mcpToolAddConnection" },
  { name: "linop_db_duplicate_connection", labelKey: "settings.mcpToolDuplicateConnection" },
  { name: "linop_db_remove_connection", labelKey: "settings.mcpToolRemoveConnection" },
  { name: "linop_db_open_table", labelKey: "settings.mcpToolOpenTable" },
  { name: "linop_db_execute_and_show", labelKey: "settings.mcpToolExecuteAndShow" },
  { name: "linop_db_plugin_list", labelKey: "settings.mcpToolPluginList" },
  { name: "linop_db_plugin_tools", labelKey: "settings.mcpToolPluginTools" },
  { name: "linop_db_plugin_call", labelKey: "settings.mcpToolPluginCall" },
] as const;

export interface McpExecutionPolicyFields {
  readOnly: boolean;
  allowDangerousSql: boolean;
}

export interface McpPolicyMutationState {
  loading: boolean;
  saving: boolean;
  loadError: string;
}

export interface McpScopeConnectionLike {
  id: string;
}

export interface McpScopeConnectionGroups<T extends McpScopeConnectionLike> {
  allowed: T[];
  available: T[];
  unavailableAllowedIds: string[];
}

export function isMcpPolicyMutationBlocked(state: McpPolicyMutationState): boolean {
  return state.loading || state.saving || state.loadError.length > 0;
}

export function matchesMcpSearchQuery(rawQuery: string, values: readonly unknown[]): boolean {
  const query = rawQuery.trim().toLocaleLowerCase();
  if (!query) return true;
  return values.some((value) => value !== null && value !== undefined && String(value).toLocaleLowerCase().includes(query));
}

export function groupMcpScopeConnections<T extends McpScopeConnectionLike>(connections: readonly T[], allowedConnectionIds: readonly string[] | null): McpScopeConnectionGroups<T> {
  if (allowedConnectionIds === null) {
    return {
      allowed: [...connections],
      available: [],
      unavailableAllowedIds: [],
    };
  }

  const allowedIds = new Set(allowedConnectionIds);
  const connectionIds = new Set(connections.map((connection) => connection.id));
  return {
    allowed: connections.filter((connection) => allowedIds.has(connection.id)),
    available: connections.filter((connection) => !allowedIds.has(connection.id)),
    unavailableAllowedIds: allowedConnectionIds.filter((id) => !connectionIds.has(id)),
  };
}

export function mcpExecutionModeFromPolicy(policy: McpExecutionPolicyFields): McpExecutionMode {
  if (policy.readOnly) return "read_only";
  return policy.allowDangerousSql ? "high_risk_write" : "safe_write";
}

export function mcpPolicyFieldsForExecutionMode(mode: McpExecutionMode): McpExecutionPolicyFields {
  switch (mode) {
    case "read_only":
      return { readOnly: true, allowDangerousSql: false };
    case "safe_write":
      return { readOnly: false, allowDangerousSql: false };
    case "high_risk_write":
      return { readOnly: false, allowDangerousSql: true };
  }
}

export function toggleMcpAllowedConnectionId(current: readonly string[] | null, availableConnectionIds: readonly string[], connectionId: string, allowed: boolean): string[] {
  return updateMcpAllowedConnectionIds(current, availableConnectionIds, [connectionId], allowed);
}

/**
 * Allowlist entry that exposes every discovered plugin tool (`linop_db_<prefix>__*`
 * scopes to one plugin). Plugin tool names come from sidecars at runtime, so
 * the static options above cannot name them; the backend matcher expands the
 * wildcard, and static tool names never contain the `__` separator.
 */
export const MCP_PLUGIN_TOOLS_WILDCARD = "linop_db_*__*";

export function toggleMcpAllowedToolName(current: readonly string[] | null, toolName: string, allowed: boolean): string[] {
  // A null allowlist means "everything", including every discovered plugin
  // tool. The first toggle materializes the list: seed the plugin wildcard so
  // checking one static tool does not silently strip every dynamic plugin
  // tool — removing the wildcard is an explicit settings action.
  const selected = new Set<string>(current === null ? [...MCP_TOOL_OPTIONS.map((tool) => tool.name), MCP_PLUGIN_TOOLS_WILDCARD] : current);
  if (allowed) selected.add(toolName);
  else selected.delete(toolName);
  return [...selected];
}

/** Adds a free-form allowlist entry (a plugin tool name or a `linop_db_<prefix>__*` wildcard). */
export function addMcpAllowedToolName(current: readonly string[] | null, rawName: string): { names: string[]; added: boolean } {
  const name = rawName.trim();
  if (!name) return { names: [...(current ?? MCP_TOOL_OPTIONS.map((tool) => tool.name))], added: false };
  const selected = new Set<string>(current === null ? MCP_TOOL_OPTIONS.map((tool) => tool.name) : current);
  const added = !selected.has(name);
  if (added) selected.add(name);
  return { names: [...selected], added };
}

/** Saved allowlist entries beyond the static options: plugin tool names and wildcards. */
export function customMcpAllowedToolNames(current: readonly string[] | null): string[] {
  const staticNames = new Set<string>(MCP_TOOL_OPTIONS.map((tool) => tool.name));
  return (current ?? []).filter((name) => !staticNames.has(name));
}

export function updateMcpAllowedConnectionIds(current: readonly string[] | null, availableConnectionIds: readonly string[], connectionIds: readonly string[], allowed: boolean): string[] {
  const selected = new Set(current === null ? availableConnectionIds : current);
  for (const connectionId of connectionIds) {
    if (allowed) selected.add(connectionId);
    else selected.delete(connectionId);
  }
  return [...selected];
}
