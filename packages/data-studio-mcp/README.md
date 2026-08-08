<div align="center">

<img src="https://raw.githubusercontent.com/geek-fun/data-studio-agent/master/docs/images/data-studio-agent.svg" width="96" height="96" alt="Data Studio Agent logo" />

# @geek-fun/data-studio-mcp

**Turn your AI coding agent (Claude Code, Cursor, Windsurf, OpenCode, Codex) into a database assistant — query, explore, and understand your databases in plain language.**

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

Turn your AI coding agent (Claude Code, Cursor, Windsurf, OpenCode, Codex) into a database assistant. Ask it to query your databases, explore schemas, and write SQL or NoSQL queries — it works with:

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
