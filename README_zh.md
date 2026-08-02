<div align="center">

# data-studio-agent

**把你的 AI 编程助手变成数据库助手 —— 用自然语言查询、探索和理解你的数据库。**

**本地优先。数据永不离开你的电脑。开源开放。**

[![Release](https://img.shields.io/github/v/release/geek-fun/data-studio-agent?color=orange&label=release&logo=github)](https://github.com/geek-fun/data-studio-agent/releases)
[![Downloads](https://img.shields.io/github/downloads/geek-fun/data-studio-agent/total?color=orange&logo=docusign)](https://github.com/geek-fun/data-studio-agent/releases)
[![npm](https://img.shields.io/npm/dt/@geek-fun/data-studio-mcp?color=orange&logo=npm)](https://www.npmjs.com/package/@geek-fun/data-studio-mcp)
[![License](https://img.shields.io/badge/License-Apache_2.0-blue.svg&logo=apache)](LICENSE)
[![Stars](https://img.shields.io/github/stars/geek-fun/data-studio-agent&logo=github)](https://github.com/geek-fun/data-studio-agent/stargazers)
[![CI](https://github.com/geek-fun/data-studio-agent/actions/workflows/ci.yml/badge.svg)](https://github.com/geek-fun/data-studio-agent/actions/workflows/ci.yml)

<p>
  <img src="https://img.shields.io/badge/SQL-PostgreSQL%20%7C%20MySQL%20%7C%20SQL%20Server%20%7C%20SQLite-336791"/>
  <img src="https://img.shields.io/badge/NoSQL-Elasticsearch%20%7C%20OpenSearch%20%7C%20MongoDB%20%7C%20DynamoDB-47A248"/>
  <img src="https://img.shields.io/badge/MCP-000000&logo=modelcontextprotocol&logoColor=white"/>
</p>

[npm](https://www.npmjs.com/package/@geek-fun/data-studio-mcp) · [dockit](https://github.com/geek-fun/dockit) · [sqlkit](https://github.com/geek-fun/sqlkit) · [Releases](https://github.com/geek-fun/data-studio-agent/releases)

[English](README.md) · 简体中文

</div>

---

本仓库是 **Data Studio MCP Server** 的家园 —— 一个统一的 [Model Context Protocol](https://modelcontextprotocol.io/) 服务器，通过 [dockit](https://github.com/geek-fun/dockit) 和 [sqlkit](https://github.com/geek-fun/sqlkit) 桌面应用，让 AI 编程助手（Claude Code、Cursor、Windsurf、OpenCode、Codex）直接访问你的数据库。

- **SQL**（通过 sqlkit）：PostgreSQL、MySQL、SQL Server、SQLite
- **NoSQL**（通过 dockit）：Elasticsearch、OpenSearch、MongoDB、DynamoDB

## 快速开始

### 1. 环境要求

安装并启动 [dockit](https://github.com/geek-fun/dockit) 和/或 [sqlkit](https://github.com/geek-fun/sqlkit)，添加数据库连接，确认 **设置 → MCP Bridge → 自动启动** 已开启（默认开启）。两个应用都装可获得完整的 SQL + NoSQL 工具集。

### 2. 安装 MCP 服务器

```bash
npm install -g @geek-fun/data-studio-mcp
```

或不安装直接运行：

```bash
npx -y @geek-fun/data-studio-mcp
```

### 3. 添加到你的 AI 工具

**Claude Code** — 在 MCP 配置中添加：

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

**Cursor / Windsurf / OpenCode / 其他 MCP 客户端** — 注册一个 stdio 服务器：

| 设置 | 值 |
|---|---|
| 类型 (Type) | stdio |
| 命令 (Command) | `npx` |
| 参数 (Args) | `-y @geek-fun/data-studio-mcp` |

### 4. 开始提问

直接用自然语言即可 —— AI 助手会替你查询数据库：

- "列出我 PostgreSQL 数据库中的所有表"
- "从 Elasticsearch 的 `orders*` 索引中找出最近 10 条订单"
- "在 MongoDB 中查找所有年龄大于 30 的用户"
- "执行这条查询并解释结果"

AI 助手可以查看表结构、运行查询、探索你的数据 — 并且它会展示执行的每一条查询。

## 工具命名

所有工具遵循 `data_studio__{backend}__{action}` 命名规则：

| 前缀 | 后端 | 示例 |
|---|---|---|
| `data_studio__sql_*` | sqlkit | `data_studio__sql_execute`、`data_studio__sql_list_tables` |
| `data_studio__es_*` | dockit | `data_studio__es_search`、`data_studio__es_list_indices` |
| `data_studio__mongo_*` | dockit | `data_studio__mongo_find`、`data_studio__mongo_insert` |
| `data_studio__dynamo_*` | dockit | `data_studio__dynamo_query`、`data_studio__dynamo_list_tables` |

## 安全性

- Bridge 仅绑定 `127.0.0.1` — 其他机器无法访问
- **破坏性操作和写入操作会被 Bridge 拒绝** — 通过 MCP 服务器只能使用只读能力
- 凭证永远不会暴露给 AI 助手；所有连接都在桌面应用内部解析

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

本仓库底层还包含 **data-studio-agent Rust 框架** —— 驱动 [dockit](https://github.com/geek-fun/dockit) 和 [sqlkit](https://github.com/geek-fun/sqlkit) 内置 AI 助手的共享 Agent 循环（Provider 适配、流式输出、工具调用、上下文压缩）。

安装、架构与集成指南见 [crates/data-studio-agent/README.md](crates/data-studio-agent/README.md)（英文）。

## 许可证

Apache 2.0 — 见 [LICENSE](LICENSE)。
