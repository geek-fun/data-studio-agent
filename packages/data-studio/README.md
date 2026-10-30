# Data Studio — ZCode Plugin

**Let your AI coding agent securely access all your databases, in plain language.**

A ZCode plugin that bundles the [Data Studio MCP server](https://www.npmjs.com/package/@geek-fun/data-studio-mcp) (`data_studio__*` tools, 116 total) with agent usage guidance.

- **SQL** (via [sqlkit](https://github.com/geek-fun/sqlkit)): 70+ databases — PostgreSQL, MySQL, SQL Server, Oracle, SQLite, DuckDB, ClickHouse, Snowflake, BigQuery, and more.
- **NoSQL** (via [dockit](https://github.com/geek-fun/dockit)): Elasticsearch, OpenSearch, MongoDB, DynamoDB.

## Prerequisites

1. Install and launch [dockit](https://github.com/geek-fun/dockit) and/or [sqlkit](https://github.com/geek-fun/sqlkit); add a database connection.
2. Keep **Settings → MCP Bridge → Auto-start** enabled in the desktop apps (on by default).
3. Node.js ≥ 18 on PATH (the MCP server is started via `npx -y @geek-fun/data-studio-mcp`).

## Disclosure

| Item | Detail |
|---|---|
| MCP server | One stdio server: `npx -y @geek-fun/data-studio-mcp` (npm package `@geek-fun/data-studio-mcp`) |
| Network access | `127.0.0.1` only — talks to the dockit/sqlkit local bridges (ports 9120/9121). npm registry access during the first `npx` download. No other endpoints; no telemetry. |
| Credentials | Never collected, stored, or transmitted. The LLM only sees opaque `connection_id`s; real credentials are resolved inside the desktop apps. |
| Command execution | Spawns the MCP server process (Node.js). No shell scripts, no hooks. |
| File writes | None by the plugin itself. |
| External services | None. |

## Security model

- **Credentials never leave the desktop apps.** Tools accept `connection_id` only.
- **Three-tier permissions** (configured in dockit/sqlkit): Read Only / Data Read-Write / Full Access.
- **Destructive operations** (DELETE, DROP, TRUNCATE) always surface an explicit confirmation prompt in the client.

## License

Apache 2.0. See [LICENSE](../../../LICENSE) in the repository root.

---

# Data Studio — ZCode 插件

**让 AI 编码助手用自然语言安全地访问你的所有数据库。**

本插件打包了 [Data Studio MCP server](https://www.npmjs.com/package/@geek-fun/data-studio-mcp)(116 个 `data_studio__*` 工具)与 agent 使用指引。

- **SQL**(经 [sqlkit](https://github.com/geek-fun/sqlkit)):70+ 种数据库 — PostgreSQL、MySQL、SQL Server、Oracle、SQLite、DuckDB、ClickHouse、Snowflake、BigQuery 等。
- **NoSQL**(经 [dockit](https://github.com/geek-fun/dockit)):Elasticsearch、OpenSearch、MongoDB、DynamoDB。

## 前置条件

1. 安装并启动 [dockit](https://github.com/geek-fun/dockit) 和/或 [sqlkit](https://github.com/geek-fun/sqlkit),并添加数据库连接。
2. 保持桌面应用 **Settings → MCP Bridge → Auto-start** 开启(默认开启)。
3. PATH 中需有 Node.js ≥ 18(MCP server 通过 `npx -y @geek-fun/data-studio-mcp` 启动)。

## 披露信息

| 项目 | 说明 |
|---|---|
| MCP server | 一个 stdio server:`npx -y @geek-fun/data-studio-mcp`(npm 包 `@geek-fun/data-studio-mcp`) |
| 网络访问 | 仅 `127.0.0.1` — 与 dockit/sqlkit 本地 bridge 通信(端口 9120/9121);首次 `npx` 下载会访问 npm registry;无其他端点,无遥测 |
| 凭据 | 不收集、不存储、不传输;LLM 只能看到不透明的 `connection_id`,真实凭据在桌面应用内部解析 |
| 命令执行 | 仅启动 MCP server 进程(Node.js);无 shell 脚本,无 hooks |
| 文件写入 | 插件本身不写入任何文件 |
| 外部服务 | 无 |

## 安全模型

- **凭据永不离开桌面应用**:工具只接受 `connection_id`。
- **三级权限**(在 dockit/sqlkit 中配置):Read Only / Data Read-Write / Full Access。
- **破坏性操作**(DELETE、DROP、TRUNCATE)始终在客户端弹出显式确认。
