<div align="center">

<img src="https://raw.githubusercontent.com/geek-fun/data-studio-agent/master/docs/images/data-studio-agent.svg" width="96" height="96" alt="Data Studio Agent logo" />

# @geek-fun/data-studio-mcp

**让你的 AI 编程智能体（Claude Code、Cursor、Windsurf、OpenCode、Codex）安全访问所有数据库 —— 用自然语言查询、探索和理解你的数据。**

**本地优先。企业级安全。开源开放。**

[![npm version](https://img.shields.io/npm/v/@geek-fun/data-studio-mcp?color=orange&logo=npm)](https://www.npmjs.com/package/@geek-fun/data-studio-mcp)
[![Downloads](https://img.shields.io/npm/dt/@geek-fun/data-studio-mcp?color=orange&logo=npm)](https://www.npmjs.com/package/@geek-fun/data-studio-mcp)
[![License](https://img.shields.io/badge/License-Apache_2.0-blue.svg&logo=apache)](https://github.com/geek-fun/data-studio-agent/blob/master/LICENSE)

<p>
  <img src="https://img.shields.io/badge/SQL-70%2B%20databases%20via%20SqlKit-336791"/>
  <img src="https://img.shields.io/badge/NoSQL-Elasticsearch%20%7C%20OpenSearch%20%7C%20MongoDB%20%7C%20DynamoDB-47A248"/>
  <img src="https://img.shields.io/badge/MCP-000000&logo=modelcontextprotocol&logoColor=white"/>
  <img src="https://img.shields.io/badge/TypeScript-3178C6?logo=typescript&logoColor=white"/>
</p>

[📖 产品页](https://www.geekfun.club/zh/products/data-studio-agent/) · [npm](https://www.npmjs.com/package/@geek-fun/data-studio-mcp) · [dockit](https://github.com/geek-fun/dockit) · [sqlkit](https://github.com/geek-fun/sqlkit) · [Releases](https://github.com/geek-fun/data-studio-agent/releases)

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

## 企业级安全

为安全优先的团队而设计。LLM 是"有权限但受控"的执行者：它可以对你的数据做很多事，但永远无法获得你的凭据。

- **凭据永不离开应用** —— LLM 只能看到一个不透明的 `connection_id`；真实凭据在 dockit/sqlkit 内部解析，绝不跨越 MCP 边界。你的密码和密钥始终留在本机、留在应用里。
- **基于 ID 的资源访问** —— 代理严格通过连接 ID 访问数据库，绝不在提示词或工具参数中嵌入凭据。模型没有任何途径获取或泄露连接密钥。
- **三级权限模型** —— 只读 / 数据读写 / 完全访问三种模式按风险等级管控每个能力。外加连接级覆盖：可将任意连接标记为只读，或按操作白名单放行。
- **显式用户确认** —— 破坏性操作（DELETE、DROP、TRUNCATE）在策略中标记为 `Ask` —— 客户端在执行任何破坏性操作前都会弹出显式确认。不会有静默执行的破坏操作。
- **操作级语句分类** —— SQL 在执行前按语句类型解析分类（读 / 写 / 删除 / DDL）。只写工具拒绝 DELETE 语句；删除工具拒绝 DDL —— 杜绝意外的权限升级。
- **仅本地桥接** —— 桥接只绑定 `127.0.0.1` —— 其他机器无法访问。一个薄路由层，无需托管服务器、无需管理 API key、不向网络暴露任何东西。

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

所有工具遵循 `data_studio__{backend}__{action}` 命名规则 —— MCP 服务器会给每个桥接工具加上 `data_studio__` 前缀。风险级别（🟢 安全 / 🟡 提升 / 🔴 破坏性）通过权限模型控制暴露：只读模式开放安全工具；数据读写模式需要提升权限；完全访问 + 显式确认才开放破坏性工具。

### MCP 服务器工具（始终可用）

| 工具 | 风险 | 用途 |
|---|---|---|
| `data_studio__list_connections` | 🟢 安全 | 列出可用连接（id、name、type）—— 不含凭据 |
| `data_studio__get_status` | 🟢 安全 | 后端可用性、工具数量、权限状态 |

### 通过 sqlkit 的 SQL（11 个工具）

| 工具 | 风险 | 用途 |
|---|---|---|
| `data_studio__sqlkit__list_connections` | 🟢 安全 | 列出 sqlkit 连接 |
| `data_studio__sqlkit__execute_query` | 🟢 安全 | 执行只读 SELECT 查询 |
| `data_studio__sqlkit__list_databases` | 🟢 安全 | 列出数据库 |
| `data_studio__sqlkit__list_schemas` | 🟢 安全 | 列出数据库中的 schema |
| `data_studio__sqlkit__list_tables` | 🟢 安全 | 列出 schema 中的表 |
| `data_studio__sqlkit__get_schema` | 🟢 安全 | 获取数据库完整 schema |
| `data_studio__sqlkit__describe_table` | 🟢 安全 | 描述表的列 |
| `data_studio__sqlkit__explain_query` | 🟢 安全 | 解释查询执行计划 |
| `data_studio__sqlkit__execute_write` | 🟡 提升 | INSERT / UPDATE / CREATE（不含 DELETE、DDL） |
| `data_studio__sqlkit__execute_delete` | 🔴 破坏性 | DELETE / DROP / TRUNCATE 语句 |
| `data_studio__sqlkit__execute_ddl` | 🔴 破坏性 | DDL 语句（ALTER、DROP TABLE 等） |

### 通过 dockit 的 NoSQL —— Elasticsearch（16 个工具）

| 工具 | 风险 | 用途 |
|---|---|---|
| `data_studio__es__search` | 🟢 安全 | 使用 Query DSL 搜索 |
| `data_studio__es__get_document` | 🟢 安全 | 按 ID 获取文档 |
| `data_studio__es__cat_indices` | 🟢 安全 | 列出索引 |
| `data_studio__es__get_mapping` | 🟢 安全 | 获取索引映射 |
| `data_studio__es__cat_aliases` | 🟢 安全 | 列出别名 |
| `data_studio__es__get_alias` | 🟢 安全 | 获取别名 |
| `data_studio__es__index_document` | 🟡 提升 | 创建或替换文档 |
| `data_studio__es__update_document` | 🟡 提升 | 局部更新文档 |
| `data_studio__es__create_index` | 🟡 提升 | 创建索引 |
| `data_studio__es__put_mapping` | 🟡 提升 | 更新索引映射 |
| `data_studio__es__put_alias` | 🟡 提升 | 创建别名 |
| `data_studio__es__update_aliases` | 🟡 提升 | 批量别名操作 |
| `data_studio__es__delete_document` | 🔴 破坏性 | 删除文档 |
| `data_studio__es__delete_by_query` | 🔴 破坏性 | 按查询删除文档 |
| `data_studio__es__delete_index` | 🔴 破坏性 | 删除索引 |
| `data_studio__es__delete_alias` | 🔴 破坏性 | 删除别名 |

### 通过 dockit 的 NoSQL —— MongoDB（26 个工具）

| 工具 | 风险 | 用途 |
|---|---|---|
| `data_studio__mongo__list_databases` | 🟢 安全 | 列出数据库 |
| `data_studio__mongo__list_collections` | 🟢 安全 | 列出集合 |
| `data_studio__mongo__find` | 🟢 安全 | 查询文档 |
| `data_studio__mongo__collection_stats` | 🟢 安全 | 集合统计 |
| `data_studio__mongo__database_stats` | 🟢 安全 | 数据库统计 |
| `data_studio__mongo__server_status` | 🟢 安全 | 服务器状态 |
| `data_studio__mongo__repl_set_status` | 🟢 安全 | 副本集状态 |
| `data_studio__mongo__shard_status` | 🟢 安全 | 分片状态 |
| `data_studio__mongo__count_documents` | 🟢 安全 | 统计文档数 |
| `data_studio__mongo__list_indexes` | 🟢 安全 | 列出索引 |
| `data_studio__mongo__sample_documents` | 🟢 安全 | 采样文档 |
| `data_studio__mongo__aggregate` | 🟡 提升 | 聚合管道 |
| `data_studio__mongo__insert_one` | 🟡 提升 | 插入单条文档 |
| `data_studio__mongo__update_many` | 🟡 提升 | 更新多条文档 |
| `data_studio__mongo__update_document` | 🟡 提升 | 更新单条文档 |
| `data_studio__mongo__create_database` | 🟡 提升 | 创建数据库 |
| `data_studio__mongo__create_collection` | 🟡 提升 | 创建集合 |
| `data_studio__mongo__rename_collection` | 🟡 提升 | 重命名集合 |
| `data_studio__mongo__clone_collection` | 🟡 提升 | 克隆集合 |
| `data_studio__mongo__create_index` | 🟡 提升 | 创建索引 |
| `data_studio__mongo__drop_index` | 🟡 提升 | 删除索引 |
| `data_studio__mongo__delete_many` | 🔴 破坏性 | 删除多条文档 |
| `data_studio__mongo__delete_document` | 🔴 破坏性 | 删除单条文档 |
| `data_studio__mongo__drop_collection` | 🔴 破坏性 | 删除集合 |
| `data_studio__mongo__drop_database` | 🔴 破坏性 | 删除数据库 |
| `data_studio__mongo__truncate_collection` | 🔴 破坏性 | 清空集合 |

### 通过 dockit 的 NoSQL —— DynamoDB（24 个工具）

| 工具 | 风险 | 用途 |
|---|---|---|
| `data_studio__dynamo__execute_query` | 🟢 安全 | 执行只读查询 |
| `data_studio__dynamo__describe_table` | 🟢 安全 | 描述表 |
| `data_studio__dynamo__list_tables` | 🟢 安全 | 列出表 |
| `data_studio__dynamo__query_table` | 🟢 安全 | 查询表 |
| `data_studio__dynamo__scan_table` | 🟢 安全 | 扫描表 |
| `data_studio__dynamo__describe_continuous_backups` | 🟢 安全 | 描述备份 |
| `data_studio__dynamo__describe_ttl` | 🟢 安全 | 描述 TTL |
| `data_studio__dynamo__get_table_metrics` | 🟢 安全 | 表指标 |
| `data_studio__dynamo__execute_write` | 🟡 提升 | 执行写入 |
| `data_studio__dynamo__create_item` | 🟡 提升 | 创建条目 |
| `data_studio__dynamo__batch_write_items` | 🟡 提升 | 批量写入条目 |
| `data_studio__dynamo__update_item` | 🟡 提升 | 更新条目 |
| `data_studio__dynamo__create_gsi` | 🟡 提升 | 创建 GSI |
| `data_studio__dynamo__update_gsi` | 🟡 提升 | 更新 GSI |
| `data_studio__dynamo__create_table` | 🟡 提升 | 创建表 |
| `data_studio__dynamo__update_table_config` | 🟡 提升 | 更新表配置 |
| `data_studio__dynamo__update_ttl` | 🟡 提升 | 更新 TTL |
| `data_studio__dynamo__update_pitr` | 🟡 提升 | 更新 PITR |
| `data_studio__dynamo__update_streams` | 🟡 提升 | 更新流 |
| `data_studio__dynamo__execute_delete` | 🔴 破坏性 | 执行删除 |
| `data_studio__dynamo__delete_item` | 🔴 破坏性 | 删除条目 |
| `data_studio__dynamo__delete_gsi` | 🔴 破坏性 | 删除 GSI |
| `data_studio__dynamo__delete_table` | 🔴 破坏性 | 删除表 |
| `data_studio__dynamo__truncate_table` | 🔴 破坏性 | 清空表 |

> **共 79 个工具**（2 个服务器工具 + 11 个 SQL + 16 个 Elasticsearch + 26 个 MongoDB + 24 个 DynamoDB）。破坏性工具（🔴）仅在桥接权限模式为**完全访问**且开启**确认破坏性操作**时才会出现在 `tools/list` 中 —— 否则完全隐藏。

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
