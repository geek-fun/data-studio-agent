import { describe, it, expect, afterAll } from 'vitest';
import { createServer, type Server } from 'node:http';
import {
  buildToolCatalog,
  buildToolAnnotations,
  buildBackendRequirementNote,
} from '../src/tools.js';

const mockServers: Server[] = [];

const startMockBackend = async (tools: unknown[]): Promise<number> => {
  return new Promise(resolve => {
    const server = createServer((req, res) => {
      res.writeHead(200, { 'Content-Type': 'application/json' });
      res.end(JSON.stringify({ tools, connections: [] }));
    });
    mockServers.push(server);
    server.listen(0, '127.0.0.1', () => {
      const addr = server.address();
      resolve(typeof addr === 'object' ? addr!.port : 0);
    });
  });
};

afterAll(() => {
  mockServers.forEach(s => s.close());
});

describe('buildToolAnnotations', () => {
  it('maps safe risk level to read-only hints', () => {
    expect(buildToolAnnotations('safe')).toEqual({
      readOnlyHint: true,
      destructiveHint: false,
      idempotentHint: true,
      openWorldHint: false,
    });
  });

  it('maps elevated risk level to open-world hint', () => {
    expect(buildToolAnnotations('elevated')).toEqual({
      readOnlyHint: false,
      destructiveHint: false,
      idempotentHint: false,
      openWorldHint: true,
    });
  });

  it('maps destructive risk level to destructive hint', () => {
    expect(buildToolAnnotations('destructive')).toEqual({
      readOnlyHint: false,
      destructiveHint: true,
      idempotentHint: false,
      openWorldHint: true,
    });
  });

  it('returns empty annotations when risk level is undefined', () => {
    expect(buildToolAnnotations(undefined)).toEqual({});
  });
});

describe('fetchBackendCatalog', () => {
  it('forwards bridge tool metadata into the catalog entry', async () => {
    const port = await startMockBackend([
      {
        name: 'es__delete_index',
        description: 'Delete an index',
        inputSchema: { type: 'object' },
        metadata: { riskLevel: 'destructive', requiredPermission: 'manage_indices' },
      },
    ]);

    const { tools } = await buildToolCatalog([
      { name: 'dockit', port, baseUrl: `http://127.0.0.1:${port}` },
    ]);

    expect(tools).toHaveLength(1);
    expect(tools[0].metadata).toEqual({
      riskLevel: 'destructive',
      requiredPermission: 'manage_indices',
    });
  });

  it('derives annotations from bridge tool metadata', async () => {
    const port = await startMockBackend([
      {
        name: 'es__search',
        description: 'Search',
        inputSchema: { type: 'object' },
        metadata: { riskLevel: 'safe' },
      },
    ]);

    const { tools } = await buildToolCatalog([
      { name: 'dockit', port, baseUrl: `http://127.0.0.1:${port}` },
    ]);

    expect(tools[0].annotations).toEqual({
      readOnlyHint: true,
      destructiveHint: false,
      idempotentHint: true,
      openWorldHint: false,
    });
  });
});

describe('buildBackendRequirementNote', () => {
  it('includes the backend display name and port', () => {
    const note = buildBackendRequirementNote({
      name: 'dockit',
      port: 9120,
      baseUrl: 'http://127.0.0.1:9120',
    });
    expect(note).toContain('Requires DocKit running on localhost:9120');
    expect(note).toContain('data_studio__get_status');
  });
});

describe('buildToolCatalog', () => {
  it('appends the backend requirement note to every tool description', async () => {
    const port = await startMockBackend([
      {
        name: 'es__search',
        description: 'Search an index',
        inputSchema: { type: 'object' },
        metadata: { riskLevel: 'safe' },
      },
    ]);

    const { tools } = await buildToolCatalog([
      { name: 'dockit', port, baseUrl: `http://127.0.0.1:${port}` },
    ]);

    expect(tools).toHaveLength(1);
    expect(tools[0].description).toContain('Search an index');
    expect(tools[0].description).toContain(`Requires DocKit running on localhost:${port}`);
    expect(tools[0].description).toContain('data_studio__get_status');
  });

  it('returns empty catalog when no backends', async () => {
    const { tools, routeMap } = await buildToolCatalog([]);
    expect(tools).toEqual([]);
    expect(routeMap.size).toBe(0);
  });

  it('skips backends that are unreachable', async () => {
    const { tools, routeMap } = await buildToolCatalog([
      {
        name: 'dockit',
        port: 1,
        baseUrl: 'http://127.0.0.1:1',
      },
    ]);
    expect(tools).toEqual([]);
    expect(routeMap.size).toBe(0);
  });
});
