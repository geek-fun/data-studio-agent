# Permission Configuration

How to control which database operations your AI agent can run. Covers the three-layer model: tool risk definitions, server-side policy enforcement, and client-side confirmation UX.

## The three layers

| Layer | What | Where | Who configures |
|---|---|---|---|
| **L1 — Tool risk** | Every capability declares a static `RiskLevel` (`safe` / `elevated` / `destructive`) | `data-studio-agent` crate, capability registry | We (static metadata) |
| **L2 — Server policy** | `McpPolicy` decides each invocation: `Allow` / `Ask` / `Deny` | dockit/sqlkit bridges — both the MCP bridge **and** the built-in agent loop | User (Settings → MCP Bridge) |
| **L3 — Client session policy** | Your AI tool's own confirmation UX (Ask/Auto mode, permission rules) | The agent client itself | User / agent client |

The layers never merge. L1 describes tools, L2 enforces safety, L3 shapes the confirmation dialogs.

## Policy decisions (`PolicyAction`)

`McpPolicy.decide(risk_level, connection_id)` returns one of:

| Action | Meaning | Enforced by |
|---|---|---|
| `Allow` | Proceed | bridge + loop |
| `Ask` | Proceed, but confirmation is recommended | Informational this iteration — both paths execute; the `Ask` signal is reserved on the enum for future server-driven confirmation |
| `Deny` | Blocked (403 for MCP; short-circuited before execution in the built-in loop) | bridge + loop |

Compatibility: `allows()` (used internally and by the MCP bridge) is exactly `decide() != Deny` — `Ask` counts as allowed.

The built-in agent loop enforces `Deny` server-side: a denied tool is marked `denied` and never reaches execution or the confirmation dialog, without a frontend round-trip.

---

## Server-side policy (dockit/sqlkit Settings → MCP)

Open **Settings → MCP Bridge** in dockit or sqlkit.

### Permission modes

| Mode | Exposed risk levels | Use case |
|---|---|---|
| **Read Only** | Safe only | Explore schemas, run SELECT queries. No writes. |
| **Data Read/Write** | Safe + Elevated | INSERT, UPDATE, index operations. No deletes/drops. |
| **Full Access** | Safe + Elevated + Destructive | Everything, including DELETE, DROP, TRUNCATE. |

The mode is set per app (dockit and sqlkit independently). The MCP server queries both bridges at startup and only exposes tools each bridge's mode allows.

### Confirm destructive toggle

When **ON** (default): Destructive tools are listed in `tools/list` with `destructiveHint: true` so clients can show a confirmation prompt, but the bridge still enforces that only Full Access mode actually executes them.

When **OFF**: Destructive tools are hidden entirely from `tools/list`. The agent never sees them.

### Connection allowlist

By default, the bridge exposes all connections. To restrict which database connections the MCP server can access:

- Add connection IDs to the allowlist in Settings → MCP Bridge
- Empty allowlist = all connections allowed
- The `data_studio__list_connections` tool only returns connections in the allowlist

### Per-connection read-only override

You can mark individual connections as read-only regardless of the global mode. This blocks Elevated and Destructive operations for that specific connection.

---

## ToolAnnotations

The server marks every tool with [MCP ToolAnnotations](https://modelcontextprotocol.io/specification/2025-03-26/server/tools#annotations) based on its risk level. Clients can use these to show native confirmation dialogs.

### Risk levels

| Level | `readOnlyHint` | `destructiveHint` | `idempotentHint` | `openWorldHint` | Examples |
|---|---|---|---|---|---|
| **safe** | `true` | `false` | `true` | `false` | `es__search`, `mongo__find`, `sql_list_tables`, `dynamo__query` |
| **elevated** | `false` | `false` | `false` | `true` | `es__index`, `mongo__insert`, `sql_execute` (INSERT/UPDATE), `dynamo__put_item` |
| **destructive** | `false` | `true` | `false` | `true` | `es__delete_*`, `mongo__delete`, `dynamo__delete_*`, `sql_execute` (DROP/DELETE/TRUNCATE) |

### Which tools are destructive

The bridge reports `riskLevel: "destructive"` for:

- **Elasticsearch**: `es_delete_by_query`, `es_delete_index`
- **MongoDB**: `mongo_delete`, `mongo_drop_collection`
- **DynamoDB**: `dynamo_delete_item`, `dynamo_delete_table`
- **SQL**: `sql_execute` when the statement contains `DROP`, `DELETE`, or `TRUNCATE`

Clients that respect `destructiveHint: true` will show a native confirmation prompt before the tool runs.

---

## `--readonly` mode

Run the MCP server in read-only mode to expose only safe tools at the client level:

```bash
npx -y @geek-fun/data-studio-mcp --readonly
```

Or the equivalent flag:

```bash
npx -y @geek-fun/data-studio-mcp --read-only
```

This filters the tool list before sending it to the client. Only tools with `riskLevel: "safe"` (and `data_studio__list_connections`) are advertised. The agent never sees elevated or destructive tools.

This is a client-side reinforced filter. The bridge still enforces its own `McpPolicy` independently. Using `--readonly` when the bridge is in Full Access mode is redundant but harmless.

---

## Per-client configuration

### Claude Code

Add the server to `.mcp.json` in your project root (or `~/.claude/mcp.json` for global):

```json
{
  "mcpServers": {
    "data-studio": {
      "command": "npx",
      "args": ["-y", "@geek-fun/data-studio-mcp"]
    }
  }
}
```

**Read-only variant:**

```json
{
  "mcpServers": {
    "data-studio": {
      "command": "npx",
      "args": ["-y", "@geek-fun/data-studio-mcp", "--readonly"]
    }
  }
}
```

**Permission rules** (`settings.json`): Claude Code lets you define ask/deny patterns for tool calls. Add rules to confirm destructive operations before they run:

```json
{
  "permissions": {
    "ask": [
      "mcp__data-studio__*delete*",
      "mcp__data-studio__*drop*",
      "mcp__data-studio__*truncate*"
    ]
  }
}
```

Tool names follow the pattern `mcp__<server-name>__<tool-name>`. If your server is named `data-studio`, the prefix is `mcp__data-studio__`.

### OpenCode

Add to `opencode.json`:

```json
{
  "mcp": {
    "data-studio": {
      "command": "npx",
      "args": ["-y", "@geek-fun/data-studio-mcp"]
    }
  }
}
```

**Read-only variant:**

```json
{
  "mcp": {
    "data-studio": {
      "command": "npx",
      "args": ["-y", "@geek-fun/data-studio-mcp", "--readonly"]
    }
  }
}
```

**Permission rules**: OpenCode supports glob-based ask rules for MCP tools. Add patterns to require confirmation:

```json
{
  "mcp": {
    "data-studio": {
      "command": "npx",
      "args": ["-y", "@geek-fun/data-studio-mcp"]
    }
  },
  "mcpPermissions": {
    "ask": [
      "mcp__data-studio__*delete*",
      "mcp__data-studio__*drop*",
      "mcp__data-studio__*truncate*"
    ]
  }
}
```

### Cursor

Add to `.cursor/mcp.json` in your project root:

```json
{
  "mcpServers": {
    "data-studio": {
      "command": "npx",
      "args": ["-y", "@geek-fun/data-studio-mcp"]
    }
  }
}
```

**Read-only variant:**

```json
{
  "mcpServers": {
    "data-studio": {
      "command": "npx",
      "args": ["-y", "@geek-fun/data-studio-mcp", "--readonly"]
    }
  }
}
```

**Permissions**: Cursor uses `permissions.json` with an `mcpAllowlist`. If a tool is not in the allowlist, Cursor prompts the user for approval each time. Omitting the allowlist entry means the user gets a confirmation prompt for every call. Tools in the allowlist run without prompting.

### Codex

Add to `config.toml`:

```toml
[mcp_servers.data_studio]
command = "npx"
args = ["-y", "@geek-fun/data-studio-mcp"]
```

**Read-only variant:**

```toml
[mcp_servers.data_studio]
command = "npx"
args = ["-y", "@geek-fun/data-studio-mcp", "--readonly"]
```

**Approval mode**: Set `approval_mode = "writes"` to require confirmation for write operations:

```toml
[mcp_servers.data_studio]
command = "npx"
args = ["-y", "@geek-fun/data-studio-mcp"]
approval_mode = "writes"
```

---

## Security model

### Two layers

1. **Client UX layer**: Your AI tool reads `ToolAnnotations` from `tools/list` and can show confirmation dialogs for `destructiveHint: true` tools. This is a convenience feature. It depends on the client implementing the annotation check.

2. **Server enforcement layer**: The dockit/sqlkit bridge enforces the permission mode regardless of client behavior. If the bridge is in Read Only mode, it rejects Elevated and Destructive calls at the HTTP level. The MCP server passes through the rejection as an error result.

### Network binding

Both dockit and sqlkit bridges bind to `127.0.0.1` only. The MCP server connects to `http://127.0.0.1:<port>`. No traffic leaves the local machine.

### Connection isolation

- Credentials live in the desktop apps. The MCP server never sees database passwords, API keys, or SSH tunnel configurations.
- The connection allowlist restricts which connections the MCP server can access, even if the bridge exposes more tools.
- Per-connection read-only overrides apply at the bridge level, before the MCP server sees the request.

### Trust model

`destructiveHint` is a hint from a trusted local server. It is not a security boundary. The actual security boundary is the bridge's permission mode, which rejects operations regardless of what the client sends. If you need to guarantee the agent cannot run destructive operations, use `--readonly` mode or set the bridge to Read Only.

---

## Version requirements

- `@geek-fun/data-studio-mcp` >= 0.1.4
- dockit or sqlkit with Phase 4 MCP bridge (permission modes, tool annotations, connection allowlist)
