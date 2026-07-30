#!/usr/bin/env node

import { Server } from '@modelcontextprotocol/sdk/server/index.js';
import { StdioServerTransport } from '@modelcontextprotocol/sdk/server/stdio.js';
import {
  ListToolsRequestSchema,
  CallToolRequestSchema,
  type CallToolResult,
} from '@modelcontextprotocol/sdk/types.js';

import { discoverBackends } from './discovery.js';
import { BackendClient } from './backends.js';
import { buildToolCatalog } from './tools.js';

async function main(): Promise<void> {
  const backends = discoverBackends();
  if (backends.length === 0) {
    console.error(
      'No backends found. Start dockit or sqlkit first, or set DOCKIT_MCP_PORT / SQLKIT_MCP_PORT environment variables.',
    );
    process.exit(1);
  }

  console.error(
    `Discovered backends: ${backends.map((b) => `${b.name} (${b.baseUrl})`).join(', ')}`,
  );

  const { tools, routeMap } = await buildToolCatalog(backends);
  if (tools.length === 0) {
    console.error('No tools found from any backend.');
    process.exit(1);
  }

  console.error(`Loaded ${tools.length} tools from ${backends.length} backend(s).`);

  const clients = new Map<string, BackendClient>(
    backends.map((b) => [b.name, new BackendClient(b)]),
  );

  const server = new Server(
    { name: 'data-studio-mcp', version: '0.1.0' },
    { capabilities: { tools: {} } },
  );

  server.setRequestHandler(ListToolsRequestSchema, async () => {
    return {
      tools: tools.map((t) => ({
        name: t.name,
        description: t.description,
        inputSchema: {
          type: 'object' as const,
          properties: (t.inputSchema as Record<string, unknown>)
            ?.properties as Record<string, object> | undefined,
          required: (t.inputSchema as Record<string, unknown>)
            ?.required as string[] | undefined,
        },
      })),
    };
  });

  server.setRequestHandler(CallToolRequestSchema, async (request) => {
    const toolName = request.params.name;
    const args = (request.params.arguments ?? {}) as Record<string, unknown>;

    const route = routeMap.get(toolName);
    if (!route) {
      return {
        content: [{ type: 'text', text: `Unknown tool: ${toolName}` }],
        isError: true,
      } satisfies CallToolResult;
    }

    const client = clients.get(route.backendName);
    if (!client) {
      return {
        content: [{ type: 'text', text: `Backend ${route.backendName} is not available` }],
        isError: true,
      } satisfies CallToolResult;
    }

    try {
      const result = await client.invokeTool(route.internalName, args);

      if (result.status >= 400) {
        return {
          content: [{ type: 'text', text: result.message ?? `Error (HTTP ${result.status})` }],
          isError: true,
        } satisfies CallToolResult;
      }

      return {
        content: [
          {
            type: 'text',
            text:
              typeof result.data === 'string'
                ? result.data
                : JSON.stringify(result.data, null, 2),
          },
        ],
      } satisfies CallToolResult;
    } catch (err) {
      const message = err instanceof Error ? err.message : String(err);
      return {
        content: [{ type: 'text', text: `Bridge error: ${message}` }],
        isError: true,
      } satisfies CallToolResult;
    }
  });

  const transport = new StdioServerTransport();
  await server.connect(transport);
}

main().catch((err) => {
  console.error('Fatal error:', err);
  process.exit(1);
});
