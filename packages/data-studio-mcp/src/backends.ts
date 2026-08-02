import type { BackendInfo } from './discovery.js';

export type BridgeToolDef = {
  name: string;
  description: string;
  inputSchema: object;
  metadata?: {
    riskLevel?: string;
    requiredPermission?: string;
  };
};

export type InvokeResult = {
  status: number;
  data?: unknown;
  message?: string;
};

export type BridgeConnection = {
  id: unknown;
  name: string;
  type: string;
};

export type BridgeCatalog = {
  tools: BridgeToolDef[];
  connections: BridgeConnection[];
};

export type BackendClient = {
  readonly name: string;
  readonly baseUrl: string;
  listTools(): Promise<BridgeCatalog>;
  invokeTool(name: string, args: Record<string, unknown>): Promise<InvokeResult>;
};

export const createBackendClient = (info: BackendInfo): BackendClient => {
  return {
    get name() {
      return info.name;
    },

    get baseUrl() {
      return info.baseUrl;
    },

    async listTools(): Promise<BridgeCatalog> {
      const res = await fetch(`${info.baseUrl}/tools`, {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        signal: AbortSignal.timeout(5_000),
      });

      if (!res.ok) {
        throw new Error(`Backend ${info.name} returned HTTP ${res.status}: ${res.statusText}`);
      }

      const body = (await res.json()) as {
        tools: BridgeToolDef[];
        connections?: BridgeConnection[];
      };

      return { tools: body.tools ?? [], connections: body.connections ?? [] };
    },

    async invokeTool(name: string, args: Record<string, unknown>): Promise<InvokeResult> {
      const res = await fetch(`${info.baseUrl}/invoke`, {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ name, args }),
        signal: AbortSignal.timeout(30_000),
      });

      return (await res.json()) as InvokeResult;
    },
  };
};
