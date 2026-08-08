<div align="center">

<img src="https://raw.githubusercontent.com/geek-fun/data-studio-agent/master/docs/images/data-studio-agent.svg" width="96" height="96" alt="Data Studio Agent logo" />

# @geek-fun/data-studio-mcp

**Let your AI coding agent (Claude Code, Cursor, Windsurf, OpenCode, Codex) securely access all your databases, in plain language.**

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

Let your AI coding agent (Claude Code, Cursor, Windsurf, OpenCode, Codex) securely access all your databases. Ask it to query, explore schemas, and write SQL or NoSQL queries. It works with:

- **SQL databases** (via [sqlkit](https://github.com/geek-fun/sqlkit)): **70+ databases** (PostgreSQL, MySQL, SQL Server, Oracle, SQLite, DuckDB, ClickHouse, Snowflake, BigQuery, and more)
- **NoSQL databases** (via [dockit](https://github.com/geek-fun/dockit)): Elasticsearch, OpenSearch, MongoDB, DynamoDB

If you already have the desktop apps installed, setup takes two steps: install this package and add it to your AI tool. There is no server to host and no API keys to manage. Everything runs locally on your machine.

## Prerequisites

1. Install [dockit](https://github.com/geek-fun/dockit) and/or [sqlkit](https://github.com/geek-fun/sqlkit) (free, open source)
2. Launch the app and add at least one database connection
3. Make sure **Settings → MCP Bridge → Auto-start** is enabled (it is by default)

Install both apps to get the full tool set, SQL and NoSQL. One app is enough to get started.

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

The agent can read schemas, run queries, and explore your data, then shows you every query it runs.

## Enterprise-grade security

The LLM gets broad access to your data, but it never sees your credentials. The policy model gates every capability by risk level.

- **Credentials never leave the apps.** The LLM only ever sees an opaque `connection_id`. Real credentials are resolved inside dockit/sqlkit and never cross the MCP boundary. Your passwords and keys stay on your machine, in your app.
- **ID-based resource access.** Agents access databases strictly by connection ID. Credentials never appear in prompts or tool arguments, so there is no path for the model to obtain or exfiltrate connection secrets.
- **Three-tier permission model.** Read Only / Data Read-Write / Full Access modes gate every capability by risk level, with per-connection overrides. You can mark any connection read-only or allowlist specific actions.
- **Explicit user confirmation.** Destructive operations (DELETE, DROP, TRUNCATE) surface as `Ask` in the policy. The client prompts the user for explicit confirmation before anything destructive runs.
- **Action-level statement classification.** SQL is parsed and classified by statement kind (Read / Write / Delete / DDL) before execution. Write-only tools reject DELETE statements; delete tools reject DDL.
- **Local-only bridge.** The bridge binds to `127.0.0.1` exclusively. It is unreachable from other machines, with no server to host and no API keys to manage.

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

The MCP server is a thin routing layer. All database drivers, SSH tunnels, and connection management live in the desktop apps, which expose a local HTTP bridge (`127.0.0.1` only). The MCP server auto-discovers running backends via each app's port file, so only the tools of running backends are exposed.

## Tool reference

All tools follow the `data_studio__{backend}__{action}` convention. The **User confirmation** column shows which operations surface an explicit confirmation prompt in your AI client before they run.

| Tool | Backend | Risk | Requires permission | User confirmation |
|---|---|---|---|---|
| `data_studio__list_connections` | Server | 🟢 Safe | Read Only | No |
| `data_studio__get_status` | Server | 🟢 Safe | Read Only | No |
| `data_studio__sqlkit__list_connections` | sqlkit | 🟢 Safe | Read Only | No |
| `data_studio__sqlkit__execute_query` | sqlkit | 🟢 Safe | Read Only | No |
| `data_studio__sqlkit__list_databases` | sqlkit | 🟢 Safe | Read Only | No |
| `data_studio__sqlkit__list_schemas` | sqlkit | 🟢 Safe | Read Only | No |
| `data_studio__sqlkit__list_tables` | sqlkit | 🟢 Safe | Read Only | No |
| `data_studio__sqlkit__get_schema` | sqlkit | 🟢 Safe | Read Only | No |
| `data_studio__sqlkit__describe_table` | sqlkit | 🟢 Safe | Read Only | No |
| `data_studio__sqlkit__explain_query` | sqlkit | 🟢 Safe | Read Only | No |
| `data_studio__sqlkit__execute_write` | sqlkit | 🟡 Elevated | Data Read-Write | No |
| `data_studio__sqlkit__execute_delete` | sqlkit | 🔴 Destructive | Full Access | Yes |
| `data_studio__sqlkit__execute_ddl` | sqlkit | 🔴 Destructive | Full Access | Yes |
| `data_studio__es__search` | dockit · Elasticsearch | 🟢 Safe | Read Only | No |
| `data_studio__es__get_document` | dockit · Elasticsearch | 🟢 Safe | Read Only | No |
| `data_studio__es__cat_indices` | dockit · Elasticsearch | 🟢 Safe | Read Only | No |
| `data_studio__es__get_mapping` | dockit · Elasticsearch | 🟢 Safe | Read Only | No |
| `data_studio__es__cat_aliases` | dockit · Elasticsearch | 🟢 Safe | Read Only | No |
| `data_studio__es__get_alias` | dockit · Elasticsearch | 🟢 Safe | Read Only | No |
| `data_studio__es__index_document` | dockit · Elasticsearch | 🟡 Elevated | Data Read-Write | No |
| `data_studio__es__update_document` | dockit · Elasticsearch | 🟡 Elevated | Data Read-Write | No |
| `data_studio__es__create_index` | dockit · Elasticsearch | 🟡 Elevated | Data Read-Write | No |
| `data_studio__es__put_mapping` | dockit · Elasticsearch | 🟡 Elevated | Data Read-Write | No |
| `data_studio__es__put_alias` | dockit · Elasticsearch | 🟡 Elevated | Data Read-Write | No |
| `data_studio__es__update_aliases` | dockit · Elasticsearch | 🟡 Elevated | Data Read-Write | No |
| `data_studio__es__delete_document` | dockit · Elasticsearch | 🔴 Destructive | Full Access | Yes |
| `data_studio__es__delete_by_query` | dockit · Elasticsearch | 🔴 Destructive | Full Access | Yes |
| `data_studio__es__delete_index` | dockit · Elasticsearch | 🔴 Destructive | Full Access | Yes |
| `data_studio__es__delete_alias` | dockit · Elasticsearch | 🔴 Destructive | Full Access | Yes |
| `data_studio__mongo__list_databases` | dockit · MongoDB | 🟢 Safe | Read Only | No |
| `data_studio__mongo__list_collections` | dockit · MongoDB | 🟢 Safe | Read Only | No |
| `data_studio__mongo__find` | dockit · MongoDB | 🟢 Safe | Read Only | No |
| `data_studio__mongo__collection_stats` | dockit · MongoDB | 🟢 Safe | Read Only | No |
| `data_studio__mongo__database_stats` | dockit · MongoDB | 🟢 Safe | Read Only | No |
| `data_studio__mongo__server_status` | dockit · MongoDB | 🟢 Safe | Read Only | No |
| `data_studio__mongo__repl_set_status` | dockit · MongoDB | 🟢 Safe | Read Only | No |
| `data_studio__mongo__shard_status` | dockit · MongoDB | 🟢 Safe | Read Only | No |
| `data_studio__mongo__count_documents` | dockit · MongoDB | 🟢 Safe | Read Only | No |
| `data_studio__mongo__list_indexes` | dockit · MongoDB | 🟢 Safe | Read Only | No |
| `data_studio__mongo__sample_documents` | dockit · MongoDB | 🟢 Safe | Read Only | No |
| `data_studio__mongo__aggregate` | dockit · MongoDB | 🟡 Elevated | Data Read-Write | No |
| `data_studio__mongo__insert_one` | dockit · MongoDB | 🟡 Elevated | Data Read-Write | No |
| `data_studio__mongo__update_many` | dockit · MongoDB | 🟡 Elevated | Data Read-Write | No |
| `data_studio__mongo__update_document` | dockit · MongoDB | 🟡 Elevated | Data Read-Write | No |
| `data_studio__mongo__create_database` | dockit · MongoDB | 🟡 Elevated | Data Read-Write | No |
| `data_studio__mongo__create_collection` | dockit · MongoDB | 🟡 Elevated | Data Read-Write | No |
| `data_studio__mongo__rename_collection` | dockit · MongoDB | 🟡 Elevated | Data Read-Write | No |
| `data_studio__mongo__clone_collection` | dockit · MongoDB | 🟡 Elevated | Data Read-Write | No |
| `data_studio__mongo__create_index` | dockit · MongoDB | 🟡 Elevated | Data Read-Write | No |
| `data_studio__mongo__drop_index` | dockit · MongoDB | 🟡 Elevated | Data Read-Write | No |
| `data_studio__mongo__delete_many` | dockit · MongoDB | 🔴 Destructive | Full Access | Yes |
| `data_studio__mongo__delete_document` | dockit · MongoDB | 🔴 Destructive | Full Access | Yes |
| `data_studio__mongo__drop_collection` | dockit · MongoDB | 🔴 Destructive | Full Access | Yes |
| `data_studio__mongo__drop_database` | dockit · MongoDB | 🔴 Destructive | Full Access | Yes |
| `data_studio__mongo__truncate_collection` | dockit · MongoDB | 🔴 Destructive | Full Access | Yes |
| `data_studio__dynamo__execute_query` | dockit · DynamoDB | 🟢 Safe | Read Only | No |
| `data_studio__dynamo__describe_table` | dockit · DynamoDB | 🟢 Safe | Read Only | No |
| `data_studio__dynamo__list_tables` | dockit · DynamoDB | 🟢 Safe | Read Only | No |
| `data_studio__dynamo__query_table` | dockit · DynamoDB | 🟢 Safe | Read Only | No |
| `data_studio__dynamo__scan_table` | dockit · DynamoDB | 🟢 Safe | Read Only | No |
| `data_studio__dynamo__describe_continuous_backups` | dockit · DynamoDB | 🟢 Safe | Read Only | No |
| `data_studio__dynamo__describe_ttl` | dockit · DynamoDB | 🟢 Safe | Read Only | No |
| `data_studio__dynamo__get_table_metrics` | dockit · DynamoDB | 🟢 Safe | Read Only | No |
| `data_studio__dynamo__execute_write` | dockit · DynamoDB | 🟡 Elevated | Data Read-Write | No |
| `data_studio__dynamo__create_item` | dockit · DynamoDB | 🟡 Elevated | Data Read-Write | No |
| `data_studio__dynamo__batch_write_items` | dockit · DynamoDB | 🟡 Elevated | Data Read-Write | No |
| `data_studio__dynamo__update_item` | dockit · DynamoDB | 🟡 Elevated | Data Read-Write | No |
| `data_studio__dynamo__create_gsi` | dockit · DynamoDB | 🟡 Elevated | Data Read-Write | No |
| `data_studio__dynamo__update_gsi` | dockit · DynamoDB | 🟡 Elevated | Data Read-Write | No |
| `data_studio__dynamo__create_table` | dockit · DynamoDB | 🟡 Elevated | Data Read-Write | No |
| `data_studio__dynamo__update_table_config` | dockit · DynamoDB | 🟡 Elevated | Data Read-Write | No |
| `data_studio__dynamo__update_ttl` | dockit · DynamoDB | 🟡 Elevated | Data Read-Write | No |
| `data_studio__dynamo__update_pitr` | dockit · DynamoDB | 🟡 Elevated | Data Read-Write | No |
| `data_studio__dynamo__update_streams` | dockit · DynamoDB | 🟡 Elevated | Data Read-Write | No |
| `data_studio__dynamo__execute_delete` | dockit · DynamoDB | 🔴 Destructive | Full Access | Yes |
| `data_studio__dynamo__delete_item` | dockit · DynamoDB | 🔴 Destructive | Full Access | Yes |
| `data_studio__dynamo__delete_gsi` | dockit · DynamoDB | 🔴 Destructive | Full Access | Yes |
| `data_studio__dynamo__delete_table` | dockit · DynamoDB | 🔴 Destructive | Full Access | Yes |
| `data_studio__dynamo__truncate_table` | dockit · DynamoDB | 🔴 Destructive | Full Access | Yes |

**79 tools total.** Read-only operations run automatically under **Read Only** mode. Elevated operations (writes, index/schema changes) require **Data Read-Write**. Destructive operations (DELETE, DROP, TRUNCATE) require **Full Access** and always surface an explicit **user confirmation** prompt.

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
