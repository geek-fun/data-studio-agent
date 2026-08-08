<div align="center">

<img src="https://raw.githubusercontent.com/geek-fun/data-studio-agent/master/docs/images/data-studio-agent.svg" width="96" height="96" alt="Data Studio Agent logo" />

# @geek-fun/data-studio-mcp

**Let your AI coding agent (Claude Code, Cursor, Windsurf, OpenCode, Codex) securely access all your databases — query, explore, and understand your data in plain language.**

**Local-first. Enterprise-grade security. Open source.**

[![npm version](https://img.shields.io/npm/v/@geek-fun/data-studio-mcp?color=orange&logo=npm)](https://www.npmjs.com/package/@geek-fun/data-studio-mcp)
[![Downloads](https://img.shields.io/npm/dt/@geek-fun/data-studio-mcp?color=orange&logo=npm)](https://www.npmjs.com/package/@geek-fun/data-studio-mcp)
[![License](https://img.shields.io/badge/License-Apache_2.0-blue.svg&logo=apache)](https://github.com/geek-fun/data-studio-agent/blob/master/LICENSE)

<p>
  <img src="https://img.shields.io/badge/SQL-70%2B%20databases%20via%20SqlKit-336791"/>
  <img src="https://img.shields.io/badge/NoSQL-Elasticsearch%20%7C%20OpenSearch%20%7C%20MongoDB%20%7C%20DynamoDB-47A248"/>
  <img src="https://img.shields.io/badge/MCP-000000&logo=modelcontextprotocol&logoColor=white"/>
  <img src="https://img.shields.io/badge/TypeScript-3178C6?logo=typescript&logoColor=white"/>
</p>

[📖 Product Page](https://www.geekfun.club/products/data-studio-agent/) · [npm](https://www.npmjs.com/package/@geek-fun/data-studio-mcp) · [dockit](https://github.com/geek-fun/dockit) · [sqlkit](https://github.com/geek-fun/sqlkit) · [Releases](https://github.com/geek-fun/data-studio-agent/releases)

English · [简体中文](README_zh.md)

</div>

---

Let your AI coding agent (Claude Code, Cursor, Windsurf, OpenCode, Codex) securely access all your databases. Ask it to query your databases, explore schemas, and write SQL or NoSQL queries — it works with:

- **SQL databases** (via [sqlkit](https://github.com/geek-fun/sqlkit)): **70+ databases** — PostgreSQL, MySQL, SQL Server, Oracle, SQLite, DuckDB, ClickHouse, Snowflake, BigQuery, and more
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

## Enterprise-grade security

Designed for security-first teams. The LLM is a privileged-but-contained actor: it can do a lot with your data, but it can never obtain your credentials.

- **Credentials never leave the apps** — the LLM only ever sees an opaque `connection_id`; real credentials are resolved inside dockit/sqlkit and never cross the MCP boundary. Your passwords and keys stay on your machine, in your app.
- **ID-based resource access** — agents access databases strictly by connection ID, never by embedding credentials in prompts or tool arguments. There is no path for the model to obtain or exfiltrate connection secrets.
- **Three-tier permission model** — Read Only / Data Read-Write / Full Access modes gate every capability by risk level. Plus per-connection overrides: mark any connection read-only, or allowlist specific actions.
- **Explicit user confirmation** — destructive operations (DELETE, DROP, TRUNCATE) surface as `Ask` in the policy — the client prompts the user for explicit confirmation before anything destructive executes. Nothing destructive runs silently.
- **Action-level statement classification** — SQL is parsed and classified by statement kind (Read / Write / Delete / DDL) before execution. Write-only tools reject DELETE statements; delete tools reject DDL — no accidental escalation.
- **Local-only bridge** — the bridge binds to `127.0.0.1` exclusively — unreachable from other machines. A thin routing layer with no server to host, no API keys to manage, nothing exposed to the network.

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

All tools follow the `data_studio__{backend}__{action}` convention — the MCP server prefixes every bridge tool with `data_studio__`. Risk levels (🟢 Safe / 🟡 Elevated / 🔴 Destructive) gate exposure via the permission model: Safe tools run under Read Only; Elevated requires Data Read-Write; Destructive requires Full Access + explicit confirmation.

### MCP server tools (always available)

| Tool | Risk | Purpose |
|---|---|---|
| `data_studio__list_connections` | 🟢 Safe | List available connections (id, name, type) — no credentials |
| `data_studio__get_status` | 🟢 Safe | Backend availability, tool counts, permission state |

### SQL via sqlkit (11 tools)

| Tool | Risk | Purpose |
|---|---|---|
| `data_studio__sqlkit__list_connections` | 🟢 Safe | List sqlkit connections |
| `data_studio__sqlkit__execute_query` | 🟢 Safe | Execute a read-only SELECT query |
| `data_studio__sqlkit__list_databases` | 🟢 Safe | List databases |
| `data_studio__sqlkit__list_schemas` | 🟢 Safe | List schemas in a database |
| `data_studio__sqlkit__list_tables` | 🟢 Safe | List tables in a schema |
| `data_studio__sqlkit__get_schema` | 🟢 Safe | Get full schema for a database |
| `data_studio__sqlkit__describe_table` | 🟢 Safe | Describe a table's columns |
| `data_studio__sqlkit__explain_query` | 🟢 Safe | Explain a query's execution plan |
| `data_studio__sqlkit__execute_write` | 🟡 Elevated | INSERT / UPDATE / CREATE (no DELETE, no DDL) |
| `data_studio__sqlkit__execute_delete` | 🔴 Destructive | DELETE / DROP / TRUNCATE statements |
| `data_studio__sqlkit__execute_ddl` | 🔴 Destructive | DDL statements (ALTER, DROP TABLE, etc.) |

### NoSQL via dockit — Elasticsearch (16 tools)

| Tool | Risk | Purpose |
|---|---|---|
| `data_studio__es__search` | 🟢 Safe | Search using Query DSL |
| `data_studio__es__get_document` | 🟢 Safe | Get a document by ID |
| `data_studio__es__cat_indices` | 🟢 Safe | List indices |
| `data_studio__es__get_mapping` | 🟢 Safe | Get index mapping |
| `data_studio__es__cat_aliases` | 🟢 Safe | List aliases |
| `data_studio__es__get_alias` | 🟢 Safe | Get an alias |
| `data_studio__es__index_document` | 🟡 Elevated | Create or replace a document |
| `data_studio__es__update_document` | 🟡 Elevated | Partial update of a document |
| `data_studio__es__create_index` | 🟡 Elevated | Create an index |
| `data_studio__es__put_mapping` | 🟡 Elevated | Update index mapping |
| `data_studio__es__put_alias` | 🟡 Elevated | Create an alias |
| `data_studio__es__update_aliases` | 🟡 Elevated | Bulk alias operations |
| `data_studio__es__delete_document` | 🔴 Destructive | Delete a document |
| `data_studio__es__delete_by_query` | 🔴 Destructive | Delete documents matching a query |
| `data_studio__es__delete_index` | 🔴 Destructive | Delete an index |
| `data_studio__es__delete_alias` | 🔴 Destructive | Delete an alias |

### NoSQL via dockit — MongoDB (26 tools)

| Tool | Risk | Purpose |
|---|---|---|
| `data_studio__mongo__list_databases` | 🟢 Safe | List databases |
| `data_studio__mongo__list_collections` | 🟢 Safe | List collections |
| `data_studio__mongo__find` | 🟢 Safe | Find documents |
| `data_studio__mongo__collection_stats` | 🟢 Safe | Collection statistics |
| `data_studio__mongo__database_stats` | 🟢 Safe | Database statistics |
| `data_studio__mongo__server_status` | 🟢 Safe | Server status |
| `data_studio__mongo__repl_set_status` | 🟢 Safe | Replica set status |
| `data_studio__mongo__shard_status` | 🟢 Safe | Shard status |
| `data_studio__mongo__count_documents` | 🟢 Safe | Count documents |
| `data_studio__mongo__list_indexes` | 🟢 Safe | List indexes |
| `data_studio__mongo__sample_documents` | 🟢 Safe | Sample documents |
| `data_studio__mongo__aggregate` | 🟡 Elevated | Aggregation pipeline |
| `data_studio__mongo__insert_one` | 🟡 Elevated | Insert one document |
| `data_studio__mongo__update_many` | 🟡 Elevated | Update many documents |
| `data_studio__mongo__update_document` | 🟡 Elevated | Update one document |
| `data_studio__mongo__create_database` | 🟡 Elevated | Create a database |
| `data_studio__mongo__create_collection` | 🟡 Elevated | Create a collection |
| `data_studio__mongo__rename_collection` | 🟡 Elevated | Rename a collection |
| `data_studio__mongo__clone_collection` | 🟡 Elevated | Clone a collection |
| `data_studio__mongo__create_index` | 🟡 Elevated | Create an index |
| `data_studio__mongo__drop_index` | 🟡 Elevated | Drop an index |
| `data_studio__mongo__delete_many` | 🔴 Destructive | Delete many documents |
| `data_studio__mongo__delete_document` | 🔴 Destructive | Delete one document |
| `data_studio__mongo__drop_collection` | 🔴 Destructive | Drop a collection |
| `data_studio__mongo__drop_database` | 🔴 Destructive | Drop a database |
| `data_studio__mongo__truncate_collection` | 🔴 Destructive | Truncate a collection |

### NoSQL via dockit — DynamoDB (24 tools)

| Tool | Risk | Purpose |
|---|---|---|
| `data_studio__dynamo__execute_query` | 🟢 Safe | Execute a read query |
| `data_studio__dynamo__describe_table` | 🟢 Safe | Describe a table |
| `data_studio__dynamo__list_tables` | 🟢 Safe | List tables |
| `data_studio__dynamo__query_table` | 🟢 Safe | Query a table |
| `data_studio__dynamo__scan_table` | 🟢 Safe | Scan a table |
| `data_studio__dynamo__describe_continuous_backups` | 🟢 Safe | Describe backups |
| `data_studio__dynamo__describe_ttl` | 🟢 Safe | Describe TTL |
| `data_studio__dynamo__get_table_metrics` | 🟢 Safe | Table metrics |
| `data_studio__dynamo__execute_write` | 🟡 Elevated | Execute a write |
| `data_studio__dynamo__create_item` | 🟡 Elevated | Create an item |
| `data_studio__dynamo__batch_write_items` | 🟡 Elevated | Batch write items |
| `data_studio__dynamo__update_item` | 🟡 Elevated | Update an item |
| `data_studio__dynamo__create_gsi` | 🟡 Elevated | Create a GSI |
| `data_studio__dynamo__update_gsi` | 🟡 Elevated | Update a GSI |
| `data_studio__dynamo__create_table` | 🟡 Elevated | Create a table |
| `data_studio__dynamo__update_table_config` | 🟡 Elevated | Update table config |
| `data_studio__dynamo__update_ttl` | 🟡 Elevated | Update TTL |
| `data_studio__dynamo__update_pitr` | 🟡 Elevated | Update PITR |
| `data_studio__dynamo__update_streams` | 🟡 Elevated | Update streams |
| `data_studio__dynamo__execute_delete` | 🔴 Destructive | Execute a delete |
| `data_studio__dynamo__delete_item` | 🔴 Destructive | Delete an item |
| `data_studio__dynamo__delete_gsi` | 🔴 Destructive | Delete a GSI |
| `data_studio__dynamo__delete_table` | 🔴 Destructive | Delete a table |
| `data_studio__dynamo__truncate_table` | 🔴 Destructive | Truncate a table |

> **Total: 79 tools** (2 server + 11 SQL + 16 Elasticsearch + 26 MongoDB + 24 DynamoDB). Destructive tools (🔴) only appear when the bridge permission mode is **Full Access** with **Confirm Destructive** enabled — otherwise they are hidden from `tools/list` entirely.

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
