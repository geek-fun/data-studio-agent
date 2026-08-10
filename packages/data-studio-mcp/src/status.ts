import { readFileSync } from 'node:fs';

import type { BackendClient } from './backends.js';
import type { MergedConnection } from './connections.js';
import type { BackendName } from './discovery.js';
import { buildToolAnnotations, type McpToolDef, type Route } from './tools.js';

/**
 * Read the package version from the nearest package.json so the reported
 * SERVER_VERSION can never drift from the published version again. The build
 * copies package.json into dist/, keeping the relative path identical between
 * src (dev/tsx) and dist (production) layouts.
 */
const loadServerVersion = (): string => {
  try {
    const pkg = JSON.parse(readFileSync(new URL('../package.json', import.meta.url), 'utf8')) as {
      version?: string;
    };
    return pkg.version ?? '0.0.0';
  } catch {
    return '0.0.0';
  }
};

export const SERVER_VERSION = loadServerVersion();

export type BackendStatus = {
  name: BackendName;
  port: number | null;
  baseUrl: string | null;
  status: 'connected' | 'unavailable';
  toolCount: number;
  hint: string;
};

export type RegistrySnapshot = {
  tools: readonly McpToolDef[];
  routeMap: Map<string, Route>;
  clients: Map<string, BackendClient>;
  statuses: readonly BackendStatus[];
  /** Per-backend connection lists, cached at poll time. */
  connections: Record<string, MergedConnection[]>;
  version: string;
  uptimeSeconds: number;
  readOnly: boolean;
};

export const GET_STATUS_TOOL = 'data_studio__get_status';

export const GET_STATUS_TOOL_DEF = {
  name: GET_STATUS_TOOL,
  title: 'Data Studio Status',
  description:
    "Get Data Studio MCP server status and backend availability (dockit/sqlkit, ports, tool counts). This is the entry point for ANY database task: if the task involves querying a database, checking tables/rows/schema, or executing SQL, use the data_studio__* tools here instead of local DB CLIs. Call this first if tools seem missing, a backend tool fails, or you are unsure which database tools are available. Report results in the user's language (中文/English).",
  inputSchema: {
    type: 'object' as const,
    properties: {},
    required: [],
    additionalProperties: false,
  },
  annotations: buildToolAnnotations('safe'),
} as const;

export type StatusPayload = {
  server: {
    name: 'data-studio-mcp';
    version: string;
    uptimeSeconds: number;
    readOnly: boolean;
  };
  tools: {
    registered: number;
    backendTools: number;
  };
  backends: Array<BackendStatus & { connectionCount: number }>;
  summary: {
    totalConnections: number;
    connectionsByType: Record<string, number>;
    toolsByBackend: Record<string, number>;
  };
};

export const buildStatusPayload = (snapshot: RegistrySnapshot): StatusPayload => {
  const backendTools = snapshot.readOnly
    ? snapshot.tools.filter(t => t.metadata?.riskLevel === 'safe')
    : snapshot.tools;

  const allConnections = Object.values(snapshot.connections ?? {}).flat();
  const connectionsByType: Record<string, number> = {};
  for (const conn of allConnections) {
    connectionsByType[conn.type] = (connectionsByType[conn.type] ?? 0) + 1;
  }

  const toolsByBackend: Record<string, number> = {};
  for (const tool of backendTools) {
    toolsByBackend[tool.backendName] = (toolsByBackend[tool.backendName] ?? 0) + 1;
  }

  return {
    server: {
      name: 'data-studio-mcp',
      version: snapshot.version,
      uptimeSeconds: snapshot.uptimeSeconds,
      readOnly: snapshot.readOnly,
    },
    tools: {
      registered: backendTools.length + 2,
      backendTools: backendTools.length,
    },
    backends: snapshot.statuses.map(s => ({
      ...s,
      connectionCount: snapshot.connections[s.name]?.length ?? 0,
    })),
    summary: {
      totalConnections: allConnections.length,
      connectionsByType,
      toolsByBackend,
    },
  };
};
