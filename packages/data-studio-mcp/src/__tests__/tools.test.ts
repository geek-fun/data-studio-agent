import { describe, it, expect } from 'vitest';
import { buildToolCatalog } from '../tools.js';

const mockBackends = [
  {
    name: 'dockit' as const,
    port: 9120,
    baseUrl: 'http://127.0.0.1:9120',
  },
  {
    name: 'sqlkit' as const,
    port: 9121,
    baseUrl: 'http://127.0.0.1:9121',
  },
];

describe('buildToolCatalog', () => {
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
