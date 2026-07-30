import { describe, it, expect, beforeEach, afterEach } from 'vitest';
import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';

// Mock fs and os before importing discovery
const testDir = fs.mkdtempSync(path.join(os.tmpdir(), 'mcp-test-'));

beforeEach(() => {
  // Clean any leftover env vars
  delete process.env.DOCKIT_MCP_PORT;
  delete process.env.SQLKIT_MCP_PORT;
});

afterEach(() => {
  // Clean up test dir contents
  try {
    fs.rmSync(testDir, { recursive: true, force: true });
  } catch {
    // ignore
  }
});

// Need to dynamically import after setting up mock dir
async function getDiscovery() {
  // Recreate test dir
  fs.mkdirSync(testDir, { recursive: true });
  return await import('../discovery.js');
}

describe('discoverBackends', () => {
  it('returns empty array when no backends found', async () => {
    const { discoverBackends } = await getDiscovery();
    // No port files exist and no env vars set
    const backends = discoverBackends();
    expect(backends).toEqual([]);
  });

  it('discovers dockit via DOCKIT_MCP_PORT env var', async () => {
    process.env.DOCKIT_MCP_PORT = '9120';
    const { discoverBackends } = await getDiscovery();
    const backends = discoverBackends();
    expect(backends).toHaveLength(1);
    expect(backends[0]).toMatchObject({
      name: 'dockit',
      port: 9120,
      baseUrl: 'http://127.0.0.1:9120',
    });
  });

  it('discovers sqlkit via SQLKIT_MCP_PORT env var', async () => {
    process.env.SQLKIT_MCP_PORT = '9121';
    const { discoverBackends } = await getDiscovery();
    const backends = discoverBackends();
    expect(backends).toHaveLength(1);
    expect(backends[0]).toMatchObject({
      name: 'sqlkit',
      port: 9121,
      baseUrl: 'http://127.0.0.1:9121',
    });
  });

  it('discovers both backends when both env vars set', async () => {
    process.env.DOCKIT_MCP_PORT = '9120';
    process.env.SQLKIT_MCP_PORT = '9121';
    const { discoverBackends } = await getDiscovery();
    const backends = discoverBackends();
    expect(backends).toHaveLength(2);
    expect(backends.map((b: { name: string }) => b.name).sort()).toEqual([
      'dockit',
      'sqlkit',
    ]);
  });

  it('ignores invalid env var values', async () => {
    process.env.DOCKIT_MCP_PORT = 'abc';
    const { discoverBackends } = await getDiscovery();
    const backends = discoverBackends();
    expect(backends).toHaveLength(0);
  });

  it('discovers backend via port file in app data dir', async () => {
    // Mock by setting up a temp dir and faking the discovery path
    // We use platform-appropriate path to verify
    const home = os.homedir();
    const platform = os.platform();
    const bundleDir =
      platform === 'darwin'
        ? path.join(home, 'Library', 'Application Support', 'club.geekfun.dockit')
        : path.join(home, '.local', 'share', 'club.geekfun.dockit');

    const portFile = path.join(bundleDir, 'mcp-port');
    fs.mkdirSync(bundleDir, { recursive: true });
    fs.writeFileSync(portFile, '9988');

    const { discoverBackends } = await getDiscovery();
    const backends = discoverBackends();
    expect(backends).toHaveLength(1);
    expect(backends[0]).toMatchObject({
      name: 'dockit',
      port: 9988,
    });

    // Cleanup
    fs.rmSync(bundleDir, { recursive: true, force: true });
  });

  it('env var takes precedence over port file', async () => {
    // Write a port file
    const home = os.homedir();
    const bundleDir =
      os.platform() === 'darwin'
        ? path.join(home, 'Library', 'Application Support', 'club.geekfun.dockit')
        : path.join(home, '.local', 'share', 'club.geekfun.dockit');
    const portFile = path.join(bundleDir, 'mcp-port');
    fs.mkdirSync(bundleDir, { recursive: true });
    fs.writeFileSync(portFile, '9988');

    // But env var says something else
    process.env.DOCKIT_MCP_PORT = '9120';

    const { discoverBackends } = await getDiscovery();
    const backends = discoverBackends();
    expect(backends).toHaveLength(1);
    expect(backends[0].port).toBe(9120);

    fs.rmSync(bundleDir, { recursive: true, force: true });
  });

  it('does not throw when port file is invalid', async () => {
    const home = os.homedir();
    const bundleDir =
      os.platform() === 'darwin'
        ? path.join(home, 'Library', 'Application Support', 'club.geekfun.dockit')
        : path.join(home, '.local', 'share', 'club.geekfun.dockit');
    const portFile = path.join(bundleDir, 'mcp-port');
    fs.mkdirSync(bundleDir, { recursive: true });
    fs.writeFileSync(portFile, 'not-a-number');

    const { discoverBackends } = await getDiscovery();
    const backends = discoverBackends();
    expect(backends).toHaveLength(0);

    fs.rmSync(bundleDir, { recursive: true, force: true });
  });
});
