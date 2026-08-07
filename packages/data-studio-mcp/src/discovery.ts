import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';

export type BackendName = 'dockit' | 'sqlkit';

export type BackendInfo = {
  name: BackendName;
  port: number;
  baseUrl: string;
};

export const KNOWN_BACKENDS: ReadonlyArray<{
  name: BackendName;
  bundleId: string;
  defaultPort: number;
  envVar: string;
}> = [
  { name: 'dockit', bundleId: 'club.geekfun.dockit', defaultPort: 9120, envVar: 'DOCKIT_MCP_PORT' },
  { name: 'sqlkit', bundleId: 'club.geekfun.sqlkit', defaultPort: 9121, envVar: 'SQLKIT_MCP_PORT' },
];

export const displayName = (name: BackendName): string => (name === 'dockit' ? 'DocKit' : 'SqlKit');

export const launchHint = (name: BackendName): string => {
  const app = displayName(name);
  if (process.platform === 'darwin') return `Launch ${app}: open -a ${app}`;
  if (process.platform === 'win32') return `Launch ${app} from the Start menu`;
  return `Launch ${app} from your applications menu`;
};

export const appDataDir = (bundleId: string): string => {
  const home = process.env.DATA_STUDIO_DATA_DIR ?? os.homedir();
  const platform = os.platform();

  if (platform === 'darwin') return path.join(home, 'Library', 'Application Support', bundleId);
  if (platform === 'linux') return path.join(home, '.local', 'share', bundleId);
  if (platform === 'win32') return path.join(home, 'AppData', 'Roaming', bundleId);
  return path.join(home, '.local', 'share', bundleId);
};

const discoverBackend = (cfg: (typeof KNOWN_BACKENDS)[number]): BackendInfo | null => {
  const envPort = process.env[cfg.envVar];
  if (envPort) {
    const port = parseInt(envPort, 10);
    if (!isNaN(port) && port > 0 && port < 65536) {
      return { name: cfg.name, port, baseUrl: `http://127.0.0.1:${port}` };
    }
  }

  const portFile = path.join(appDataDir(cfg.bundleId), 'mcp-port');
  try {
    const content = fs.readFileSync(portFile, 'utf-8').trim();
    const port = parseInt(content, 10);
    if (!isNaN(port) && port > 0 && port < 65536) {
      return { name: cfg.name, port, baseUrl: `http://127.0.0.1:${port}` };
    }
  } catch {
    // Port file not found
  }

  return null;
};

export const discoverBackends = (): BackendInfo[] => {
  return KNOWN_BACKENDS.map(discoverBackend).filter((b): b is BackendInfo => b !== null);
};
