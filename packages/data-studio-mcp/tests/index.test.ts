import { describe, it, expect } from 'vitest';
import { InMemoryTransport } from '@modelcontextprotocol/sdk/inMemory.js';
import { Client } from '@modelcontextprotocol/sdk/client/index.js';
import { Server } from '@modelcontextprotocol/sdk/server/index.js';
import {
  buildServer,
  parseReadonly,
  filterReadOnly,
  LIST_CONNECTIONS_TOOL,
  type BuildServerOptions,
} from '../src/index.js';
import { buildToolAnnotations, type McpToolDef } from '../src/tools.js';

type RiskLevel = 'safe' | 'elevated' | 'destructive';

const makeTool = (name: string, riskLevel?: RiskLevel): McpToolDef => ({
  name,
  description: `Description for ${name}`,
  inputSchema: { type: 'object', properties: {}, required: [] },
  backendName: 'dockit',
  internalName: name,
  metadata: riskLevel ? { riskLevel } : undefined,
  annotations: buildToolAnnotations(riskLevel),
});

const emptyOpts = (overrides: Partial<BuildServerOptions> = {}): BuildServerOptions => ({
  tools: [],
  routeMap: new Map(),
  clients: new Map(),
  readonly: false,
  ...overrides,
});

describe('parseReadonly', () => {
  it('is true when --readonly is present', () => {
    expect(parseReadonly(['--readonly'])).toBe(true);
  });

  it('is true when --read-only is present', () => {
    expect(parseReadonly(['--read-only'])).toBe(true);
  });

  it('is true when mixed with other flags', () => {
    expect(parseReadonly(['--foo', '--readonly', 'bar'])).toBe(true);
  });

  it('is false when no flag is present', () => {
    expect(parseReadonly([])).toBe(false);
    expect(parseReadonly(['--foo'])).toBe(false);
  });

  it('does not match similar flags', () => {
    expect(parseReadonly(['--readonly-mode', '--read'])).toBe(false);
  });
});

describe('filterReadOnly', () => {
  it('keeps only safe tools', () => {
    const tools = [
      makeTool('data_studio__dockit__es_search', 'safe'),
      makeTool('data_studio__dockit__es_delete', 'destructive'),
      makeTool('data_studio__dockit__es_update', 'elevated'),
    ];
    expect(filterReadOnly(tools).map(t => t.name)).toEqual(['data_studio__dockit__es_search']);
  });

  it('drops tools without an explicit safe risk level', () => {
    const tools = [makeTool('data_studio__dockit__es_unknown')];
    expect(filterReadOnly(tools)).toEqual([]);
  });

  it('always keeps the synthetic list_connections tool', () => {
    const tools = [makeTool(LIST_CONNECTIONS_TOOL)];
    expect(filterReadOnly(tools)).toEqual([makeTool(LIST_CONNECTIONS_TOOL)]);
  });
});

describe('buildServer', () => {
  it('returns an MCP Server instance', () => {
    expect(buildServer(emptyOpts())).toBeInstanceOf(Server);
  });

  it('emits annotations in tools/list', async () => {
    const [clientTransport, serverTransport] = InMemoryTransport.createLinkedPair();
    const client = new Client({ name: 'test-client', version: '0.0.0' });
    const server = buildServer(
      emptyOpts({
        tools: [
          makeTool('data_studio__dockit__es_search', 'safe'),
          makeTool('data_studio__dockit__es_delete_index', 'destructive'),
        ],
      }),
    );

    await server.connect(serverTransport);
    await client.connect(clientTransport);

    const { tools } = await client.listTools();
    const names = tools.map(t => t.name);
    expect(names).toContain('data_studio__dockit__es_search');
    expect(names).toContain('data_studio__dockit__es_delete_index');
    expect(names).toContain(LIST_CONNECTIONS_TOOL);

    const search = tools.find(t => t.name === 'data_studio__dockit__es_search');
    expect(search?.annotations).toEqual({
      readOnlyHint: true,
      destructiveHint: false,
      idempotentHint: true,
      openWorldHint: false,
    });

    const destructive = tools.find(t => t.name === 'data_studio__dockit__es_delete_index');
    expect(destructive?.annotations).toEqual({
      readOnlyHint: false,
      destructiveHint: true,
      idempotentHint: false,
      openWorldHint: true,
    });

    const listConnections = tools.find(t => t.name === LIST_CONNECTIONS_TOOL);
    expect(listConnections?.annotations).toEqual({
      readOnlyHint: true,
      destructiveHint: false,
      idempotentHint: true,
      openWorldHint: false,
    });
  });

  it('exposes only safe tools plus list_connections when readonly is true', async () => {
    const [clientTransport, serverTransport] = InMemoryTransport.createLinkedPair();
    const client = new Client({ name: 'test-client', version: '0.0.0' });
    const server = buildServer(
      emptyOpts({
        tools: [
          makeTool('data_studio__dockit__es_search', 'safe'),
          makeTool('data_studio__dockit__es_delete_index', 'destructive'),
          makeTool('data_studio__dockit__es_update', 'elevated'),
        ],
        readonly: true,
      }),
    );

    await server.connect(serverTransport);
    await client.connect(clientTransport);

    const { tools } = await client.listTools();
    expect(tools.map(t => t.name)).toEqual([
      LIST_CONNECTIONS_TOOL,
      'data_studio__dockit__es_search',
    ]);
  });
});
