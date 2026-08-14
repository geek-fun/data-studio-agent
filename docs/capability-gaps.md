# DBA Capability Gap Analysis

> Audience: data product owners / agent capability maintainers
> Related code: `crates/data-studio-agent` (capability registry), dockit / sqlkit `capabilities/*.rs`
> Analysis date: 2026-08-14

## Background

A user asked the Data Studio agent to create a DynamoDB table; the agent replied that it had no `CreateTable` capability. Investigation confirmed this was a **real tool-exposure gap, not a missing implementation** — every DynamoDB capability (including create/delete table, TTL, PITR, etc.) was already implemented in the capability registry, but 12 of them were tagged `&["ui"]` (UI-only), so the agent's `agent_tools()` lookup filtered them out.

## Architecture facts

The unified capability chain:

```
dockit (Tauri)
 └─ capabilities/{dynamo,es,mongo,dockit}.rs  ──register──▶ CapabilityRegistry (data_studio_agent crate)
      └─ mcp_bridge.rs → /tools, /invoke (127.0.0.1:9120) ──serves──▶ agent / MCP clients
```

- **Registry** = `crates/data-studio-agent/src/capabilities/registry.rs`; `init_registry(&[register_all, ...])` runs once at dockit/sqlkit startup.
- **Agent-visible tools** = capabilities tagged `"agent"` (`registry.agent_tools()`); `mcp_bridge.rs` `/tools` returns that filtered result.
- **UI capabilities** = those tagged `"ui"` (`registry.ui_capabilities()`).
- A capability tagged with both is visible to both surfaces; `"ui"`-only capabilities are invisible to the agent.

> Note: the dockit-ultimate repo has a parallel `agent/tools.rs` (hard-coded DynamoDB tools, no bridge). By agreement, dockit-ultimate is out of scope for this analysis.

### How sqlkit / data-studio-mcp relate to the chain

- **sqlkit** shares the same `data_studio_agent` crate registry: `init_registry(&[crate::capabilities::sqlkit::register_all])`; its `mcp_bridge.rs` also exposes tools via `reg.agent_tools()`.
- **data-studio-mcp** is a pure dynamic router (`tools.ts` `buildToolCatalog` pulls the catalog live from each bridge's `/tools`); it has **no hard-coded tool list**, so registry changes propagate automatically with zero MCP-server changes.

## DBA capability matrix (status vs gaps)

### DynamoDB (dockit, 26 capabilities at analysis time)

**Implemented and agent-visible (14):**

| Capability | Risk | Required permission |
|---|---|---|
| `dynamo__execute_query` | 🟢 Safe | read |
| `dynamo__describe_table` | 🟢 Safe | read |
| `dynamo__list_tables` | 🟢 Safe | read |
| `dynamo__query_table` | 🟢 Safe | read |
| `dynamo__scan_table` | 🟢 Safe | read |
| `dynamo__execute_write` | 🟡 Elevated | create |
| `dynamo__create_item` | 🟡 Elevated | create |
| `dynamo__batch_write_items` | 🟡 Elevated | create |
| `dynamo__batch_get_items` | 🟢 Safe | read |
| `dynamo__update_item` | 🟡 Elevated | update |
| `dynamo__transact_write_items` | 🟡 Elevated | create |
| `dynamo__create_gsi` | 🟡 Elevated | create |
| `dynamo__execute_delete` | 🔴 Destructive | delete |
| `dynamo__delete_item` | 🔴 Destructive | delete |

**Implemented but UI-only (12) → agent gap:**

| Capability | Risk | Required permission | DBA scenario |
|---|---|---|---|
| `dynamo__create_table` | 🟡 Elevated | create | Create table |
| `dynamo__update_table_config` | 🟡 Elevated | update | Capacity/billing tuning |
| `dynamo__update_gsi` | 🟡 Elevated | update | Index scaling |
| `dynamo__update_ttl` | 🟡 Elevated | update | Data lifecycle |
| `dynamo__update_pitr` | 🟡 Elevated | update | Backup policy |
| `dynamo__update_streams` | 🟡 Elevated | update | CDC streams |
| `dynamo__delete_table` | 🔴 Destructive | delete | Drop table |
| `dynamo__delete_gsi` | 🔴 Destructive | delete | Drop index |
| `dynamo__truncate_table` | 🔴 Destructive | delete | Empty table |
| `dynamo__describe_ttl` | 🟢 Safe | read | TTL status |
| `dynamo__describe_continuous_backups` | 🟢 Safe | read | PITR status |
| `dynamo__get_table_metrics` | 🟢 Safe | read | CloudWatch monitoring |

**Not implemented at analysis time** (the first four rows are now implemented — see Phase 2 below; the remaining are still missing):

| Missing capability | DBA scenario | Notes |
|---|---|---|
| ~~`dynamo__restore_table`~~ | Restore from backup/PITR | ✅ implemented in Phase 2 |
| ~~`dynamo__create_backup` / `list_backups` / `describe_backup`~~ | Backup management | ✅ implemented in Phase 2 |
| ~~`dynamo__list_tags` / `tag_resource`~~ | Resource tagging | ✅ implemented in Phase 2 |
| ~~`dynamo__describe_limits`~~ | Account quota | ✅ implemented in Phase 2 |
| `dynamo__list_global_tables` | Global tables (replication) | Low frequency |
| `dynamo__export_table` / `import_table` | Data migration | Low frequency |

### Elasticsearch (dockit, 19 capabilities at analysis time)

**Implemented and agent-visible (19):** search, get_document, index_document, update_document, delete_document, delete_by_query, cat_indices, get_mapping, create_index, delete_index, put_mapping, cat_aliases, get_alias, put_alias, delete_alias, update_aliases, bulk, count, reindex

**UI-only (0): no gap**

**Not implemented at analysis time** (snapshot/restore and cluster health are now implemented — see Phase 2):

| Missing capability | DBA scenario |
|---|---|
| ~~snapshot / restore~~ | ✅ implemented in Phase 2 |
| ~~Cluster health (cluster health / cat nodes / cat shards)~~ | ✅ implemented in Phase 2 |
| ILM lifecycle management | Index lifecycle |
| Data streams / index template management | Operations |
| Roles/permissions management | Security |
| Slow logs / task management | Diagnostics |

### MongoDB (dockit, 30 capabilities at analysis time)

**Implemented and agent-visible (30): all open** (list_databases, list_collections, find, aggregate, insert/update/delete, index & collection management, repl-set/shard status, server status, etc.)

**UI-only (0): no gap**

**Not implemented at analysis time** (slow-query profile and user/role listing are now implemented — see Phase 2):

| Missing capability | DBA scenario |
|---|---|
| ~~Slow-query profile~~ | ✅ implemented in Phase 2 (`get_slow_queries`) |
| ~~User/role permission management~~ | ✅ implemented in Phase 2 (`list_users`) |
| oplog / change streams | CDC |
| Index rebuild | Operations |
| Generic command execution (db.runCommand) | Flexible ops |

### SQL (sqlkit, 20 capabilities at analysis time)

**Implemented and agent-visible (20): all open** (execute_query, execute_write, execute_delete, execute_ddl, list_databases/schemas/tables, get_schema, describe_table, explain_query, list_indexes/foreign_keys/views/procedures/functions/triggers, get_object_ddl, get_table_info, get_foreign_keys, list_connections)

**UI-only (0): no gap**

**Not implemented at analysis time** (session management, slow queries, and privilege management are now implemented — see Phase 2):

| Missing capability | DBA scenario |
|---|---|
| ~~Backup / restore~~ | High frequency (not yet implemented) |
| ~~Session/lock management (incl. kill session)~~ | ✅ implemented in Phase 2 (`list_sessions` / `kill_session`) |
| ~~Privilege management (GRANT/REVOKE)~~ | ✅ implemented in Phase 2 |
| ~~Slow-query log~~ | ✅ implemented in Phase 2 (`get_slow_queries`) |

## Key conclusions

1. **The DBA capability gap concentrates on DynamoDB's 12 UI-only capabilities** — `create_table` is just one; delete table, TTL, PITR, Streams, capacity config, and metric monitoring are also routine DBA operations.
2. **ES / Mongo / SQL agent coverage was already complete** (69 capabilities fully open); the real gaps lie in deeper operations: backup/restore, privilege governance, slow-query diagnostics, monitoring.
3. **Closing the 12 DynamoDB gaps = flipping tags + verifying permission mapping, zero new code** (each capability already has a full handler).
4. **Genuinely missing capabilities need new handlers**, but all are standard SDK calls, individually manageable.

## Implementation status

### Phase 1: Expose DynamoDB's 12 existing capabilities (done)

`dockit/src-tauri/src/capabilities/dynamo.rs` — flipped 12 `&["ui"]` tags to `&["agent", "ui"]`:

- 🟢 Safe (3): `describe_ttl`, `describe_continuous_backups`, `get_table_metrics`
- 🟡 Elevated (6): `create_table`, `update_table_config`, `update_gsi`, `update_ttl`, `update_pitr`, `update_streams`
- 🔴 Destructive (3): `delete_table`, `delete_gsi`, `truncate_table`

Verified: `registry.agent_tools()` DynamoDB count 14 → 26; agent-tag regression test GREEN@26.

**Impact on sqlkit / MCP: none.** sqlkit's registry is independent; data-studio-mcp pulls dynamically, so new tools appear in the MCP catalog automatically.

### Phase 2: Add high-frequency DBA capabilities (done)

| Priority | Database | Capability | Note |
|---|---|---|---|
| P0 | DynamoDB | `restore_table` | From PITR/backup, highest frequency |
| P0 | DynamoDB | `list_backups` / `create_backup` / `describe_backup` | Backup management |
| P1 | DynamoDB | `describe_limits` | Account quota |
| P1 | DynamoDB | `list_tags` / `tag_resource` | Resource tagging |
| P1 | SQL | Session/lock management (incl. kill) | Incident handling |
| P1 | SQL | Slow-query log | Diagnostics |
| P1 | ES | Cluster health (cluster health / cat nodes / cat shards) | Monitoring |
| P2 | Mongo | Slow-query profile / read-only user & role listing | Diagnostics/security |
| P2 | ES | snapshot / restore | Backup & restore |
| P2 | SQL | Privilege management (GRANT/REVOKE) | Security |

**Implementation template for each new capability:** add a handler struct + `CapabilityHandler` impl + `reg!` registration in dockit/sqlkit `capabilities/<db>.rs` (see existing `DynamoCreateTable` etc.), then set the tags and risk level.

**Impact on sqlkit / MCP:** new DynamoDB/ES/Mongo capabilities only touch dockit; new SQL capabilities only touch sqlkit; MCP needs no changes (dynamic catalog). New capabilities must carry the `"agent"` tag or they will not appear in `/tools`.

### Phase 3: Consistency safeguards (partially done)

1. **Capability description optimization** — enriched the 12 newly-exposed DynamoDB descriptions with DBA-scenario guidance (EN/中文) to improve the agent's tool-selection accuracy. (done)
2. **README tool table sync** — the tools tables in the EN/ZH READMEs were rebuilt to list all 116 agent-visible capabilities, previously stale at 79. (done)
3. **Regression tests** — agent-tag tests assert the full DynamoDB (26), ES, Mongo, and SQL capability sets carry the `"agent"` tag, preventing future regressions of the UI-only bug class. (done; these run in `cargo test`)
4. **CI gate** — a static-scan workflow (`capability-gate.yml` + `check-capability-tags.sh`) was initially added to fail-fast registrations missing the `"agent"` tag, then **removed by decision**; the regression tests in (3) remain the safeguard.
