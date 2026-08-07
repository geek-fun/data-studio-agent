import type { BackendClient } from './backends.js';
import type { BackendName } from './discovery.js';
import { buildToolAnnotations, type McpToolDef, type Route } from './tools.js';

export const SERVER_VERSION = '0.1.5';

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
  version: string;
  uptimeSeconds: number;
  readOnly: boolean;
};

export const GET_STATUS_TOOL = 'data_studio__get_status';

export const GET_STATUS_TOOL_DEF = {
  name: GET_STATUS_TOOL,
  description:
    'Get Data Studio MCP server status and backend availability. Call this first if tools seem missing or a backend tool fails.',
  inputSchema: { type: 'object' as const, properties: {}, required: [] },
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
  backends: BackendStatus[];
};

export const buildStatusPayload = (snapshot: RegistrySnapshot): StatusPayload => {
  const backendTools = snapshot.readOnly
    ? snapshot.tools.filter(t => t.metadata?.riskLevel === 'safe')
    : snapshot.tools;
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
    backends: [...snapshot.statuses],
  };
};
