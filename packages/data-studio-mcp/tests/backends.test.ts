import { describe, it, expect, afterAll } from 'vitest';
import { createServer, type Server } from 'node:http';
import { createBackendClient, probeBackend } from '../src/backends.js';

let mockServer: Server | null = null;
let mockPort = 0;
let lastInvokeBody: unknown = null;

const startMock = (failTools = false): Promise<number> => {
  return new Promise(resolve => {
    mockServer = createServer((req, res) => {
      if (req.method === 'POST' && req.url === '/tools') {
        if (failTools) {
          res.writeHead(500);
          res.end('Internal Server Error');
          return;
        }
        res.writeHead(200, { 'Content-Type': 'application/json' });
        res.end(
          JSON.stringify({
            tools: [
              {
                name: 'es__search',
                description: 'Search',
                inputSchema: { type: 'object' },
                metadata: { riskLevel: 'safe' },
              },
            ],
            connections: [{ id: 1, name: 'prod-es', type: 'ELASTICSEARCH' }],
          }),
        );
      } else if (req.method === 'POST' && req.url === '/invoke') {
        let raw = '';
        req.on('data', chunk => (raw += chunk));
        req.on('end', () => {
          lastInvokeBody = JSON.parse(raw);
          res.writeHead(200, { 'Content-Type': 'application/json' });
          res.end(
            JSON.stringify({
              status: 200,
              data: { hits: [{ _source: { title: 'doc1' } }] },
            }),
          );
        });
      } else {
        res.writeHead(404);
        res.end();
      }
    });
    mockServer.listen(0, '127.0.0.1', () => {
      const addr = mockServer!.address();
      resolve(typeof addr === 'object' ? addr!.port : 0);
    });
  });
};

afterAll(() => {
  mockServer?.close();
});

describe('BackendClient', () => {
  it('listTools returns tools and connections from the bridge', async () => {
    mockPort = await startMock();
    const client = createBackendClient({
      name: 'dockit',
      port: mockPort,
      baseUrl: `http://127.0.0.1:${mockPort}`,
    });

    const catalog = await client.listTools();
    expect(catalog.tools).toHaveLength(1);
    expect(catalog.tools[0].name).toBe('es__search');
    expect(catalog.connections).toEqual([{ id: 1, name: 'prod-es', type: 'ELASTICSEARCH' }]);
  });

  it('listTools throws when bridge returns error', async () => {
    const port = await startMock(true);
    const client = createBackendClient({
      name: 'dockit',
      port,
      baseUrl: `http://127.0.0.1:${port}`,
    });

    await expect(client.listTools()).rejects.toThrow();
  });

  it('listTools throws when bridge is unreachable', async () => {
    const client = createBackendClient({
      name: 'dockit',
      port: 1,
      baseUrl: 'http://127.0.0.1:1',
    });

    await expect(client.listTools()).rejects.toThrow();
  });

  it('invokeTool returns result from the bridge', async () => {
    if (!mockPort) mockPort = await startMock();
    const client = createBackendClient({
      name: 'dockit',
      port: mockPort,
      baseUrl: `http://127.0.0.1:${mockPort}`,
    });

    const result = await client.invokeTool('es__search', { index: 'test' });
    expect(result.status).toBe(200);
    expect(result.data).toEqual({ hits: [{ _source: { title: 'doc1' } }] });
  });

  it('invokeTool hoists connection_id to the top level of the request body', async () => {
    if (!mockPort) mockPort = await startMock();
    const client = createBackendClient({
      name: 'dockit',
      port: mockPort,
      baseUrl: `http://127.0.0.1:${mockPort}`,
    });

    const result = await client.invokeTool('sqlkit__list_databases', {
      connection_id: 'si-console-local',
    });
    expect(result.status).toBe(200);
    expect(lastInvokeBody).toEqual({
      name: 'sqlkit__list_databases',
      args: {},
      connection_id: 'si-console-local',
    });
  });

  it('invokeTool leaves the request body untouched when no connection_id is given', async () => {
    if (!mockPort) mockPort = await startMock();
    const client = createBackendClient({
      name: 'dockit',
      port: mockPort,
      baseUrl: `http://127.0.0.1:${mockPort}`,
    });

    await client.invokeTool('es__search', { index: 'test', size: 10 });
    expect(lastInvokeBody).toEqual({
      name: 'es__search',
      args: { index: 'test', size: 10 },
    });
  });

  it('exposes name and baseUrl', async () => {
    const client = createBackendClient({
      name: 'dockit',
      port: 9120,
      baseUrl: 'http://127.0.0.1:9120',
    });

    expect(client.name).toBe('dockit');
    expect(client.baseUrl).toBe('http://127.0.0.1:9120');
  });
});

describe('probeBackend', () => {
  it('returns true for a reachable backend', async () => {
    const port = await startMock();
    const ok = await probeBackend({
      name: 'dockit',
      port,
      baseUrl: `http://127.0.0.1:${port}`,
    });
    expect(ok).toBe(true);
  });

  it('returns false for an unreachable backend', async () => {
    const ok = await probeBackend({ name: 'dockit', port: 1, baseUrl: 'http://127.0.0.1:1' });
    expect(ok).toBe(false);
  });
});
