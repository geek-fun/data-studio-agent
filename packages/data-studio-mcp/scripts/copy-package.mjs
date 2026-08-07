#!/usr/bin/env node

/**
 * Copies package.json into dist/ so dist/src/status.js can resolve
 * '../package.json' at runtime (mirroring the src/ layout used by tsx in dev).
 */

import { copyFileSync } from 'node:fs';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';

const root = dirname(dirname(fileURLToPath(import.meta.url)));
copyFileSync(join(root, 'package.json'), join(root, 'dist', 'package.json'));
