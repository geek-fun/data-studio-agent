<div align="center">

# @geek-fun/data-studio-mcp

**把你的 AI 编程助手（Claude Code、Cursor、Windsurf、OpenCode、Codex）变成数据库助手 —— 用自然语言查询、探索和理解你的数据库。**

**本地优先。数据永不离开你的电脑。开源开放。**

[![npm version](https://img.shields.io/npm/v/@geek-fun/data-studio-mcp?color=orange&logo=npm)](https://www.npmjs.com/package/@geek-fun/data-studio-mcp)
[![Downloads](https://img.shields.io/npm/dt/@geek-fun/data-studio-mcp?color=orange&logo=npm)](https://www.npmjs.com/package/@geek-fun/data-studio-mcp)
[![License](https://img.shields.io/badge/License-Apache_2.0-blue.svg&logo=apache)](https://github.com/geek-fun/data-studio-agent/blob/master/LICENSE)

<p>
  <img src="https://img.shields.io/badge/SQL-PostgreSQL%20%7C%20MySQL%20%7C%20SQL%20Server%20%7C%20SQLite-336791"/>
  <img src="https://img.shields.io/badge/NoSQL-Elasticsearch%20%7C%20OpenSearch%20%7C%20MongoDB%20%7C%20DynamoDB-47A248"/>
  <img src="https://img.shields.io/badge/TypeScript-3178C6?logo=typescript&logoColor=white"/>
</p>

[npm](https://www.npmjs.com/package/@geek-fun/data-studio-mcp) · [dockit](https://github.com/geek-fun/dockit) · [sqlkit](https://github.com/geek-fun/sqlkit) · [Releases](https://github.com/geek-fun/data-studio-agent/releases)

[English](README.md) · 简体中文

</div>

---

让你的 AI 编程助手（Claude Code、Cursor、Windsurf、OpenCode、Codex）变成数据库助手。直接用自然语言提问，让它查询数据、查看表结构、编写 SQL 或 NoSQL 查询 — 支持：

- **SQL 数据库**（通过 [sqlkit](https://github.com/geek-fun/sqlkit)）：PostgreSQL、MySQL、SQL Server、SQLite
- **NoSQL 数据库**（通过 [dockit](https://github.com/geek-fun/dockit)）：Elasticsearch、OpenSearch、MongoDB、DynamoDB

已经装了桌面应用？那就只需两步：安装本包，然后把它加到你的 AI 工具里。无需部署服务器、无需管理 API Key — 一切都在你本机运行。

## 环境要求

1. 安装 [dockit](https://github.com/geek-fun/dockit) 和/或 [sqlkit](https://github.com/geek-fun/sqlkit)（免费、开源）
2. 启动应用并添加至少一个数据库连接
3. 确认 **设置 → MCP Bridge → 自动启动** 已开启（默认开启）

两个应用都装可获得完整工具集 — SQL 和 NoSQL。装一个即可开始使用。

## 安装

**全局安装（推荐）：**

```bash
npm install -g @geek-fun/data-studio-mcp
```

**或不安装直接运行**（每次执行）：

```bash
npx -y @geek-fun/data-studio-mcp
```

包只需下载一次，之后在本地运行。你的数据永远不会离开你的电脑。

## 添加到你的 AI 工具

### Claude Code

在 MCP 配置中添加：

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

### Cursor / Windsurf / OpenCode / 其他 MCP 客户端

添加一个新的 MCP 服务器：

| 设置 | 值 |
|---|---|
| 类型 (Type) | stdio |
| 命令 (Command) | `npx` |
| 参数 (Args) | `-y @geek-fun/data-studio-mcp` |

## 使用方法

连接成功后，直接用自然语言提问即可。AI 助手会为你调用数据库工具：

- "列出我 PostgreSQL 数据库中的所有表"
- "从 Elasticsearch 的 `orders*` 索引中找出最近 10 条订单"
- "在 MongoDB 中查找所有年龄大于 30 的用户"
- "执行这条查询并解释结果"

AI 助手可以查看表结构、运行查询、探索你的数据 — 并且它会展示执行的每一条查询。

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
@geek-fun/data-studio-mcp   ← 本包（纯 TypeScript）
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

MCP 服务器是一个轻量的路由层。所有数据库驱动、SSH 隧道和连接管理都在桌面应用中，由应用暴露一个本机 HTTP Bridge（仅 `127.0.0.1`）。MCP 服务器通过各应用的端口文件自动发现正在运行的后端 — 只暴露已运行后端的工具。

## 工具命名

所有工具遵循 `data_studio__{backend}__{action}` 命名规则：

| 前缀 | 后端 | 示例 |
|---|---|---|
| `data_studio__sql_*` | sqlkit | `data_studio__sql_execute`、`data_studio__sql_list_tables` |
| `data_studio__es_*` | dockit | `data_studio__es_search`、`data_studio__es_list_indices` |
| `data_studio__mongo_*` | dockit | `data_studio__mongo_find`、`data_studio__mongo_insert` |
| `data_studio__dynamo_*` | dockit | `data_studio__dynamo_query`、`data_studio__dynamo_list_tables` |

## 开发

```bash
npm install
npm run build      # 编译 TypeScript
npm test           # 运行单元测试 (vitest)
npm run lint:check # ESLint
```

## 发布

修改 `package.json` 中的版本号并合并到 `master`。[发布工作流](https://github.com/geek-fun/data-studio-agent/blob/master/.github/workflows/release-mcp.yml) 会自动发布到 npm（OIDC 可信发布）并创建 tag 和 GitHub Release。

## 许可证

[Apache-2.0](https://github.com/geek-fun/data-studio-agent/blob/master/LICENSE)
