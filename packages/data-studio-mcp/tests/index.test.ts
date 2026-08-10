import { readFileSync } from 'node:fs';

import { describe, it, expect } from 'vitest';
import { InMemoryTransport } from '@modelcontextprotocol/sdk/inMemory.js';
import { Client } from '@modelcontextprotocol/sdk/client/index.js';
import { Server } from '@modelcontextprotocol/sdk/server/index.js';
import { ToolListChangedNotificationSchema } from '@modelcontextprotocol/sdk/types.js';
import {
  buildServer,
  parseReadonly,
  filterReadOnly,
  LIST_CONNECTIONS_TOOL,
  type BuildServerOptions,
} from '../src/index.js';
import {
  GET_STATUS_TOOL,
  SERVER_VERSION,
  type BackendStatus,
  type RegistrySnapshot,
} from '../src/status.js';
import { buildToolAnnotations, type McpToolDef, type Route } from '../src/tools.js';
import type { BackendClient } from '../src/backends.js';

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

const makeSnapshot = (overrides: Partial<RegistrySnapshot> = {}): RegistrySnapshot => ({
  tools: [],
  routeMap: new Map(),
  clients: new Map(),
  statuses: [],
  connections: {},
  version: '0.1.5',
  uptimeSeconds: 0,
  readOnly: false,
  ...overrides,
});

const makeOpts = (
  overrides: { snapshot?: RegistrySnapshot; readonly?: boolean } = {},
): BuildServerOptions => ({
  getSnapshot: () => overrides.snapshot ?? makeSnapshot(),
  readonly: overrides.readonly ?? false,
});

const connectPair = async (server: Server) => {
  const [clientTransport, serverTransport] = InMemoryTransport.createLinkedPair();
  const client = new Client({ name: 'test-client', version: '0.0.0' });
  await server.connect(serverTransport);
  await client.connect(clientTransport);
  return { client, clientTransport, serverTransport };
};

const textOf = (result: { content: Array<{ type: string; text?: string }> }): string =>
  result.content.find(c => c.type === 'text')?.text ?? '';

describe('SERVER_VERSION', () => {
  it('matches the version in package.json (never drifts)', () => {
    const pkg = JSON.parse(readFileSync(new URL('../package.json', import.meta.url), 'utf8')) as {
      version: string;
    };
    expect(SERVER_VERSION).toBe(pkg.version);
  });
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

  it('always keeps the synthetic get_status tool', () => {
    const tools = [makeTool(GET_STATUS_TOOL)];
    expect(filterReadOnly(tools)).toEqual([makeTool(GET_STATUS_TOOL)]);
  });
});

describe('buildServer', () => {
  it('returns an MCP Server instance', () => {
    expect(buildServer(makeOpts())).toBeInstanceOf(Server);
  });

  it('S1: stays up with zero backends and always registers both synthetic tools', async () => {
    const { client } = await connectPair(buildServer(makeOpts()));
    const { tools } = await client.listTools();
    expect(tools.map(t => t.name)).toEqual([LIST_CONNECTIONS_TOOL, GET_STATUS_TOOL]);
  });

  it('emits annotations in tools/list', async () => {
    const snapshot = makeSnapshot({
      tools: [
        makeTool('data_studio__dockit__es_search', 'safe'),
        makeTool('data_studio__dockit__es_delete_index', 'destructive'),
      ],
    });
    const { client } = await connectPair(buildServer(makeOpts({ snapshot })));

    const { tools } = await client.listTools();
    const names = tools.map(t => t.name);
    expect(names).toContain('data_studio__dockit__es_search');
    expect(names).toContain('data_studio__dockit__es_delete_index');
    expect(names).toContain(LIST_CONNECTIONS_TOOL);
    expect(names).toContain(GET_STATUS_TOOL);

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

    const getStatus = tools.find(t => t.name === GET_STATUS_TOOL);
    expect(getStatus?.annotations).toEqual({
      readOnlyHint: true,
      destructiveHint: false,
      idempotentHint: true,
      openWorldHint: false,
    });
  });

  it('exposes only safe tools plus synthetic tools when readonly is true', async () => {
    const snapshot = makeSnapshot({
      tools: [
        makeTool('data_studio__dockit__es_search', 'safe'),
        makeTool('data_studio__dockit__es_delete_index', 'destructive'),
        makeTool('data_studio__dockit__es_update', 'elevated'),
      ],
    });
    const { client } = await connectPair(buildServer(makeOpts({ snapshot, readonly: true })));

    const { tools } = await client.listTools();
    expect(tools.map(t => t.name)).toEqual([
      LIST_CONNECTIONS_TOOL,
      GET_STATUS_TOOL,
      'data_studio__dockit__es_search',
    ]);
  });

  it('rejects non-safe tool calls in readonly mode even if the client knows the name', async () => {
    const routeMap = new Map<string, Route>([
      [
        'data_studio__dockit__es_delete_index',
        { backendName: 'dockit', internalName: 'es_delete_index' },
      ],
    ]);
    const clients = new Map<string, BackendClient>([
      [
        'dockit',
        {
          name: 'dockit',
          baseUrl: 'http://127.0.0.1:9120',
          listTools: async () => ({ tools: [], connections: [] }),
          invokeTool: async () => ({ status: 200, data: { ok: true } }),
        },
      ],
    ]);
    const statuses: BackendStatus[] = [
      {
        name: 'dockit',
        port: 9120,
        baseUrl: 'http://127.0.0.1:9120',
        status: 'connected',
        toolCount: 1,
        hint: 'Connected on port 9120.',
      },
    ];
    const snapshot = makeSnapshot({
      tools: [makeTool('data_studio__dockit__es_delete_index', 'destructive')],
      routeMap,
      clients,
      statuses,
    });
    const { client } = await connectPair(buildServer(makeOpts({ snapshot, readonly: true })));

    const res = await client.callTool({
      name: 'data_studio__dockit__es_delete_index',
      arguments: {},
    });
    expect(res.isError).toBe(true);
    expect(res.structuredContent).toMatchObject({
      error: 'READONLY_MODE',
      tool: 'data_studio__dockit__es_delete_index',
    });
  });

  it('S2: get_status returns per-backend availability, hints, tool counts and server info', async () => {
    const statuses: BackendStatus[] = [
      {
        name: 'dockit',
        port: 9120,
        baseUrl: 'http://127.0.0.1:9120',
        status: 'unavailable',
        toolCount: 0,
        hint: 'Launch DocKit: open -a DocKit',
      },
      {
        name: 'sqlkit',
        port: null,
        baseUrl: null,
        status: 'unavailable',
        toolCount: 0,
        hint: 'SqlKit is not running or its port file is missing',
      },
    ];
    const snapshot = makeSnapshot({
      statuses,
      connections: {
        sqlkit: [
          { backend: 'sqlkit', id: 'c1', name: 'local-pg', type: 'PostgreSQL' },
          { backend: 'sqlkit', id: 'c2', name: 'local-mysql', type: 'MySQL' },
        ],
      },
    });
    const { client } = await connectPair(buildServer(makeOpts({ snapshot })));

    const res = await client.callTool({ name: GET_STATUS_TOOL });
    expect(res.isError).toBeFalsy();
    const payload = JSON.parse(textOf(res)) as {
      server: { version: string };
      backends: Array<{ name: string; status: string; hint: string; connectionCount: number }>;
      summary: { totalConnections: number; connectionsByType: Record<string, number> };
    };
    expect(payload.server.version).toBe('0.1.5');
    expect(payload.backends).toHaveLength(2);
    expect(payload.backends[0]).toMatchObject({
      name: 'dockit',
      status: 'unavailable',
      hint: 'Launch DocKit: open -a DocKit',
      connectionCount: 0,
    });
    expect(payload.backends[1]).toMatchObject({
      name: 'sqlkit',
      status: 'unavailable',
      connectionCount: 2,
    });
    expect(payload.summary).toMatchObject({
      totalConnections: 2,
      connectionsByType: { PostgreSQL: 1, MySQL: 1 },
    });
  });

  it('S5: calling a backend tool whose backend is unavailable returns a structured error', async () => {
    const routeMap = new Map<string, Route>([
      ['data_studio__dockit__es_search', { backendName: 'dockit', internalName: 'es_search' }],
    ]);
    const snapshot = makeSnapshot({ routeMap });
    const { client } = await connectPair(buildServer(makeOpts({ snapshot })));

    const res = await client.callTool({
      name: 'data_studio__dockit__es_search',
      arguments: { index: 'orders' },
    });
    expect(res.isError).toBe(true);
    expect(res.structuredContent).toMatchObject({
      error: 'BACKEND_UNAVAILABLE',
      backend: 'dockit',
    });
    expect(textOf(res)).toContain('DocKit');
    expect(textOf(res)).toContain('data_studio__get_status');
  });

  it('S5: a bridge failure during a tool call returns a structured error with remediation', async () => {
    const failingClient: BackendClient = {
      name: 'dockit',
      baseUrl: 'http://127.0.0.1:9120',
      listTools: async () => ({ tools: [], connections: [] }),
      invokeTool: async () => {
        throw new Error('connect ECONNREFUSED 127.0.0.1:9120');
      },
    };
    const routeMap = new Map<string, Route>([
      ['data_studio__dockit__es_search', { backendName: 'dockit', internalName: 'es_search' }],
    ]);
    const statuses: BackendStatus[] = [
      {
        name: 'dockit',
        port: 9120,
        baseUrl: 'http://127.0.0.1:9120',
        status: 'connected',
        toolCount: 1,
        hint: 'Connected on port 9120.',
      },
    ];
    const snapshot = makeSnapshot({
      routeMap,
      statuses,
      clients: new Map([['dockit', failingClient]]),
    });
    const { client } = await connectPair(buildServer(makeOpts({ snapshot })));

    const res = await client.callTool({
      name: 'data_studio__dockit__es_search',
      arguments: { index: 'orders' },
    });
    expect(res.isError).toBe(true);
    expect(res.structuredContent).toMatchObject({
      error: 'BACKEND_UNAVAILABLE',
      backend: 'dockit',
      port: 9120,
    });
    expect(textOf(res)).toContain('DocKit');
  });

  it('S5: an unknown tool returns a structured error pointing at get_status', async () => {
    const { client } = await connectPair(buildServer(makeOpts()));

    const res = await client.callTool({ name: 'data_studio__nope', arguments: {} });
    expect(res.isError).toBe(true);
    expect(res.structuredContent).toMatchObject({ error: 'UNKNOWN_TOOL' });
    expect(textOf(res)).toContain('data_studio__get_status');
  });

  it('S3: delivers tools/list_changed notifications to the client', async () => {
    const server = buildServer(makeOpts());
    const { client } = await connectPair(server);
    let changed = 0;
    client.setNotificationHandler(ToolListChangedNotificationSchema, () => {
      changed += 1;
    });

    await server.sendToolListChanged();
    await new Promise(r => setTimeout(r, 0));
    expect(changed).toBe(1);
  });

  it('R1: lists connections resource and a schema resource per connected backend', async () => {
    const clientStub: BackendClient = {
      name: 'dockit',
      baseUrl: 'http://127.0.0.1:9120',
      listTools: async () => ({
        tools: [],
        connections: [{ id: '1', name: 'local-es', type: 'Elasticsearch' }],
      }),
      invokeTool: async (name: string) => {
        if (name === 'es__cat_indices') {
          return { status: 200, data: [{ index: 'users' }, { index: 'orders' }] };
        }
        if (name === 'es__get_mapping') {
          return { status: 200, data: { properties: { title: { type: 'text' } } } };
        }
        return { status: 200, data: '' };
      },
    };
    const snapshot = makeSnapshot({
      clients: new Map([['dockit', clientStub]]),
      connections: {
        dockit: [{ backend: 'dockit', id: '1', name: 'local-es', type: 'Elasticsearch' }],
      },
      statuses: [
        {
          name: 'dockit',
          port: 9120,
          baseUrl: 'http://127.0.0.1:9120',
          status: 'connected',
          toolCount: 0,
          hint: 'Connected',
        },
      ],
    });
    const { client } = await connectPair(buildServer(makeOpts({ snapshot })));

    const { resources } = await client.listResources();
    const uris = resources.map(r => r.uri);
    expect(uris).toContain('data-studio://connections');
    expect(uris).toContain('data-studio://dockit/1/schema');

    const read = await client.readResource({
      uri: 'data-studio://dockit/1/schema',
    });
    expect(read.contents[0].text).toContain('users');
  });

  it('R2: exposes resource templates and prompts', async () => {
    const { client } = await connectPair(buildServer(makeOpts()));

    const { resourceTemplates } = await client.listResourceTemplates();
    expect(resourceTemplates[0].uriTemplate).toContain('{connection_id}');

    const { prompts } = await client.listPrompts();
    const names = prompts.map(p => p.name);
    expect(names).toContain('explore-database');
    expect(names).toContain('debug-query');

    const got = await client.getPrompt({
      name: 'inspect-table',
      arguments: { connection_id: 'c1', table: 'users' },
    });
    expect(got.messages[0].content).toMatchObject({
      type: 'text',
      text: expect.stringContaining('users'),
    });
  });

  it('C1: completes connection ids and database names', async () => {
    const clientStub: BackendClient = {
      name: 'sqlkit',
      baseUrl: 'http://127.0.0.1:9121',
      listTools: async () => ({
        tools: [],
        connections: [{ id: 'conn-1', name: 'local-pg', type: 'PostgreSQL' }],
      }),
      invokeTool: async (_name, _args) => ({
        status: 200,
        data: { data: [{ name: 'si_dev' }, { name: 'postgres' }] },
      }),
    };
    const snapshot = makeSnapshot({
      clients: new Map([['sqlkit', clientStub]]),
    });
    const { client } = await connectPair(buildServer(makeOpts({ snapshot })));

    const connCompletion = await client.complete({
      ref: { type: 'ref/prompt', name: 'explore-database' },
      argument: { name: 'connection_id', value: '' },
    });
    expect(connCompletion.completion.values).toContain('conn-1');

    const dbCompletion = await client.complete({
      ref: { type: 'ref/prompt', name: 'explore-database' },
      argument: { name: 'database', value: '' },
    });
    expect(dbCompletion.completion.values).toContain('si_dev');
  });

  it('SC1: execute_query results carry structuredContent', async () => {
    const clientStub: BackendClient = {
      name: 'sqlkit',
      baseUrl: 'http://127.0.0.1:9121',
      listTools: async () => ({ tools: [], connections: [] }),
      invokeTool: async () => ({
        status: 200,
        data: {
          columns: ['id', 'name'],
          rows: [{ id: 1, name: 'a' }],
        },
      }),
    };
    const routeMap = new Map<string, Route>([
      [
        'data_studio__sqlkit__execute_query',
        { backendName: 'sqlkit', internalName: 'execute_query' },
      ],
    ]);
    const tools = [
      {
        ...makeTool('execute_query', 'safe'),
        name: 'data_studio__sqlkit__execute_query',
        backendName: 'sqlkit' as const,
      },
    ];
    const snapshot = makeSnapshot({
      routeMap,
      tools,
      clients: new Map([['sqlkit', clientStub]]),
      statuses: [
        {
          name: 'sqlkit',
          port: 9121,
          baseUrl: 'http://127.0.0.1:9121',
          status: 'connected',
          toolCount: 1,
          hint: 'Connected',
        },
      ],
    });
    const { client } = await connectPair(buildServer(makeOpts({ snapshot })));

    const res = await client.callTool({
      name: 'data_studio__sqlkit__execute_query',
      arguments: { connection_id: 'c1', sql: 'SELECT id, name FROM users' },
    });
    expect(res.isError).toBeFalsy();
    expect(res.structuredContent).toMatchObject({
      columns: ['id', 'name'],
      rowCount: 1,
    });
  });
});
