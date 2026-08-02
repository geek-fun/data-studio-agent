#!/usr/bin/env node

import { resolve } from 'node:path';
import { pathToFileURL } from 'node:url';

import { Server } from '@modelcontextprotocol/sdk/server/index.js';
import { StdioServerTransport } from '@modelcontextprotocol/sdk/server/stdio.js';
import {
  ListToolsRequestSchema,
  CallToolRequestSchema,
  type CallToolResult,
} from '@modelcontextprotocol/sdk/types.js';

import { discoverBackends } from './discovery.js';
import { createBackendClient, type BackendClient } from './backends.js';
import { buildToolCatalog, buildToolAnnotations, type McpToolDef } from './tools.js';
import { listConnections } from './connections.js';

export const LIST_CONNECTIONS_TOOL = 'data_studio__list_connections';

export type BuildServerOptions = {
  tools: readonly McpToolDef[];
  routeMap: Map<string, { backendName: 'dockit' | 'sqlkit'; internalName: string }>;
  clients: Map<string, BackendClient>;
  readonly: boolean;
};

export const parseReadonly = (argv: readonly string[]): boolean =>
  argv.includes('--readonly') || argv.includes('--read-only');

export const filterReadOnly = (tools: readonly McpToolDef[]): McpToolDef[] =>
  tools.filter(t => t.name === LIST_CONNECTIONS_TOOL || t.metadata?.riskLevel === 'safe');

const errorResult = (text: string): CallToolResult => ({
  content: [{ type: 'text', text }],
  isError: true,
});

const okResult = (data: unknown): CallToolResult => ({
  content: [
    { type: 'text', text: typeof data === 'string' ? data : JSON.stringify(data, null, 2) },
  ],
});

export const buildServer = ({ tools, routeMap, clients, readonly }: BuildServerOptions): Server => {
  const effectiveTools = readonly ? filterReadOnly(tools) : tools;

  const server = new Server(
    { name: 'data-studio-mcp', version: '0.1.0' },
    { capabilities: { tools: {} } },
  );

  server.setRequestHandler(ListToolsRequestSchema, async () => ({
    tools: [
      {
        name: LIST_CONNECTIONS_TOOL,
        description:
          'List all database connections from both dockit and sqlkit. Each entry has { backend, id, name, type } — use id as connection_id when calling database tools.',
        inputSchema: { type: 'object' as const, properties: {}, required: [] },
        annotations: buildToolAnnotations('safe'),
      },
      ...effectiveTools.map(t => ({
        name: t.name,
        description: t.description,
        inputSchema: {
          type: 'object' as const,
          properties: (t.inputSchema as Record<string, unknown>)?.properties as
            Record<string, object> | undefined,
          required: (t.inputSchema as Record<string, unknown>)?.required as string[] | undefined,
        },
        annotations: t.annotations,
      })),
    ],
  }));

  server.setRequestHandler(CallToolRequestSchema, async request => {
    const toolName = request.params.name;
    const args = (request.params.arguments ?? {}) as Record<string, unknown>;

    if (toolName === LIST_CONNECTIONS_TOOL) {
      return okResult(await listConnections(clients));
    }

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

  return server;
};

export const main = async (): Promise<void> => {
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

  const readonly = parseReadonly(process.argv.slice(2));
  const filteredTools = readonly ? filterReadOnly(tools) : tools;
  if (readonly) {
    console.error(`Read-only mode: exposing ${filteredTools.length} of ${tools.length} tools.`);
  }

  const clients = new Map(backends.map(b => [b.name, createBackendClient(b)]));

  const server = buildServer({ tools: filteredTools, routeMap, clients, readonly });

  const transport = new StdioServerTransport();
  await server.connect(transport);
};

const isMainEntry =
  process.argv[1] !== undefined && import.meta.url === pathToFileURL(resolve(process.argv[1])).href;

if (isMainEntry) {
  main().catch(err => {
    console.error('Fatal error:', err);
    process.exit(1);
  });
}
