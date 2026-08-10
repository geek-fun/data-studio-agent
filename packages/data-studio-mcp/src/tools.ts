import type { ToolAnnotations } from '@modelcontextprotocol/sdk/types.js';
import { displayName, type BackendInfo, type BackendName } from './discovery.js';
import { createBackendClient, type BridgeToolDef } from './backends.js';

export type McpToolDef = {
  name: string;
  title?: string;
  description: string;
  inputSchema: object;
  backendName: string;
  internalName: string;
  metadata?: BridgeToolDef['metadata'];
  annotations?: ToolAnnotations;
};

type RiskLevel = 'safe' | 'elevated' | 'destructive';

const ANNOTATIONS_BY_RISK: Record<RiskLevel, ToolAnnotations> = {
  safe: {
    readOnlyHint: true,
    destructiveHint: false,
    idempotentHint: true,
    openWorldHint: false,
  },
  elevated: {
    readOnlyHint: false,
    destructiveHint: false,
    idempotentHint: false,
    openWorldHint: true,
  },
  destructive: {
    readOnlyHint: false,
    destructiveHint: true,
    idempotentHint: false,
    openWorldHint: true,
  },
};

export const buildToolAnnotations = (riskLevel: RiskLevel | undefined): ToolAnnotations =>
  riskLevel ? ANNOTATIONS_BY_RISK[riskLevel] : {};

export type Route = { backendName: BackendName; internalName: string };

type CatalogEntry = {
  tool: McpToolDef;
  route: Route;
};

export const buildBackendRequirementNote = (backend: BackendInfo): string =>
  `⚠️ Requires ${displayName(backend.name)} running on localhost:${backend.port}. If unavailable, call data_studio__get_status.`;

/** Derive a human-readable title from a snake_case tool name (e.g. "execute_query" → "Execute Query"). */
export const humanizeToolName = (name: string): string => {
  const lastSegment = name.split('__').pop() ?? name;
  const words = lastSegment.split('_').filter(Boolean);
  if (words.length === 0) return name;
  const capitalized = words.map(w => w.charAt(0).toUpperCase() + w.slice(1)).join(' ');
  return capitalized.length <= 48 ? capitalized : capitalized.slice(0, 45) + '...';
};

const fetchBackendCatalog = async (backend: BackendInfo): Promise<readonly CatalogEntry[]> => {
  const client = createBackendClient(backend);
  let bridgeTools: BridgeToolDef[];
  try {
    bridgeTools = (await client.listTools()).tools;
  } catch (err) {
    console.error(`Failed to fetch tools from ${backend.name}:`, err);
    return [];
  }

  const note = buildBackendRequirementNote(backend);
  return bridgeTools.map(bt => {
    const mcpName = `data_studio__${backend.name}__${bt.name}`;
    return {
      tool: {
        name: mcpName,
        title: humanizeToolName(bt.name),
        description: bt.description.includes('data_studio__get_status')
          ? bt.description
          : `${bt.description}\n\n${note}`,
        inputSchema: bt.inputSchema,
        backendName: backend.name,
        internalName: bt.name,
        metadata: bt.metadata,
        annotations: buildToolAnnotations(bt.metadata?.riskLevel),
      },
      route: { backendName: backend.name, internalName: bt.name },
    } satisfies CatalogEntry;
  });
};

export const buildToolCatalog = async (
  backends: readonly BackendInfo[],
): Promise<{
  tools: McpToolDef[];
  routeMap: Map<string, { backendName: 'dockit' | 'sqlkit'; internalName: string }>;
}> => {
  const entries = (await Promise.all(backends.map(fetchBackendCatalog))).flat();
  return {
    tools: entries.map(e => e.tool),
    routeMap: new Map(entries.map(e => [e.tool.name, e.route])),
  };
};
