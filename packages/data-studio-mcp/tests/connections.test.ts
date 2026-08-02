import { describe, it, expect, afterAll } from 'vitest';
import { createServer, type Server } from 'node:http';
import { createBackendClient } from '../src/backends.js';
import { listConnections } from '../src/connections.js';

const servers: Server[] = [];

const startBackend = async (
  connections: Array<{ id: number; name: string; type: string }>,
): Promise<number> => {
  return new Promise(resolve => {
    const server = createServer((req, res) => {
      res.writeHead(200, { 'Content-Type': 'application/json' });
      res.end(JSON.stringify({ tools: [], connections }));
    });
    servers.push(server);
    server.listen(0, '127.0.0.1', () => {
      const addr = server.address();
      resolve(typeof addr === 'object' ? addr!.port : 0);
    });
  });
};

afterAll(() => {
  servers.forEach(s => s.close());
});

describe('listConnections', () => {
  it('merges connections from both backends with backend tag', async () => {
    const dockitPort = await startBackend([{ id: 1, name: 'prod-es', type: 'ELASTICSEARCH' }]);
    const sqlkitPort = await startBackend([{ id: 2, name: 'analytics-pg', type: 'POSTGRESQL' }]);

    const clients = new Map([
      [
        'dockit',
        createBackendClient({
          name: 'dockit',
          port: dockitPort,
          baseUrl: `http://127.0.0.1:${dockitPort}`,
        }),
      ],
      [
        'sqlkit',
        createBackendClient({
          name: 'sqlkit',
          port: sqlkitPort,
          baseUrl: `http://127.0.0.1:${sqlkitPort}`,
        }),
      ],
    ]);

    const result = await listConnections(clients);
    expect(result).toEqual([
      { backend: 'dockit', id: 1, name: 'prod-es', type: 'ELASTICSEARCH' },
      { backend: 'sqlkit', id: 2, name: 'analytics-pg', type: 'POSTGRESQL' },
    ]);
  });

  it('skips unreachable backends', async () => {
    const dockitPort = await startBackend([{ id: 1, name: 'prod-es', type: 'ELASTICSEARCH' }]);

    const clients = new Map([
      [
        'dockit',
        createBackendClient({
          name: 'dockit',
          port: dockitPort,
          baseUrl: `http://127.0.0.1:${dockitPort}`,
        }),
      ],
      ['sqlkit', createBackendClient({ name: 'sqlkit', port: 1, baseUrl: 'http://127.0.0.1:1' })],
    ]);

    const result = await listConnections(clients);
    expect(result).toEqual([{ backend: 'dockit', id: 1, name: 'prod-es', type: 'ELASTICSEARCH' }]);
  });

  it('returns empty list when no backends have connections', async () => {
    const dockitPort = await startBackend([]);
    const clients = new Map([
      [
        'dockit',
        createBackendClient({
          name: 'dockit',
          port: dockitPort,
          baseUrl: `http://127.0.0.1:${dockitPort}`,
        }),
      ],
    ]);

    const result = await listConnections(clients);
    expect(result).toEqual([]);
  });
});
