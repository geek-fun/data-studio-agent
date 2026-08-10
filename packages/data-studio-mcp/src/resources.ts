import type { BackendClient } from './backends.js';
import { listConnections, type MergedConnection } from './connections.js';
import type { BackendStatus } from './status.js';
import type { BackendName } from './discovery.js';

/**
 * Read-only resources, resource templates, and prompts that make database
 * structure discoverable without requiring a tool call first.
 *
 * Resources carry stable identity (connections, schema snapshots) so clients
 * can cache and auto-attach them as context. Templates parameterize schemas by
 * connection. Prompts expose reusable workflows the user can invoke directly.
 */

const RESOURCE_PREFIX = 'data-studio';

// ── Resources ───────────────────────────────────────────────────────────────

export const CONNECTIONS_RESOURCE_URI = `${RESOURCE_PREFIX}://connections`;

export type ResourceDefinition = {
  uri: string;
  name: string;
  mimeType: string;
  description?: string;
};

export const listResources = async (
  clients: Map<string, BackendClient>,
  statuses: readonly BackendStatus[],
): Promise<ResourceDefinition[]> => {
  const resources: ResourceDefinition[] = [
    {
      uri: CONNECTIONS_RESOURCE_URI,
      name: 'Database Connections',
      mimeType: 'application/json',
      description: 'All configured database connections across dockit and sqlkit.',
    },
  ];

  for (const status of statuses) {
    if (status.status !== 'connected') continue;
    const client = clients.get(status.name);
    if (!client) continue;
    const conns = await safeListConnections(client);
    for (const conn of conns) {
      resources.push({
        uri: schemaResourceUri(status.name as BackendName, String(conn.id)),
        name: `Schema: ${conn.name}`,
        mimeType: 'text/plain',
        description: `Database schema for connection "${conn.name}" (${conn.type})`,
      });
    }
  }

  return resources;
};

// ── Resource Templates ──────────────────────────────────────────────────────

export const SCHEMA_RESOURCE_TEMPLATE = `${RESOURCE_PREFIX}://{backend}/{connection_id}/schema`;

export const listResourceTemplates = (): Array<{
  uriTemplate: string;
  name: string;
  mimeType: string;
  description: string;
}> => [
  {
    uriTemplate: SCHEMA_RESOURCE_TEMPLATE,
    name: 'Database Schema',
    mimeType: 'text/plain',
    description:
      'Schema snapshot (tables, columns, types, keys) for a connection. backend: dockit|sqlkit, connection_id: from data_studio://connections.',
  },
];

export type ReadResourceResult = {
  contents: Array<{
    uri: string;
    mimeType: string;
    text: string;
  }>;
};

export const readResource = async (
  uri: string,
  clients: Map<string, BackendClient>,
): Promise<ReadResourceResult> => {
  if (uri === CONNECTIONS_RESOURCE_URI) {
    const conns = await listConnections(clients);
    return {
      contents: [
        {
          uri,
          mimeType: 'application/json',
          text: JSON.stringify(conns, null, 2),
        },
      ],
    };
  }

  const match = /^data-studio:\/\/(dockit|sqlkit)\/([^/]+)\/schema$/.exec(uri);
  if (!match) {
    throw new Error(`Unknown resource: ${uri}`);
  }

  const [, backend, connectionId] = match;
  const client = clients.get(backend);
  if (!client) {
    throw new Error(`Backend ${backend} not available for resource ${uri}`);
  }

  const schemaText =
    backend === 'sqlkit'
      ? await fetchSchemaText(client, connectionId)
      : await fetchSchemaViaListTables(client, connectionId);

  return {
    contents: [
      {
        uri,
        mimeType: 'text/plain',
        text: schemaText,
      },
    ],
  };
};

// ── Prompts ─────────────────────────────────────────────────────────────────

export type PromptDefinition = {
  name: string;
  description: string;
  arguments?: Array<{ name: string; description: string; required?: boolean }>;
};

export const listPrompts = (): PromptDefinition[] => [
  {
    name: 'explore-database',
    description:
      'Explore a database connection: list its tables and give a readable summary of its schema.',
    arguments: [
      {
        name: 'connection_id',
        description: 'Connection id (from data_studio__list_connections)',
        required: true,
      },
      {
        name: 'database',
        description: 'Optional database name to scope exploration',
        required: false,
      },
    ],
  },
  {
    name: 'inspect-table',
    description: 'Inspect a single table: columns, types, keys, and a sample of rows.',
    arguments: [
      { name: 'connection_id', description: 'Connection id', required: true },
      { name: 'table', description: 'Table name', required: true },
      { name: 'database', description: 'Optional database name', required: false },
      { name: 'limit', description: 'Sample row limit (default 20)', required: false },
    ],
  },
  {
    name: 'debug-query',
    description: 'Diagnose a slow or failing SQL query: review it, run EXPLAIN, and suggest fixes.',
    arguments: [
      { name: 'connection_id', description: 'Connection id', required: true },
      { name: 'sql', description: 'The SQL query to diagnose', required: true },
      { name: 'database', description: 'Optional database name', required: false },
    ],
  },
];

// ── Helpers ─────────────────────────────────────────────────────────────────

const schemaResourceUri = (backend: BackendName, connectionId: string): string =>
  `${RESOURCE_PREFIX}://${backend}/${connectionId}/schema`;

const safeListConnections = async (client: BackendClient): Promise<MergedConnection[]> => {
  try {
    const { connections } = await client.listTools();
    return (connections ?? []).map(conn => ({ backend: client.name, ...conn }));
  } catch {
    return [];
  }
};

const fetchSchemaText = async (client: BackendClient, connectionId: string): Promise<string> => {
  const result = await client.invokeTool('get_schema', {
    connection_id: connectionId,
  });
  if (result.status >= 400) {
    throw new Error(`Failed to read schema: ${result.message ?? `HTTP ${result.status}`}`);
  }
  const data = result.data;
  if (typeof data === 'string') return data;
  if (data && typeof data === 'object' && 'data' in data) {
    const inner = (data as Record<string, unknown>).data;
    if (typeof inner === 'string') return inner;
    if (inner !== undefined) return JSON.stringify(inner, null, 2);
  }
  return JSON.stringify(data, null, 2);
};

const fetchSchemaViaListTables = async (
  client: BackendClient,
  connectionId: string,
): Promise<string> => {
  const tablesResult = await client.invokeTool('list_tables', { connection_id: connectionId });
  if (tablesResult.status >= 400) {
    throw new Error(
      `Failed to list tables: ${tablesResult.message ?? `HTTP ${tablesResult.status}`}`,
    );
  }
  const tableData = tablesResult.data;
  const tables = Array.isArray(tableData)
    ? tableData
    : Array.isArray((tableData as Record<string, unknown>)?.data)
      ? ((tableData as Record<string, unknown>).data as unknown[])
      : [];
  const tableNames = tables
    .map(t => {
      if (!t || typeof t !== 'object') return null;
      const rec = t as Record<string, unknown>;
      return String(rec.table_name ?? rec.name ?? rec.tableName ?? '');
    })
    .filter(Boolean);
  if (tableNames.length === 0) return 'No tables found.';
  const lines: string[] = [];
  for (const table of tableNames) {
    lines.push(`Table: ${table}`);
    try {
      const colsResult = await client.invokeTool('list_columns', {
        connection_id: connectionId,
        table,
      });
      if (colsResult.status < 400) {
        const colData = colsResult.data;
        const cols = Array.isArray(colData)
          ? colData
          : Array.isArray((colData as Record<string, unknown>)?.data)
            ? ((colData as Record<string, unknown>).data as unknown[])
            : [];
        for (const c of cols) {
          if (!c || typeof c !== 'object') continue;
          const rec = c as Record<string, unknown>;
          const name = String(rec.name ?? rec.column_name ?? '?');
          const type = String(rec.data_type ?? rec.type ?? '?');
          const pk = rec.is_primary_key === true ? ' PK' : '';
          const nullable = rec.nullable === false ? '' : ' NULL';
          lines.push(`  ${name}: ${type}${pk}${nullable}`);
        }
      }
    } catch {
      // skip tables whose columns can't be fetched
    }
    lines.push('');
  }
  return lines.join('\n');
};
