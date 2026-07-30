import type { BackendInfo } from './discovery.js';
import { BackendClient, type BridgeToolDef } from './backends.js';

/**
 * An MCP tool definition enriched with routing metadata.
 */
export interface McpToolDef {
  name: string;
  description: string;
  inputSchema: object;
  backendName: string;
  internalName: string;
}

/**
 * Build the unified MCP tool catalog from all discovered backends.
 *
 * Each backend's internal tools are prefixed with `data_studio__{backend}__`
 * to create unique MCP tool names. The mapping is returned so the call
 * handler can reverse-lookup the target backend and internal name.
 */
export async function buildToolCatalog(
  backends: BackendInfo[],
): Promise<{ tools: McpToolDef[]; routeMap: Map<string, { backendName: string; internalName: string }> }> {
  const tools: McpToolDef[] = [];
  const routeMap = new Map<string, { backendName: string; internalName: string }>();

  for (const backend of backends) {
    const client = new BackendClient(backend);
    let bridgeTools: BridgeToolDef[];

    try {
      bridgeTools = await client.listTools();
    } catch (err) {
      console.error(`Failed to fetch tools from ${backend.name}:`, err);
      continue;
    }

    for (const bt of bridgeTools) {
      const mcpName = `data_studio__${backend.name}__${bt.name}`;
      tools.push({
        name: mcpName,
        description: bt.description,
        inputSchema: bt.inputSchema,
        backendName: backend.name,
        internalName: bt.name,
      });
      routeMap.set(mcpName, {
        backendName: backend.name,
        internalName: bt.name,
      });
    }
  }

  return { tools, routeMap };
}
