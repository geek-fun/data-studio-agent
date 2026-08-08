<div align="center">

<img src="docs/images/data-studio-agent.svg" width="96" height="96" alt="Data Studio Agent logo" />

# Data Studio Agent

**Let your AI coding agent securely access all your databases — query, explore, and understand your data in plain language.**

**Local-first. Enterprise-grade security. Open source.**

[![Release](https://img.shields.io/github/v/release/geek-fun/data-studio-agent?color=orange&label=release&logo=github)](https://github.com/geek-fun/data-studio-agent/releases)
[![Downloads](https://img.shields.io/github/downloads/geek-fun/data-studio-agent/total?color=orange&logo=docusign)](https://github.com/geek-fun/data-studio-agent/releases)
[![npm](https://img.shields.io/npm/dt/@geek-fun/data-studio-mcp?color=orange&logo=npm)](https://www.npmjs.com/package/@geek-fun/data-studio-mcp)
[![License](https://img.shields.io/badge/License-Apache_2.0-blue.svg&logo=apache)](LICENSE)
[![Stars](https://img.shields.io/github/stars/geek-fun/data-studio-agent&logo=github)](https://github.com/geek-fun/data-studio-agent/stargazers)
[![CI](https://github.com/geek-fun/data-studio-agent/actions/workflows/ci.yml/badge.svg)](https://github.com/geek-fun/data-studio-agent/actions/workflows/ci.yml)

<p>
  <img src="https://img.shields.io/badge/SQL-70%2B%20databases%20via%20SqlKit-336791"/>
  <img src="https://img.shields.io/badge/NoSQL-Elasticsearch%20%7C%20OpenSearch%20%7C%20MongoDB%20%7C%20DynamoDB-47A248"/>
  <img src="https://img.shields.io/badge/MCP-000000&logo=modelcontextprotocol&logoColor=white"/>
  <img src="https://img.shields.io/badge/Claude%20Code%20%7C%20Cursor%20%7C%20OpenCode%20%7C%20Codex%20%7C%20Cline-7C3AED"/>
</p>

[📖 Product Page](https://www.geekfun.club/products/data-studio-agent/) · [npm](https://www.npmjs.com/package/@geek-fun/data-studio-mcp) · [dockit](https://github.com/geek-fun/dockit) · [sqlkit](https://github.com/geek-fun/sqlkit) · [Releases](https://github.com/geek-fun/data-studio-agent/releases)

English · [简体中文](README_zh.md)

</div>

---

This repository is home to the **Data Studio MCP Server** — a unified [Model Context Protocol](https://modelcontextprotocol.io/) server that gives AI coding agents direct access to your databases through the [dockit](https://github.com/geek-fun/dockit) and [sqlkit](https://github.com/geek-fun/sqlkit) desktop apps.

- **SQL** (via sqlkit): **70+ databases** — PostgreSQL, MySQL, SQL Server, Oracle, SQLite, DuckDB, ClickHouse, Snowflake, BigQuery, and more
- **NoSQL** (via dockit): Elasticsearch, OpenSearch, MongoDB, DynamoDB

## Features

- **Any AI coding agent** — Claude Code, Cursor, Windsurf, OpenCode, Codex, Cline, Pi, Qoder, GitHub Copilot, and any MCP client
- **Any OS** — macOS, Windows, Linux
- **Any LLM model** — bring your own provider, no lock-in
- **One MCP server, one config** — routes to both SqlKit (SQL) and DocKit (NoSQL) bridges over localhost
- **Enterprise-grade security** — see below

## Quick start

### 1. Prerequisites

Install and launch [dockit](https://github.com/geek-fun/dockit) and/or [sqlkit](https://github.com/geek-fun/sqlkit), add a database connection, and make sure **Settings → MCP Bridge → Auto-start** is enabled (default). Install both apps for the full SQL + NoSQL tool set.

### 2. Install the MCP server

```bash
npm install -g @geek-fun/data-studio-mcp
```

Or run without installing (npx downloads on first run):

```bash
npx -y @geek-fun/data-studio-mcp
```

### 3. Add it to your AI tool

**OpenAI Codex** — one command:

```bash
codex mcp add data-studio -- npx -y @geek-fun/data-studio-mcp
```

**Claude Code** — one command:

```bash
claude mcp add --transport stdio data-studio -- npx -y @geek-fun/data-studio-mcp
```

**Cursor** — create `.cursor/mcp.json` (project) or `~/.cursor/mcp.json` (global):

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

**Windsurf** — create `~/.codeium/windsurf/mcp_config.json` (global only):

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

**OpenCode** — add to `opencode.json` (project) or `~/.config/opencode/opencode.json` (global):

```json
{
  "$schema": "https://opencode.ai/config.json",
  "mcp": {
    "data-studio": {
      "type": "local",
      "command": ["npx", "-y", "@geek-fun/data-studio-mcp"],
      "enabled": true
    }
  }
}
```

**Any other MCP client** — register a stdio server with command `npx` and args `-y @geek-fun/data-studio-mcp`.

### 4. Tune permissions (optional)

Open **Settings → MCP Bridge** in dockit/sqlkit to control what the agent can do:

| Permission mode | What the agent can do |
|---|---|
| **Read Only** (default) | Explore schemas, run SELECT queries. No writes. |
| **Data Read/Write** | INSERT, UPDATE, index operations. No deletes/drops. |
| **Full Access** | Everything, including DELETE, DROP, TRUNCATE. |

### 5. Start asking

Just use plain language — the agent queries your databases for you:

- "List all tables in my PostgreSQL database"
- "Show me the last 10 orders from the Elasticsearch index `orders*`"
- "Find all users older than 30 in MongoDB"
- "Run this query and explain the results"

The agent reads schemas, runs queries, and explores your data — and shows you every query it executes.

## Enterprise-grade security

Designed for security-first teams. The LLM is a privileged-but-contained actor: it can do a lot with your data, but it can never obtain your credentials.

- **Credentials never leave the apps** — the LLM only ever sees an opaque `connection_id`; real credentials are resolved inside dockit/sqlkit and never cross the MCP boundary. Your passwords and keys stay on your machine, in your app.
- **ID-based resource access** — agents access databases strictly by connection ID, never by embedding credentials in prompts or tool arguments. There is no path for the model to obtain or exfiltrate connection secrets.
- **Three-tier permission model** — Read Only / Data Read-Write / Full Access modes gate every capability by risk level. Plus per-connection overrides: mark any connection read-only, or allowlist specific actions.
- **Explicit user confirmation** — destructive operations (DELETE, DROP, TRUNCATE) surface as `Ask` in the policy — the client prompts the user for explicit confirmation before anything destructive executes. Nothing destructive runs silently.
- **Action-level statement classification** — SQL is parsed and classified by statement kind (Read / Write / Delete / DDL) before execution. Write-only tools reject DELETE statements; delete tools reject DDL — no accidental escalation.
- **Local-only bridge** — the bridge binds to `127.0.0.1` exclusively — unreachable from other machines. A thin routing layer with no server to host, no API keys to manage, nothing exposed to the network.

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

## How it works

```
code agent (Claude Code / Cursor / OpenCode ...)
    |
    | MCP stdio protocol
    v
@geek-fun/data-studio-mcp   ← npm package (pure TypeScript)
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

The MCP server is a thin routing layer. All database drivers, SSH tunnels, and connection management live in the desktop apps, which expose a local HTTP bridge (`127.0.0.1` only). The MCP server auto-discovers running backends via each app's port file.

---

## For developers

Under the hood, this repository also contains the **data-studio-agent Rust framework** — the shared AI agent loop (provider adapters, streaming, tool calling, context compaction) that powers the built-in assistants in [dockit](https://github.com/geek-fun/dockit) and [sqlkit](https://github.com/geek-fun/sqlkit).

See [crates/data-studio-agent/README.md](crates/data-studio-agent/README.md) for installation, architecture, and integration guides.

## License

Apache 2.0 — see [LICENSE](LICENSE).
