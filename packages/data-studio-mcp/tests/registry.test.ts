import { describe, it, expect, afterEach } from 'vitest';
import { createServer, type Server } from 'node:http';
import { BackendRegistry } from '../src/registry.js';
import type { BackendInfo } from '../src/discovery.js';

const mockServers: Server[] = [];
const registries: BackendRegistry[] = [];

const startMockBackend = async (tools: unknown[]): Promise<number> => {
  return new Promise(resolve => {
    const server = createServer((req, res) => {
      if (req.method === 'POST' && req.url === '/tools') {
        res.writeHead(200, { 'Content-Type': 'application/json' });
        res.end(JSON.stringify({ tools, connections: [] }));
      } else {
        res.writeHead(200, { 'Content-Type': 'application/json' });
        res.end('{}');
      }
    });
    mockServers.push(server);
    server.listen(0, '127.0.0.1', () => {
      const addr = server.address();
      resolve(typeof addr === 'object' ? addr!.port : 0);
    });
  });
};

afterEach(() => {
  mockServers.splice(0).forEach(s => s.close());
  registries.splice(0).forEach(r => r.stop());
});

const makeRegistry = (
  opts: {
    candidates?: BackendInfo[];
    reachable?: Set<string>;
    pollIntervalMs?: number;
  } = {},
): BackendRegistry => {
  const registry = new BackendRegistry({
    readonly: false,
    pollIntervalMs: opts.pollIntervalMs ?? 60_000,
    discover: () => opts.candidates ?? [],
    probe: async info => opts.reachable?.has(info.name) ?? false,
  });
  registries.push(registry);
  return registry;
};

const searchTool = {
  name: 'es_search',
  description: 'Search an index',
  inputSchema: { type: 'object' },
  metadata: { riskLevel: 'safe' },
};

describe('BackendRegistry', () => {
  it('S2: reports both backends unavailable with hints when none reachable', async () => {
    const registry = makeRegistry();
    await registry.start();
    const snap = registry.snapshot();

    expect(snap.tools).toEqual([]);
    expect(snap.statuses).toHaveLength(2);
    for (const s of snap.statuses) {
      expect(s.status).toBe('unavailable');
      expect(s.hint.length).toBeGreaterThan(0);
      expect(s.toolCount).toBe(0);
    }
  });

  it('S2b: reports discovered-but-unreachable backends with their port and a hint', async () => {
    const candidates: BackendInfo[] = [
      { name: 'sqlkit', port: 9121, baseUrl: 'http://127.0.0.1:9121' },
    ];
    const registry = makeRegistry({ candidates, reachable: new Set() });
    await registry.start();

    const sqlkit = registry.snapshot().statuses.find(s => s.name === 'sqlkit');
    expect(sqlkit?.status).toBe('unavailable');
    expect(sqlkit?.port).toBe(9121);
    expect(sqlkit?.hint).toContain('9121');
  });

  it('S6: registers tools from reachable backends and marks them connected', async () => {
    const port = await startMockBackend([searchTool]);
    const candidates: BackendInfo[] = [
      { name: 'dockit', port, baseUrl: `http://127.0.0.1:${port}` },
    ];
    const registry = makeRegistry({ candidates, reachable: new Set(['dockit']) });
    await registry.start();
    const snap = registry.snapshot();

    expect(snap.tools.map(t => t.name)).toEqual(['data_studio__dockit__es_search']);
    expect(snap.routeMap.has('data_studio__dockit__es_search')).toBe(true);
    expect(snap.clients.has('dockit')).toBe(true);

    const dockit = snap.statuses.find(s => s.name === 'dockit');
    expect(dockit?.status).toBe('connected');
    expect(dockit?.toolCount).toBe(1);
    expect(snap.statuses.find(s => s.name === 'sqlkit')?.status).toBe('unavailable');
  });

  it('S3: registers tools and notifies exactly once when a backend comes online', async () => {
    const port = await startMockBackend([searchTool]);
    let candidates: BackendInfo[] = [];
    const reachable = new Set<string>();
    const registry = new BackendRegistry({
      readonly: false,
      pollIntervalMs: 60_000,
      discover: () => candidates,
      probe: async info => reachable.has(info.name),
    });
    registries.push(registry);
    let onChangeCalls = 0;
    registry.onToolsChanged = () => {
      onChangeCalls += 1;
    };

    await registry.start();
    expect(registry.snapshot().tools).toEqual([]);
    expect(onChangeCalls).toBe(0);

    candidates = [{ name: 'dockit', port, baseUrl: `http://127.0.0.1:${port}` }];
    reachable.add('dockit');
    const changed = await registry.refresh();
    expect(changed).toBe(true);
    expect(onChangeCalls).toBe(1);
    expect(registry.snapshot().tools.map(t => t.name)).toEqual(['data_studio__dockit__es_search']);

    const changedAgain = await registry.refresh();
    expect(changedAgain).toBe(false);
    expect(onChangeCalls).toBe(1);
  });

  it('S3b: removes tools and notifies when a backend goes offline', async () => {
    const port = await startMockBackend([searchTool]);
    const candidates: BackendInfo[] = [
      { name: 'dockit', port, baseUrl: `http://127.0.0.1:${port}` },
    ];
    const reachable = new Set<string>(['dockit']);
    const registry = new BackendRegistry({
      readonly: false,
      pollIntervalMs: 60_000,
      discover: () => candidates,
      probe: async info => reachable.has(info.name),
    });
    registries.push(registry);

    await registry.start();
    expect(registry.snapshot().tools).toHaveLength(1);

    let onChangeCalls = 0;
    registry.onToolsChanged = () => {
      onChangeCalls += 1;
    };

    reachable.delete('dockit');
    const changed = await registry.refresh();
    expect(changed).toBe(true);
    expect(onChangeCalls).toBe(1);
    expect(registry.snapshot().tools).toEqual([]);
    expect(registry.snapshot().clients.has('dockit')).toBe(false);
    expect(registry.snapshot().statuses.find(s => s.name === 'dockit')?.status).toBe('unavailable');
  });

  it('skips overlapping refresh calls while one is in flight', async () => {
    let releaseProbe: () => void = () => {};
    const probeGate = new Promise<void>(resolve => {
      releaseProbe = resolve;
    });
    let probeCalls = 0;
    const registry = new BackendRegistry({
      readonly: false,
      pollIntervalMs: 60_000,
      discover: () => [{ name: 'dockit', port: 9120, baseUrl: 'http://127.0.0.1:9120' }],
      probe: async () => {
        probeCalls += 1;
        await probeGate;
        return false;
      },
    });
    registries.push(registry);

    const first = registry.refresh();
    const second = registry.refresh();
    releaseProbe();

    const [firstResult, secondResult] = await Promise.all([first, second]);
    expect(probeCalls).toBe(1);
    expect(secondResult).toBe(false);
  });
});
