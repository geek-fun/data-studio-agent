# @geek-fun/data-studio-mcp

Unified [Model Context Protocol](https://modelcontextprotocol.io/) server for SQL and NoSQL databases, powered by the [dockit](https://github.com/geek-fun/dockit) and [sqlkit](https://github.com/geek-fun/sqlkit) desktop apps.

Give any MCP-compatible AI coding agent (Claude Code, Cursor, Windsurf, OpenCode, Codex) direct access to your databases:

- **SQL** (via sqlkit): PostgreSQL, MySQL, SQL Server, SQLite
- **NoSQL** (via dockit): Elasticsearch, OpenSearch, MongoDB, DynamoDB

## How it works

```
code agent (Claude Code / Cursor / OpenCode ...)
    |
    | MCP stdio protocol
    v
@geek-fun/data-studio-mcp   ← this package (pure TypeScript)
    |
    | HTTP (localhost)
    +----------------+----------------+
    v                v                |
dockit:9120    sqlkit:9121            |
(NoSQL bridge)  (SQL bridge)          |
    |                |                |
    v                v                |
Elasticsearch    PostgreSQL           |
MongoDB          MySQL                |
DynamoDB         SQL Server           |
OpenSearch       SQLite               |
```

The MCP server is a thin routing layer. All database drivers, SSH tunnels, and connection management live in the desktop apps, which expose a local HTTP bridge (`127.0.0.1` only). No credentials ever leave your machine.

## Prerequisites

1. Install [dockit](https://github.com/geek-fun/dockit) and/or [sqlkit](https://github.com/geek-fun/sqlkit)
2. Launch the app(s) — the MCP bridge starts automatically (enable **Settings → MCP Bridge → Auto-start**)
3. Configure at least one database connection in the app

The MCP server auto-discovers running backends via each app's port file. Only the tools of running backends are exposed — run both apps to get the full tool set.

## Installation

Install globally (recommended):

```bash
npm install -g @geek-fun/data-studio-mcp
```

Or run directly with `npx` (no install needed):

```bash
npx -y @geek-fun/data-studio-mcp
```

## Configuration

### Claude Code

Add to your MCP config:

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

### Cursor / Windsurf / OpenCode / other MCP clients

Register a new MCP server with:

- **Type**: stdio
- **Command**: `npx`
- **Args**: `-y @geek-fun/data-studio-mcp`

## Tool naming

All tools follow the `data_studio__{backend}__{action}` convention:

| Prefix | Backend | Examples |
|---|---|---|
| `data_studio__sql_*` | sqlkit | `data_studio__sql_execute`, `data_studio__sql_list_tables` |
| `data_studio__es_*` | dockit | `data_studio__es_search`, `data_studio__es_list_indices` |
| `data_studio__mongo_*` | dockit | `data_studio__mongo_find`, `data_studio__mongo_insert` |
| `data_studio__dynamo_*` | dockit | `data_studio__dynamo_query`, `data_studio__dynamo_list_tables` |

## Safety

- The bridge binds to `127.0.0.1` only — unreachable from other machines
- **Destructive and elevated operations are rejected by the bridge** — only read-safe capabilities are exposed through the MCP server
- Credentials are never exposed to the agent; all connections are resolved inside the desktop apps

## Development

```bash
npm install
npm run build      # compile TypeScript
npm test           # run unit tests (vitest)
npm run lint:check # ESLint
```

## Releasing

Bump the version in `package.json` and merge to `master`. The [release workflow](https://github.com/geek-fun/data-studio-agent/blob/master/.github/workflows/release-mcp.yml) publishes the package to npm (OIDC trusted publishing) and creates a tag + GitHub release.

## License

[Apache-2.0](https://github.com/geek-fun/data-studio-agent/blob/master/LICENSE)
