import { createBackendClient, probeBackend, type BackendClient } from './backends.js';
import {
  discoverBackends,
  KNOWN_BACKENDS,
  launchHint,
  displayName,
  type BackendInfo,
  type BackendName,
} from './discovery.js';
import { buildToolCatalog, type McpToolDef, type Route } from './tools.js';
import { SERVER_VERSION, type BackendStatus, type RegistrySnapshot } from './status.js';

export type BackendRegistryOptions = {
  readonly: boolean;
  pollIntervalMs?: number;
  discover?: () => readonly BackendInfo[];
  probe?: (info: BackendInfo) => Promise<boolean>;
};

export class BackendRegistry {
  private readonly readOnly: boolean;
  private readonly pollIntervalMs: number;
  private readonly discover: () => readonly BackendInfo[];
  private readonly probe: (info: BackendInfo) => Promise<boolean>;
  private readonly startedAt = Date.now();
  private timer: ReturnType<typeof setInterval> | null = null;
  private tools: readonly McpToolDef[] = [];
  private routeMap = new Map<string, Route>();
  private clients = new Map<string, BackendClient>();
  private statuses: readonly BackendStatus[] = [];
  private signature = '';
  private refreshing = false;

  onToolsChanged: ((snapshot: RegistrySnapshot) => void) | null = null;

  constructor(options: BackendRegistryOptions) {
    this.readOnly = options.readonly;
    this.pollIntervalMs = options.pollIntervalMs ?? 10_000;
    this.discover = options.discover ?? discoverBackends;
    this.probe = options.probe ?? probeBackend;
  }

  async start(): Promise<RegistrySnapshot> {
    await this.refresh();
    this.timer = setInterval(() => {
      void this.refresh();
    }, this.pollIntervalMs);
    this.timer.unref();
    return this.snapshot();
  }

  stop(): void {
    if (this.timer) {
      clearInterval(this.timer);
      this.timer = null;
    }
  }

  snapshot(): RegistrySnapshot {
    return {
      tools: this.tools,
      routeMap: this.routeMap,
      clients: this.clients,
      statuses: this.statuses,
      version: SERVER_VERSION,
      uptimeSeconds: Math.floor((Date.now() - this.startedAt) / 1000),
      readOnly: this.readOnly,
    };
  }

  async refresh(): Promise<boolean> {
    if (this.refreshing) return false;
    this.refreshing = true;
    try {
      return await this.poll();
    } finally {
      this.refreshing = false;
    }
  }

  private async poll(): Promise<boolean> {
    const candidates = this.discover();
    const reachable = await this.probeAll(candidates);
    const connected = candidates.filter(c => reachable.has(c.name));
    const { tools, routeMap } = await buildToolCatalog(connected);

    this.clients = new Map(connected.map(b => [b.name, createBackendClient(b)]));
    this.statuses = this.buildStatuses(candidates, reachable, tools);

    const signature = tools
      .map(t => t.name)
      .sort()
      .join(',');
    const changed = signature !== this.signature;
    if (changed) {
      this.tools = tools;
      this.routeMap = routeMap;
      this.signature = signature;
      this.onToolsChanged?.(this.snapshot());
    }
    return changed;
  }

  private async probeAll(candidates: readonly BackendInfo[]): Promise<Set<BackendName>> {
    const results = await Promise.all(
      candidates.map(async c => ({ name: c.name, ok: await this.probe(c) })),
    );
    return new Set(results.filter(r => r.ok).map(r => r.name));
  }

  private buildStatuses(
    candidates: readonly BackendInfo[],
    reachable: ReadonlySet<BackendName>,
    tools: readonly McpToolDef[],
  ): BackendStatus[] {
    return KNOWN_BACKENDS.map(cfg => {
      const candidate = candidates.find(c => c.name === cfg.name);
      const connected = reachable.has(cfg.name);

      if (!candidate) {
        return {
          name: cfg.name,
          port: null,
          baseUrl: null,
          status: 'unavailable',
          toolCount: 0,
          hint: `${displayName(cfg.name)} is not running or its port file is missing. ${launchHint(cfg.name)} and enable MCP Bridge auto-start, or set ${cfg.envVar}.`,
        };
      }
      if (!connected) {
        return {
          name: cfg.name,
          port: candidate.port,
          baseUrl: candidate.baseUrl,
          status: 'unavailable',
          toolCount: 0,
          hint: `Port file points to ${candidate.port} but no bridge is responding. ${launchHint(cfg.name)}. Once running, its tools appear automatically.`,
        };
      }
      return {
        name: cfg.name,
        port: candidate.port,
        baseUrl: candidate.baseUrl,
        status: 'connected',
        toolCount: tools.filter(t => t.backendName === cfg.name).length,
        hint: `Connected on port ${candidate.port}.`,
      };
    });
  }
}
