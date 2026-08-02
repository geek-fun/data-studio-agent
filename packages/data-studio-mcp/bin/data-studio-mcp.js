#!/usr/bin/env node

/**
 * Launcher shim for @geek-fun/data-studio-mcp.
 *
 * In development (when src/index.ts exists without a matching src/index.js),
 * this loads tsx to transpile TypeScript on the fly. In production (after
 * `npm run build`), it directly imports the compiled JavaScript.
 */

import { existsSync } from 'node:fs';
import { fileURLToPath } from 'node:url';
import { dirname, join } from 'node:path';

const __dirname = dirname(fileURLToPath(import.meta.url));
const compiled = join(__dirname, '..', 'dist', 'src', 'index.js');

if (existsSync(compiled)) {
  // Production path: compiled JS
  const { main } = await import(compiled);
  await main();
} else {
  // Development path: use tsx for on-the-fly transpilation
  try {
    await import('tsx/esm');
  } catch {
    // tsx not installed — fall back to node with --experimental-strip-types
  }
  const { main } = await import(join(__dirname, '..', 'src', 'index.ts'));
  await main();
}
