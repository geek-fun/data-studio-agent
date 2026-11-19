import { randomUUID } from 'node:crypto';
import { readFileSync } from 'node:fs';

/**
 * Read the package version from the nearest package.json so the reported
 * SERVER_VERSION can never drift from the published version again. The build
 * copies package.json into dist/, keeping the relative path identical between
 * src (dev/tsx) and dist (production) layouts.
 *
 * Lives here — not in status.ts — because identity headers ride every bridge
 * request (backends.ts) and importing status.ts from backends.ts would close
 * an import cycle through tools.ts.
 */
const loadServerVersion = (): string => {
  try {
    const pkg = JSON.parse(readFileSync(new URL('../package.json', import.meta.url), 'utf8')) as {
      version?: string;
    };
    return pkg.version ?? '0.0.0';
  } catch {
    return '0.0.0';
  }
};

export const SERVER_VERSION = loadServerVersion();

/**
 * Identity attached to every bridge request so the dockit/sqlkit tray can
 * show which Data Studio MCP sessions are live. The session id is generated
 * once per process; the AI client name arrives with the MCP initialize
 * handshake and stays empty until then (older releases degrade gracefully —
 * the tray falls back to "last seen" phrasing).
 */
export const SESSION_ID = randomUUID();

export const identityState = { clientName: '' };

export const identityHeaders = (): Record<string, string> => {
  const product = `data-studio-mcp/${SERVER_VERSION}`;
  return {
    'X-Data-Studio-Session': SESSION_ID,
    'X-Data-Studio-Client': identityState.clientName
      ? `${product} (${identityState.clientName})`
      : product,
  };
};
