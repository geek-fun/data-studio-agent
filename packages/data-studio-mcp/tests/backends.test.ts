import { describe, it, expect, afterAll } from 'vitest';
import { createServer, type Server } from 'node:http';
import { createBackendClient } from '../src/backends.js';

let mockServer: Server | null = null;
let mockPort = 0;

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
        res.writeHead(200, { 'Content-Type': 'application/json' });
        res.end(
          JSON.stringify({
            status: 200,
            data: { hits: [{ _source: { title: 'doc1' } }] },
          }),
        );
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
