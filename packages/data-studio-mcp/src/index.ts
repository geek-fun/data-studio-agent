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

import { launchHint, type BackendName } from './discovery.js';
import { listConnections } from './connections.js';
import { buildToolAnnotations, type McpToolDef } from './tools.js';
import {
  buildStatusPayload,
  GET_STATUS_TOOL,
  GET_STATUS_TOOL_DEF,
  SERVER_VERSION,
  type BackendStatus,
  type RegistrySnapshot,
} from './status.js';
import { BackendRegistry } from './registry.js';

export const LIST_CONNECTIONS_TOOL = 'data_studio__list_connections';

export type BuildServerOptions = {
  getSnapshot: () => RegistrySnapshot;
  readonly: boolean;
};

export const parseReadonly = (argv: readonly string[]): boolean =>
  argv.includes('--readonly') || argv.includes('--read-only');

export const filterReadOnly = (tools: readonly McpToolDef[]): McpToolDef[] =>
  tools.filter(
    t =>
      t.name === LIST_CONNECTIONS_TOOL ||
      t.name === GET_STATUS_TOOL ||
      t.metadata?.riskLevel === 'safe',
  );

const LIST_CONNECTIONS_TOOL_DEF = {
  name: LIST_CONNECTIONS_TOOL,
  description:
    'List all database connections from both dockit and sqlkit. Each entry has { backend, id, name, type } — use id as connection_id when calling database tools. Call this when a task needs to read/query a database (row counts, table contents, schema) to pick the right connection before executing SQL — never fall back to psql/mysql CLIs. If no connections appear, call data_studio__get_status. Report results in the user\'s language (中文/English).',
  inputSchema: { type: 'object' as const, properties: {}, required: [] },
  annotations: buildToolAnnotations('safe'),
} as const;

const SYNTHETIC_TOOLS = [LIST_CONNECTIONS_TOOL_DEF, GET_STATUS_TOOL_DEF];

const okResult = (data: unknown): CallToolResult => ({
  content: [
    { type: 'text', text: typeof data === 'string' ? data : JSON.stringify(data, null, 2) },
  ],
});

const structuredError = (
  error: string,
  text: string,
  extra: Record<string, unknown> = {},
): CallToolResult => ({
  content: [{ type: 'text', text }],
  isError: true,
  structuredContent: { error, ...extra },
});

const remediationFor = (backendName: BackendName, status: BackendStatus | undefined): string => {
  if (status && status.status === 'unavailable' && status.hint) return status.hint;
  return launchHint(backendName);
};

export const buildServer = ({ getSnapshot, readonly }: BuildServerOptions): Server => {
  const server = new Server(
    { name: 'data-studio-mcp', version: SERVER_VERSION },
    { capabilities: { tools: { listChanged: true }, logging: {} } },
  );

  server.setRequestHandler(ListToolsRequestSchema, async () => {
    const snapshot = getSnapshot();
    const listed = readonly ? filterReadOnly(snapshot.tools) : snapshot.tools;
    return {
      tools: [
        ...SYNTHETIC_TOOLS.map(t => ({
          name: t.name,
          description: t.description,
          inputSchema: { type: 'object' as const, properties: {}, required: [] },
          annotations: t.annotations,
        })),
        ...listed.map(t => ({
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
    };
  });

  server.setRequestHandler(CallToolRequestSchema, async request => {
    const toolName = request.params.name;
    const args = (request.params.arguments ?? {}) as Record<string, unknown>;
    const snapshot = getSnapshot();

    if (toolName === LIST_CONNECTIONS_TOOL) {
      return okResult(await listConnections(snapshot.clients));
    }

    if (toolName === GET_STATUS_TOOL) {
      return okResult(buildStatusPayload(snapshot));
    }

    const route = snapshot.routeMap.get(toolName);
    if (!route) {
      return structuredError(
        'UNKNOWN_TOOL',
        `Unknown tool: ${toolName}. Call data_studio__get_status to see available backends and tools.`,
        { tool: toolName },
      );
    }

    if (readonly) {
      const toolDef = snapshot.tools.find(t => t.name === toolName);
      if (toolDef?.metadata?.riskLevel !== 'safe') {
        return structuredError(
          'READONLY_MODE',
          `Tool ${toolName} is not available in read-only mode. Call data_studio__get_status to see the available tools.`,
          { tool: toolName },
        );
      }
    }

    const client = snapshot.clients.get(route.backendName);
    const status = snapshot.statuses.find(s => s.name === route.backendName);
    if (!client || !status || status.status !== 'connected') {
      const remediation = remediationFor(route.backendName, status);
      return structuredError(
        'BACKEND_UNAVAILABLE',
        `${route.backendName} backend unavailable${status?.port ? ` (port ${status.port})` : ''}. Fix: ${remediation}. Check: data_studio__get_status.`,
        { backend: route.backendName, port: status?.port ?? null, remediation },
      );
    }

    try {
      const result = await client.invokeTool(route.internalName, args);
      if (result.status >= 400) {
        return structuredError('BACKEND_ERROR', result.message ?? `Error (HTTP ${result.status})`, {
          backend: route.backendName,
          status: result.status,
          message: result.message ?? null,
        });
      }
      return okResult(result.data);
    } catch (err) {
      const detail = err instanceof Error ? err.message : String(err);
      const remediation = remediationFor(route.backendName, status);
      return structuredError(
        'BACKEND_UNAVAILABLE',
        `${route.backendName} backend unavailable (${detail}). Fix: ${remediation}. Check: data_studio__get_status.`,
        { backend: route.backendName, port: status.port, remediation },
      );
    }
  });

  return server;
};

export const main = async (): Promise<void> => {
  const readonly = parseReadonly(process.argv.slice(2));
  const registry = new BackendRegistry({ readonly });

  const snapshot = await registry.start();

  console.error(
    `Discovered backends: ${snapshot.statuses
      .map(s => `${s.name} (${s.port ?? 'no port'})`)
      .join(', ')}`,
  );
  const connectedCount = snapshot.statuses.filter(s => s.status === 'connected').length;
  if (connectedCount > 0) {
    console.error(`Loaded ${snapshot.tools.length} tools from ${connectedCount} backend(s).`);
  } else {
    console.error(
      'No backends reachable. Server stays up; call data_studio__get_status for remediation.',
    );
  }
  if (readonly) {
    console.error('Read-only mode: exposing only safe tools.');
  }

  const server = buildServer({ getSnapshot: () => registry.snapshot(), readonly });

  const transport = new StdioServerTransport();
  await server.connect(transport);

  registry.onToolsChanged = () => {
    server.sendToolListChanged().catch(() => {
      // session may be closing — notification is best-effort
    });
  };

  if (connectedCount === 0) {
    await server
      .sendLoggingMessage({
        level: 'warning',
        logger: 'startup',
        data: 'No backends reachable. Call data_studio__get_status for remediation hints.',
      })
      .catch(() => {
        // client may not surface server logs — best-effort
      });
  }
};

const isMainEntry =
  process.argv[1] !== undefined && import.meta.url === pathToFileURL(resolve(process.argv[1])).href;

if (isMainEntry) {
  main().catch(err => {
    console.error('Fatal error:', err);
    process.exit(1);
  });
}
