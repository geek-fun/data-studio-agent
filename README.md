<div align="center">

<img src="docs/images/data-studio-agent.svg" width="96" height="96" alt="Data Studio Agent logo" />

# Data Studio Agent

**Turn your AI coding agent into a database assistant — query, explore, and understand your databases in plain language.**

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

All tools follow the `data_studio__{backend}__{action}` convention:

| Prefix | Backend | Examples |
|---|---|---|
| `data_studio__sql_*` | sqlkit | `data_studio__sql_execute`, `data_studio__sql_list_tables` |
| `data_studio__es_*` | dockit | `data_studio__es_search`, `data_studio__es_list_indices` |
| `data_studio__mongo_*` | dockit | `data_studio__mongo_find`, `data_studio__mongo_insert` |
| `data_studio__dynamo_*` | dockit | `data_studio__dynamo_query`, `data_studio__dynamo_list_tables` |

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
