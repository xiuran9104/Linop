# Crates

Rust crates for DBX live here.

## Directories

- `linop-db-core/` - application orchestration: connections, queries, schema operations, data workflows, AI tools, administration, persistence, safety, and host services.
- `linop-db-types/` - shared connection, query, metadata, and serialization contracts; generated database identities.
- `linop-db-sql/` - SQL parsing, dialects, analysis, risk classification, DDL/DML generation, and schema diff planning.
- `linop-db-drivers/` - native database adapters, database Agents, tunnels, execution budgets, and driver lifecycle.
- `linop-db-formats/` - data formatting and file encoders, independent of database connections.
- `linop-db-ai-provider/` - AI provider clients, CLI adapters, streaming, and token usage.
- `linop-db-plugin-runtime/` - plugin packages, signatures, marketplace, subprocess sessions, and host requests.
- `linop-db-platform/` - shared process, path, proxy, download, version, and host-prompt primitives.
- `linop-db-web/` - the Docker/web backend service binary published as `linop-db-web`.
- `linop-db-cli/` - the command-line application.
- `linop-db-mcp/` - the MCP service library and binary.
- `linop-db-sqlite-worker/` - the isolated SQLite file-host worker and protocol.

The workspace root is defined in the repository-level `Cargo.toml`.

See [ARCHITECTURE.md](ARCHITECTURE.md) for dependency rules, compatibility exports, feature selection, and validation commands.
