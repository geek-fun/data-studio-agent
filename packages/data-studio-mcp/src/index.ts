#!/usr/bin/env node

import { Server } from '@modelcontextprotocol/sdk/server/index.js';
import { StdioServerTransport } from '@modelcontextprotocol/sdk/server/stdio.js';
import {
  ListToolsRequestSchema,
  CallToolRequestSchema,
  type CallToolResult,
} from '@modelcontextprotocol/sdk/types.js';

import { discoverBackends } from './discovery.js';
import { createBackendClient } from './backends.js';
import { buildToolCatalog } from './tools.js';

const errorResult = (text: string): CallToolResult => ({
  content: [{ type: 'text', text }],
  isError: true,
});

const okResult = (data: unknown): CallToolResult => ({
  content: [
    { type: 'text', text: typeof data === 'string' ? data : JSON.stringify(data, null, 2) },
  ],
});

const main = async (): Promise<void> => {
  const backends = discoverBackends();
  if (backends.length === 0) {
    console.error(
      'No backends found. Start dockit or sqlkit first, or set DOCKIT_MCP_PORT / SQLKIT_MCP_PORT environment variables.',
    );
    process.exit(1);
  }

  console.error(`Discovered backends: ${backends.map(b => `${b.name} (${b.baseUrl})`).join(', ')}`);

  const { tools, routeMap } = await buildToolCatalog(backends);
  if (tools.length === 0) {
    console.error('No tools found from any backend.');
    process.exit(1);
  }

  console.error(`Loaded ${tools.length} tools from ${backends.length} backend(s).`);

  const clients = new Map(backends.map(b => [b.name, createBackendClient(b)]));

  const server = new Server(
    { name: 'data-studio-mcp', version: '0.1.0' },
    { capabilities: { tools: {} } },
  );

  server.setRequestHandler(ListToolsRequestSchema, async () => ({
    tools: tools.map(t => ({
      name: t.name,
      description: t.description,
      inputSchema: {
        type: 'object' as const,
        properties: (t.inputSchema as Record<string, unknown>)?.properties as
          Record<string, object> | undefined,
        required: (t.inputSchema as Record<string, unknown>)?.required as string[] | undefined,
      },
    })),
  }));

  server.setRequestHandler(CallToolRequestSchema, async request => {
    const toolName = request.params.name;
    const args = (request.params.arguments ?? {}) as Record<string, unknown>;

    const route = routeMap.get(toolName);
    if (!route) return errorResult(`Unknown tool: ${toolName}`);

    const client = clients.get(route.backendName);
    if (!client) return errorResult(`Backend ${route.backendName} is not available`);

    try {
      const result = await client.invokeTool(route.internalName, args);
      if (result.status >= 400) {
        return errorResult(result.message ?? `Error (HTTP ${result.status})`);
      }
      return okResult(result.data);
    } catch (err) {
      return errorResult(`Bridge error: ${err instanceof Error ? err.message : String(err)}`);
    }
  });

  const transport = new StdioServerTransport();
  await server.connect(transport);
};

main().catch(err => {
  console.error('Fatal error:', err);
  process.exit(1);
});
