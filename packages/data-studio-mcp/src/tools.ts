import type { BackendInfo } from './discovery.js';
import { createBackendClient, type BridgeToolDef } from './backends.js';

export type McpToolDef = {
  name: string;
  description: string;
  inputSchema: object;
  backendName: string;
  internalName: string;
};

type CatalogEntry = {
  tool: McpToolDef;
  route: { backendName: 'dockit' | 'sqlkit'; internalName: string };
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

  return bridgeTools.map(bt => {
    const mcpName = `data_studio__${backend.name}__${bt.name}`;
    return {
      tool: {
        name: mcpName,
        description: bt.description,
        inputSchema: bt.inputSchema,
        backendName: backend.name,
        internalName: bt.name,
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
