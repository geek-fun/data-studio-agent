# @geek-fun/data-studio-mcp

Turn your AI coding agent (Claude Code, Cursor, Windsurf, OpenCode, Codex) into a database assistant. Ask it to query your databases, explore schemas, and write SQL or NoSQL queries — it works with:

- **SQL databases** (via [sqlkit](https://github.com/geek-fun/sqlkit)): PostgreSQL, MySQL, SQL Server, SQLite
- **NoSQL databases** (via [dockit](https://github.com/geek-fun/dockit)): Elasticsearch, OpenSearch, MongoDB, DynamoDB

You already have the desktop apps installed? Then setup is two steps: install this package, and add it to your AI tool. No server to host, no API keys to manage — everything runs locally on your machine.

## Prerequisites

1. Install [dockit](https://github.com/geek-fun/dockit) and/or [sqlkit](https://github.com/geek-fun/sqlkit) (free, open source)
2. Launch the app and add at least one database connection
3. Make sure **Settings → MCP Bridge → Auto-start** is enabled (it is by default)

Install both apps to get the full tool set — SQL and NoSQL. One app is enough to get started.

## Installation

**Global install (recommended):**

```bash
npm install -g @geek-fun/data-studio-mcp
```

**Or run without installing** (each time):

```bash
npx -y @geek-fun/data-studio-mcp
```

The package downloads once, then runs locally. Your data never leaves your machine.

## Add it to your AI tool

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

Add a new MCP server:

| Setting | Value |
|---|---|
| Type | stdio |
| Command | `npx` |
| Args | `-y @geek-fun/data-studio-mcp` |

## Usage

Once connected, just ask in plain language. The agent calls the database tools for you:

- "List all tables in my PostgreSQL database"
- "Show me the last 10 orders from the Elasticsearch index `orders*`"
- "Find all users older than 30 in MongoDB"
- "Run this query and explain the results"

The agent can read schemas, run queries, and explore your data — and it will show you every query it runs.

## Safety

- The bridge binds to `127.0.0.1` only — unreachable from other machines
- **Destructive and elevated operations are rejected by the bridge** — only read-safe capabilities are exposed through the MCP server
- Credentials are never exposed to the agent; all connections are resolved inside the desktop apps

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

The MCP server is a thin routing layer. All database drivers, SSH tunnels, and connection management live in the desktop apps, which expose a local HTTP bridge (`127.0.0.1` only). The MCP server auto-discovers running backends via each app's port file — only the tools of running backends are exposed.

## Tool naming

All tools follow the `data_studio__{backend}__{action}` convention:

| Prefix | Backend | Examples |
|---|---|---|
| `data_studio__sql_*` | sqlkit | `data_studio__sql_execute`, `data_studio__sql_list_tables` |
| `data_studio__es_*` | dockit | `data_studio__es_search`, `data_studio__es_list_indices` |
| `data_studio__mongo_*` | dockit | `data_studio__mongo_find`, `data_studio__mongo_insert` |
| `data_studio__dynamo_*` | dockit | `data_studio__dynamo_query`, `data_studio__dynamo_list_tables` |

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
