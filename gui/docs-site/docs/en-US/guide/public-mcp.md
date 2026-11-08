# Public MCP and external automation

Public MCP lets Codex, Claude Desktop, Claude Code, and compatible clients call the currently running Linop. It is not a fixed cloud API: discovery uses a dynamic loopback port and a user-only token, while live tools and schemas come from the local application.

## Choose a server mode

Temporary mode is appropriate for short-lived use; clients should not expect it to remain available after the task or app ends. Persistent mode supports clients that need continuing local discovery. Both should remain loopback-only and use the protected per-user discovery information.

Do not expose discovery files, tokens, or ports to the public network. Changing mode, restarting Linop, or changing Tool Exposure may restart the endpoint, requiring MCP and ACP clients to reconnect.

## Set permissions and Tool Exposure

Safe, Confirm, and Auto profiles control approval behavior. Begin with Safe or Confirm, and use Auto only when the task, target, and tools are tightly controlled. These profiles do not replace database or server authorization.

Tool Exposure independently controls Terminal, SSH Exec, visible terminal, Connections, SFTP, Redis, MongoDB, Database, and internal functions. Enable only the groups needed for the current client, then turn them off afterward.

## Install client prerequisites

The bridge requires Node.js 20+ and working `npx`. Verify the version, then copy the generated configuration for Codex, Claude Desktop, Claude Code, or a generic MCP JSON client from Linop Settings. Follow each client's path and restart instructions.

Linop can also install or update its Skill for Codex and Agents. Install `@linop/cli` globally before using the Skill; an AI Agent then uses `linop ... --json` to discover and operate databases, SSH sessions, terminals, files, connections, and workspaces owned by Linop. The Skill does not place a static copy of every tool in the prompt; it teaches the Agent to inspect status, commands, and live schemas only when needed.

## Why use the Linop Skill

When Linop is registered as a native MCP server, a client may send many enabled tool names, descriptions, and JSON schemas to the model on every turn. As the catalog grows, those repeated definitions consume context and Tokens. The Linop Skill gives an AI Agent a compact workflow and runs `linop` status, domain, or `tool schema/call` commands only when the task requires them.

```bash
npm install -g @linop/cli@latest
linop skill install --target codex --scope user
linop status --json
linop db query --help
linop tool schema <tool-name> --json
linop tool call <tool-name> --arguments '<json-object>' --json
```

Representative read-only operations are shown below. Replace placeholders with connection or session ids returned by the running Linop app, and inspect `--help` or the live schema before execution:

```bash
linop connections list --json
linop connections sessions --json
linop ssh exec --target <ssh-session-id> --command 'uname -a' --json
linop sftp list --connection <ssh-connection-id-or-name> --path /var/log --json
linop redis get --connection-id <redis-connection-id-or-name> --key app:status --json
linop mongo find --connection-id <mongo-session-id> --database app --collection users --filter '{"active":true}' --limit 20 --json
linop db query --connection <database-connection-id-or-name> --sql 'SELECT 1' --json
linop terminal read --target <terminal-session-id> --lines 80 --json
```

This is useful for terminal-capable Agents such as Codex: they can discover current Linop resources and operations on demand without pre-registering the complete Linop catalog in every model turn. It can reduce repeated context and token overhead, although the exact savings depend on how the client injects MCP tool definitions and how many groups are enabled.

The Skill does not completely remove MCP from the transport. The `linop` CLI still connects internally to Linop's authenticated loopback Public MCP endpoint, and Linop remains responsible for Tool Exposure, resource ids, permissions, approvals, sessions, results, and auditing. The Skill changes the Agent-side interaction from “carry every tool each turn” to “discover and call through terminal commands on demand.”

## Use @linop/cli

`@linop/cli` provides `status`, `tools`, `schema`, `call`, and resource-oriented domain commands. The separate `@linop/mcp` package only runs the stdio bridge for compatible MCP clients.

Run `linop ...` only after checking the package source. Always obtain resource IDs and argument definitions from the current tools/schema output. Never guess IDs, reuse IDs from another device, or attempt to bypass approval.

Check or update the resolved CLI when needed:

```bash
npm view @linop/cli version
linop --version
```

Run `npm install -g @linop/cli@latest` before using the Skill, and `npm update -g @linop/cli` to update it. Native MCP clients should use `npx -y @linop/mcp@latest` as their stdio bridge.

## Approve and troubleshoot

The approval window shows the requesting client, operation, resource, and parameters. Reject unexpected requests and correct the client instead of globally weakening permissions. ACP authorization does not automatically approve Public MCP.

For failures, check that Linop is running, server mode, Node.js, client configuration, discovery permissions, Tool Exposure, and live schema. Reconnect after endpoint changes. Redact tokens, local paths, connection names, and business parameters from logs.
