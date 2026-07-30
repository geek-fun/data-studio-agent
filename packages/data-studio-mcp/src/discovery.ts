import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';

export interface BackendInfo {
  name: 'dockit' | 'sqlkit';
  port: number;
  baseUrl: string;
}

/**
 * Platform-specific app data directory for a Tauri app with the given bundle ID.
 * Replicates Tauri v2's `app_data_dir()` resolution.
 */
function appDataDir(bundleId: string): string {
  const home = os.homedir();
  const platform = os.platform();

  if (platform === 'darwin') {
    return path.join(home, 'Library', 'Application Support', bundleId);
  }
  if (platform === 'linux') {
    return path.join(home, '.local', 'share', bundleId);
  }
  if (platform === 'win32') {
    return path.join(home, 'AppData', 'Roaming', bundleId);
  }
  // Fallback: try XDG config home on Linux, or app data dir pattern
  return path.join(home, '.local', 'share', bundleId);
}

const BACKENDS: Array<{
  name: 'dockit' | 'sqlkit';
  bundleId: string;
  defaultPort: number;
  envVar: string;
}> = [
  {
    name: 'dockit',
    bundleId: 'club.geekfun.dockit',
    defaultPort: 9120,
    envVar: 'DOCKIT_MCP_PORT',
  },
  {
    name: 'sqlkit',
    bundleId: 'club.geekfun.sqlkit',
    defaultPort: 9121,
    envVar: 'SQLKIT_MCP_PORT',
  },
];

/**
 * Try to discover a backend by:
 * 1. Explicit env var DOCKIT_MCP_PORT / SQLKIT_MCP_PORT
 * 2. Port file at app_data_dir/mcp-port
 */
function discoverBackend(
  cfg: (typeof BACKENDS)[number],
): BackendInfo | null {
  // 1. Environment variable override
  const envPort = process.env[cfg.envVar];
  if (envPort) {
    const port = parseInt(envPort, 10);
    if (!isNaN(port) && port > 0 && port < 65536) {
      return {
        name: cfg.name,
        port,
        baseUrl: `http://127.0.0.1:${port}`,
      };
    }
  }

  // 2. Port file written by the desktop app
  const dataDir = appDataDir(cfg.bundleId);
  const portFile = path.join(dataDir, 'mcp-port');
  try {
    const content = fs.readFileSync(portFile, 'utf-8').trim();
    const port = parseInt(content, 10);
    if (!isNaN(port) && port > 0 && port < 65536) {
      return {
        name: cfg.name,
        port,
        baseUrl: `http://127.0.0.1:${port}`,
      };
    }
  } catch {
    // Port file not found — backend not running
  }

  return null;
}

/**
 * Discover all running backends.
 */
export function discoverBackends(): BackendInfo[] {
  const results: BackendInfo[] = [];

  for (const cfg of BACKENDS) {
    const backend = discoverBackend(cfg);
    if (backend) {
      results.push(backend);
    }
  }

  return results;
}
