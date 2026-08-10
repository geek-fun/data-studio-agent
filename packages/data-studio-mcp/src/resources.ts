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
  statuses: readonly BackendStatus[],
  cachedConnections: Record<string, MergedConnection[]>,
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
    const conns = cachedConnections[status.name] ?? [];
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
  cachedConnections: Record<string, MergedConnection[]>,
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

  const connType = cachedConnections[backend]?.find(c => String(c.id) === connectionId)?.type ?? '';

  let schemaText: string;
  if (backend === 'sqlkit') {
    schemaText = await fetchSchemaText(client, connectionId);
  } else {
    const t = connType.toLowerCase();
    if (t.includes('elastic') || t.includes('open')) {
      schemaText = await fetchEsSchema(client, connectionId);
    } else if (t.includes('mongo')) {
      schemaText = await fetchMongoSchema(client, connectionId);
    } else if (t.includes('dynamo')) {
      schemaText = await fetchDynamoSchema(client, connectionId);
    } else {
      schemaText = `Schema not available for connection type "${connType}".`;
    }
  }

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

const extractArray = (data: unknown): unknown[] => {
  if (Array.isArray(data)) return data;
  if (data && typeof data === 'object' && 'data' in data) {
    const inner = (data as Record<string, unknown>).data;
    if (Array.isArray(inner)) return inner;
  }
  return [];
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

const fetchEsSchema = async (client: BackendClient, connectionId: string): Promise<string> => {
  const indicesResult = await client.invokeTool('es__cat_indices', {
    connection_id: connectionId,
  });
  if (indicesResult.status >= 400) {
    throw new Error(
      `Failed to list ES indices: ${indicesResult.message ?? `HTTP ${indicesResult.status}`}`,
    );
  }
  const indices = extractArray(indicesResult.data);
  const indexNames = indices
    .map(i => {
      if (!i || typeof i !== 'object') return null;
      const rec = i as Record<string, unknown>;
      return String(rec.index ?? rec.name ?? '');
    })
    .filter(Boolean);
  if (indexNames.length === 0) return 'No indices found.';
  const lines: string[] = [];
  for (const idx of indexNames) {
    lines.push(`Index: ${idx}`);
    try {
      const mapResult = await client.invokeTool('es__get_mapping', {
        connection_id: connectionId,
        index: idx,
      });
      if (mapResult.status < 400) {
        lines.push(`  ${JSON.stringify(mapResult.data).slice(0, 500)}`);
      }
    } catch {
      // skip indices whose mapping can't be fetched
    }
    lines.push('');
  }
  return lines.join('\n');
};

const fetchMongoSchema = async (client: BackendClient, connectionId: string): Promise<string> => {
  const dbsResult = await client.invokeTool('mongo__list_databases', {
    connection_id: connectionId,
  });
  if (dbsResult.status >= 400) {
    throw new Error(
      `Failed to list MongoDB databases: ${dbsResult.message ?? `HTTP ${dbsResult.status}`}`,
    );
  }
  const dbs = extractArray(dbsResult.data);
  const dbNames = dbs
    .map(d => {
      if (!d || typeof d !== 'object') return null;
      const rec = d as Record<string, unknown>;
      return String(rec.name ?? rec.database ?? '');
    })
    .filter(Boolean);
  if (dbNames.length === 0) return 'No databases found.';
  const lines: string[] = [];
  for (const db of dbNames) {
    lines.push(`Database: ${db}`);
    try {
      const colsResult = await client.invokeTool('mongo__list_collections', {
        connection_id: connectionId,
        database: db,
      });
      if (colsResult.status < 400) {
        const cols = extractArray(colsResult.data);
        for (const c of cols) {
          const rec = c as Record<string, unknown> | null;
          if (rec) lines.push(`  Collection: ${String(rec.name ?? rec.collection ?? '?')}`);
        }
      }
    } catch {
      // skip databases whose collections can't be fetched
    }
    lines.push('');
  }
  return lines.join('\n');
};

const fetchDynamoSchema = async (client: BackendClient, connectionId: string): Promise<string> => {
  const tablesResult = await client.invokeTool('dynamo__list_tables', {
    connection_id: connectionId,
  });
  if (tablesResult.status >= 400) {
    throw new Error(
      `Failed to list DynamoDB tables: ${tablesResult.message ?? `HTTP ${tablesResult.status}`}`,
    );
  }
  const tables = extractArray(tablesResult.data);
  const tableNames = tables
    .map(t => {
      if (!t || typeof t !== 'object') return null;
      const rec = t as Record<string, unknown>;
      return String(rec.table_name ?? rec.tableName ?? rec.name ?? '');
    })
    .filter(Boolean);
  if (tableNames.length === 0) return 'No tables found.';
  const lines: string[] = [];
  for (const table of tableNames) {
    lines.push(`Table: ${table}`);
    try {
      const descResult = await client.invokeTool('dynamo__describe_table', {
        connection_id: connectionId,
        table_name: table,
      });
      if (descResult.status < 400) {
        const desc = extractArray(descResult.data);
        for (const c of desc) {
          const rec = c as Record<string, unknown> | null;
          if (rec) {
            const name = String(rec.attribute_name ?? rec.name ?? '?');
            const type = String(rec.attribute_type ?? rec.type ?? '?');
            lines.push(`  ${name}: ${type}`);
          }
        }
      }
    } catch {
      // skip tables whose description can't be fetched
    }
    lines.push('');
  }
  return lines.join('\n');
};
