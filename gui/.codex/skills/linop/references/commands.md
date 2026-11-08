# Dynamic command discovery

All Agent calls include `--json`. Add `--discovery <path>` only when the user or host supplies an explicit discovery file.

The running Linop host is authoritative for tool names, descriptions, annotations, and input schemas:

```bash
linop status --json
linop --help
linop tool list --json
linop tool schema <tool-name> --json
```

Prefer the domain command shown by `linop --help`. Use the low-level tool call only when no matching domain command exists or its mapping cannot represent the live schema:

```bash
linop tool call <tool-name> --arguments '<json-object-matching-live-schema>' --json
```

## Representative Bash commands

First discover the connection or session IDs with the live host tools. Replace every placeholder with an ID returned by Linop, and inspect the domain help/schema before executing:

```bash
linop connections list --json
linop connections sessions --json

linop ssh exec \
  --target <ssh-session-id> \
  --command 'uname -a' \
  --json

linop sftp list \
  --connection <ssh-connection-id-or-name> \
  --path /var/log \
  --json

linop redis get \
  --connection-id <redis-connection-id-or-name> \
  --key app:status \
  --json

linop mongo find \
  --connection-id <mongo-session-id> \
  --database app \
  --collection users \
  --filter '{"active":true}' \
  --limit 20 \
  --json

linop db query \
  --connection <database-connection-id-or-name> \
  --sql 'SELECT 1' \
  --json

linop db exec \
  --connection <database-connection-id-or-name> \
  --database app \
  --sql 'CREATE TABLE IF NOT EXISTS example (id BIGINT PRIMARY KEY)' \
  --json

linop terminal read \
  --target <terminal-session-id> \
  --lines 80 \
  --json
```

For SQL, use `db query` only for read-only statements and `db exec` for DDL, DML, scripts, and other write-capable SQL. Do not use `tool call db.exec` when the `db exec` domain command is available, and do not pass onetcli-only flags such as `--allow-write` to the `linop` CLI. Linop's live permission mode and approval flow govern mutations.

These are convenience mappings, not a promise that every host exposes every domain. When a convenience command is absent or does not match the live schema, use `tool list`, `tool schema`, and `tool call`.

Compatibility aliases are available for low-level discovery and fallback calls:

```bash
linop tools --json
linop schema <tool-name> --json
linop call <tool-name> --arguments '<json-object>' --json
```
