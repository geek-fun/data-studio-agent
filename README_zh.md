<div align="center">

<img src="docs/images/data-studio-agent.svg" width="96" height="96" alt="Data Studio Agent logo" />

# Data Studio Agent

**让你的 AI 编程智能体安全访问所有数据库，一切用自然语言。**

**本地优先。企业级安全。开源开放。**

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

[📖 产品页](https://www.geekfun.club/zh/products/data-studio-agent/) · [npm](https://www.npmjs.com/package/@geek-fun/data-studio-mcp) · [dockit](https://github.com/geek-fun/dockit) · [sqlkit](https://github.com/geek-fun/sqlkit) · [Releases](https://github.com/geek-fun/data-studio-agent/releases)

[English](README.md) · 简体中文

</div>

---

本仓库包含 **Data Studio MCP Server**，一个 [Model Context Protocol](https://modelcontextprotocol.io/) 服务器，通过 [dockit](https://github.com/geek-fun/dockit) 和 [sqlkit](https://github.com/geek-fun/sqlkit) 桌面应用，让你的 AI 编程智能体直接访问数据库。

- **SQL**（通过 sqlkit）：**70+ 种数据库**（PostgreSQL、MySQL、SQL Server、Oracle、SQLite、DuckDB、ClickHouse、Snowflake、BigQuery 等）
- **NoSQL**（通过 dockit）：Elasticsearch、OpenSearch、MongoDB、DynamoDB

## 特性

- **任意 AI 编程助手。** Claude Code、Cursor、Windsurf、OpenCode、Codex、Cline、Pi、Qoder、GitHub Copilot，或任何 MCP 客户端。
- **任意操作系统。** macOS、Windows、Linux。
- **任意 LLM 模型。** 自带 Provider，无锁定。
- **一个 MCP 服务器、一份配置。** 在本机同时路由到 SqlKit（SQL）和 DocKit（NoSQL）的桥接。
- **企业级安全。** 详见下文。

## 快速开始

### 1. 环境要求

安装并启动 [dockit](https://github.com/geek-fun/dockit) 和/或 [sqlkit](https://github.com/geek-fun/sqlkit)，添加数据库连接，确认 **设置 → MCP Bridge → 自动启动** 已开启（默认开启）。两个应用都装可获得完整的 SQL + NoSQL 工具集。

### 2. 安装 MCP 服务器

```bash
npm install -g @geek-fun/data-studio-mcp
```

或不安装直接运行（npx 首次运行自动下载）：

```bash
npx -y @geek-fun/data-studio-mcp
```

### 3. 添加到你的 AI 工具

**OpenAI Codex**，一条命令：

```bash
codex mcp add data-studio -- npx -y @geek-fun/data-studio-mcp
```

**Claude Code**，一条命令：

```bash
claude mcp add --transport stdio data-studio -- npx -y @geek-fun/data-studio-mcp
```

**Cursor。** 创建 `.cursor/mcp.json`（项目）或 `~/.cursor/mcp.json`（全局）：

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

**Windsurf。** 创建 `~/.codeium/windsurf/mcp_config.json`（仅全局）：

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

**OpenCode。** 添加到 `opencode.json`（项目）或 `~/.config/opencode/opencode.json`（全局）：

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

**任何其他 MCP 客户端。** 注册一个 stdio 服务器，命令 `npx`，参数 `-y @geek-fun/data-studio-mcp`。

### 4. 调整权限（可选）

在 dockit/sqlkit 中打开 **设置 → MCP Bridge**，控制代理能做什么：

| 权限模式 | 代理能做什么 |
|---|---|
| **只读**（默认） | 浏览 schema、运行 SELECT 查询。不可写。 |
| **数据读写** | INSERT、UPDATE、索引操作。不可删除/删表。 |
| **完全访问** | 一切操作，包括 DELETE、DROP、TRUNCATE。 |

### 5. 开始提问

直接用自然语言。AI 助手会替你查询数据库：

- "列出我 PostgreSQL 数据库中的所有表"
- "从 Elasticsearch 的 `orders*` 索引中找出最近 10 条订单"
- "在 MongoDB 中查找所有年龄大于 30 的用户"
- "执行这条查询并解释结果"

AI 助手可以查看表结构、运行查询、探索你的数据，然后展示它执行的每一条查询。

## 企业级安全

LLM 能对你的数据做很多事，但它永远看不到你的凭据。策略模型按风险等级管控每个能力。

- **凭据永不离开应用。** LLM 只能看到一个不透明的 `connection_id`。真实凭据在 dockit/sqlkit 内部解析，绝不跨越 MCP 边界。你的密码和密钥始终留在本机、留在应用里。
- **基于 ID 的资源访问。** 代理严格通过连接 ID 访问数据库。凭据不会出现在提示词或工具参数中，模型没有途径获取或泄露连接密钥。
- **三级权限模型。** 只读 / 数据读写 / 完全访问三种模式按风险等级管控每个能力，外加连接级覆盖。你可以将任意连接标记为只读，或按操作白名单放行。
- **显式用户确认。** 破坏性操作（DELETE、DROP、TRUNCATE）在策略中标记为 `Ask`。客户端在执行任何破坏性操作前都会弹出显式确认。
- **操作级语句分类。** SQL 在执行前按语句类型解析分类（读 / 写 / 删除 / DDL）。只写工具拒绝 DELETE 语句；删除工具拒绝 DDL。
- **仅本地桥接。** 桥接只绑定 `127.0.0.1`，其他机器无法访问，无需托管服务器、无需管理 API key。

## 工具参考

所有工具遵循 `data_studio__{backend}__{action}` 命名规则。**用户确认**列标明哪些操作会在你的 AI 客户端中弹出显式确认提示。

| 工具 | 后端 | 风险 | 所需权限 | 用户确认 |
|---|---|---|---|---|
| `data_studio__list_connections` | 服务器 | 🟢 安全 | 只读 | 否 |
| `data_studio__get_status` | 服务器 | 🟢 安全 | 只读 | 否 |
| `data_studio__sqlkit__list_connections` | sqlkit | 🟢 安全 | 只读 | 否 |
| `data_studio__sqlkit__execute_query` | sqlkit | 🟢 安全 | 只读 | 否 |
| `data_studio__sqlkit__list_databases` | sqlkit | 🟢 安全 | 只读 | 否 |
| `data_studio__sqlkit__list_schemas` | sqlkit | 🟢 安全 | 只读 | 否 |
| `data_studio__sqlkit__list_tables` | sqlkit | 🟢 安全 | 只读 | 否 |
| `data_studio__sqlkit__get_schema` | sqlkit | 🟢 安全 | 只读 | 否 |
| `data_studio__sqlkit__describe_table` | sqlkit | 🟢 安全 | 只读 | 否 |
| `data_studio__sqlkit__explain_query` | sqlkit | 🟢 安全 | 只读 | 否 |
| `data_studio__sqlkit__execute_write` | sqlkit | 🟡 提升 | 数据读写 | 否 |
| `data_studio__sqlkit__execute_delete` | sqlkit | 🔴 破坏性 | 完全访问 | 是 |
| `data_studio__sqlkit__execute_ddl` | sqlkit | 🔴 破坏性 | 完全访问 | 是 |
| `data_studio__es__search` | dockit · Elasticsearch | 🟢 安全 | 只读 | 否 |
| `data_studio__es__get_document` | dockit · Elasticsearch | 🟢 安全 | 只读 | 否 |
| `data_studio__es__cat_indices` | dockit · Elasticsearch | 🟢 安全 | 只读 | 否 |
| `data_studio__es__get_mapping` | dockit · Elasticsearch | 🟢 安全 | 只读 | 否 |
| `data_studio__es__cat_aliases` | dockit · Elasticsearch | 🟢 安全 | 只读 | 否 |
| `data_studio__es__get_alias` | dockit · Elasticsearch | 🟢 安全 | 只读 | 否 |
| `data_studio__es__index_document` | dockit · Elasticsearch | 🟡 提升 | 数据读写 | 否 |
| `data_studio__es__update_document` | dockit · Elasticsearch | 🟡 提升 | 数据读写 | 否 |
| `data_studio__es__create_index` | dockit · Elasticsearch | 🟡 提升 | 数据读写 | 否 |
| `data_studio__es__put_mapping` | dockit · Elasticsearch | 🟡 提升 | 数据读写 | 否 |
| `data_studio__es__put_alias` | dockit · Elasticsearch | 🟡 提升 | 数据读写 | 否 |
| `data_studio__es__update_aliases` | dockit · Elasticsearch | 🟡 提升 | 数据读写 | 否 |
| `data_studio__es__delete_document` | dockit · Elasticsearch | 🔴 破坏性 | 完全访问 | 是 |
| `data_studio__es__delete_by_query` | dockit · Elasticsearch | 🔴 破坏性 | 完全访问 | 是 |
| `data_studio__es__delete_index` | dockit · Elasticsearch | 🔴 破坏性 | 完全访问 | 是 |
| `data_studio__es__delete_alias` | dockit · Elasticsearch | 🔴 破坏性 | 完全访问 | 是 |
| `data_studio__mongo__list_databases` | dockit · MongoDB | 🟢 安全 | 只读 | 否 |
| `data_studio__mongo__list_collections` | dockit · MongoDB | 🟢 安全 | 只读 | 否 |
| `data_studio__mongo__find` | dockit · MongoDB | 🟢 安全 | 只读 | 否 |
| `data_studio__mongo__collection_stats` | dockit · MongoDB | 🟢 安全 | 只读 | 否 |
| `data_studio__mongo__database_stats` | dockit · MongoDB | 🟢 安全 | 只读 | 否 |
| `data_studio__mongo__server_status` | dockit · MongoDB | 🟢 安全 | 只读 | 否 |
| `data_studio__mongo__repl_set_status` | dockit · MongoDB | 🟢 安全 | 只读 | 否 |
| `data_studio__mongo__shard_status` | dockit · MongoDB | 🟢 安全 | 只读 | 否 |
| `data_studio__mongo__count_documents` | dockit · MongoDB | 🟢 安全 | 只读 | 否 |
| `data_studio__mongo__list_indexes` | dockit · MongoDB | 🟢 安全 | 只读 | 否 |
| `data_studio__mongo__sample_documents` | dockit · MongoDB | 🟢 安全 | 只读 | 否 |
| `data_studio__mongo__aggregate` | dockit · MongoDB | 🟡 提升 | 数据读写 | 否 |
| `data_studio__mongo__insert_one` | dockit · MongoDB | 🟡 提升 | 数据读写 | 否 |
| `data_studio__mongo__update_many` | dockit · MongoDB | 🟡 提升 | 数据读写 | 否 |
| `data_studio__mongo__update_document` | dockit · MongoDB | 🟡 提升 | 数据读写 | 否 |
| `data_studio__mongo__create_database` | dockit · MongoDB | 🟡 提升 | 数据读写 | 否 |
| `data_studio__mongo__create_collection` | dockit · MongoDB | 🟡 提升 | 数据读写 | 否 |
| `data_studio__mongo__rename_collection` | dockit · MongoDB | 🟡 提升 | 数据读写 | 否 |
| `data_studio__mongo__clone_collection` | dockit · MongoDB | 🟡 提升 | 数据读写 | 否 |
| `data_studio__mongo__create_index` | dockit · MongoDB | 🟡 提升 | 数据读写 | 否 |
| `data_studio__mongo__drop_index` | dockit · MongoDB | 🟡 提升 | 数据读写 | 否 |
| `data_studio__mongo__delete_many` | dockit · MongoDB | 🔴 破坏性 | 完全访问 | 是 |
| `data_studio__mongo__delete_document` | dockit · MongoDB | 🔴 破坏性 | 完全访问 | 是 |
| `data_studio__mongo__drop_collection` | dockit · MongoDB | 🔴 破坏性 | 完全访问 | 是 |
| `data_studio__mongo__drop_database` | dockit · MongoDB | 🔴 破坏性 | 完全访问 | 是 |
| `data_studio__mongo__truncate_collection` | dockit · MongoDB | 🔴 破坏性 | 完全访问 | 是 |
| `data_studio__dynamo__execute_query` | dockit · DynamoDB | 🟢 安全 | 只读 | 否 |
| `data_studio__dynamo__describe_table` | dockit · DynamoDB | 🟢 安全 | 只读 | 否 |
| `data_studio__dynamo__list_tables` | dockit · DynamoDB | 🟢 安全 | 只读 | 否 |
| `data_studio__dynamo__query_table` | dockit · DynamoDB | 🟢 安全 | 只读 | 否 |
| `data_studio__dynamo__scan_table` | dockit · DynamoDB | 🟢 安全 | 只读 | 否 |
| `data_studio__dynamo__describe_continuous_backups` | dockit · DynamoDB | 🟢 安全 | 只读 | 否 |
| `data_studio__dynamo__describe_ttl` | dockit · DynamoDB | 🟢 安全 | 只读 | 否 |
| `data_studio__dynamo__get_table_metrics` | dockit · DynamoDB | 🟢 安全 | 只读 | 否 |
| `data_studio__dynamo__execute_write` | dockit · DynamoDB | 🟡 提升 | 数据读写 | 否 |
| `data_studio__dynamo__create_item` | dockit · DynamoDB | 🟡 提升 | 数据读写 | 否 |
| `data_studio__dynamo__batch_write_items` | dockit · DynamoDB | 🟡 提升 | 数据读写 | 否 |
| `data_studio__dynamo__update_item` | dockit · DynamoDB | 🟡 提升 | 数据读写 | 否 |
| `data_studio__dynamo__create_gsi` | dockit · DynamoDB | 🟡 提升 | 数据读写 | 否 |
| `data_studio__dynamo__update_gsi` | dockit · DynamoDB | 🟡 提升 | 数据读写 | 否 |
| `data_studio__dynamo__create_table` | dockit · DynamoDB | 🟡 提升 | 数据读写 | 否 |
| `data_studio__dynamo__update_table_config` | dockit · DynamoDB | 🟡 提升 | 数据读写 | 否 |
| `data_studio__dynamo__update_ttl` | dockit · DynamoDB | 🟡 提升 | 数据读写 | 否 |
| `data_studio__dynamo__update_pitr` | dockit · DynamoDB | 🟡 提升 | 数据读写 | 否 |
| `data_studio__dynamo__update_streams` | dockit · DynamoDB | 🟡 提升 | 数据读写 | 否 |
| `data_studio__dynamo__execute_delete` | dockit · DynamoDB | 🔴 破坏性 | 完全访问 | 是 |
| `data_studio__dynamo__delete_item` | dockit · DynamoDB | 🔴 破坏性 | 完全访问 | 是 |
| `data_studio__dynamo__delete_gsi` | dockit · DynamoDB | 🔴 破坏性 | 完全访问 | 是 |
| `data_studio__dynamo__delete_table` | dockit · DynamoDB | 🔴 破坏性 | 完全访问 | 是 |
| `data_studio__dynamo__truncate_table` | dockit · DynamoDB | 🔴 破坏性 | 完全访问 | 是 |

**共 79 个工具。** 只读操作在**只读**模式下自动运行。提升操作（写入、索引/schema 变更）需要**数据读写**权限。破坏性操作（DELETE、DROP、TRUNCATE）需要**完全访问**权限，并始终弹出显式**用户确认**提示。

## 工作原理

```
AI 编程助手 (Claude Code / Cursor / OpenCode ...)
    |
    | MCP stdio 协议
    v
@geek-fun/data-studio-mcp   ← npm 包（纯 TypeScript）
    |
    | HTTP（仅本机）
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

MCP 服务器是一个轻量的路由层。所有数据库驱动、SSH 隧道和连接管理都在桌面应用中，由应用暴露一个本机 HTTP Bridge（仅 `127.0.0.1`）。MCP 服务器通过各应用的端口文件自动发现正在运行的后端。

---

## 给开发者

本仓库还包含 **data-studio-agent Rust 框架**，即驱动 [dockit](https://github.com/geek-fun/dockit) 和 [sqlkit](https://github.com/geek-fun/sqlkit) 内置 AI 助手的共享 Agent 循环（Provider 适配、流式输出、工具调用、上下文压缩）。

安装、架构与集成指南见 [crates/data-studio-agent/README.md](crates/data-studio-agent/README.md)（英文）。

## 许可证

Apache 2.0。见 [LICENSE](LICENSE)。
