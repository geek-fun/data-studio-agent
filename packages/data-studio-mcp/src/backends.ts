import type { BackendInfo } from './discovery.js';

export interface BridgeToolDef {
  name: string;
  description: string;
  inputSchema: object;
  metadata?: {
    riskLevel?: string;
    requiredPermission?: string;
  };
}

export interface InvokeResult {
  status: number;
  data?: unknown;
  message?: string;
}

export interface BackendClient {
  readonly name: string;
  readonly baseUrl: string;
  listTools(): Promise<BridgeToolDef[]>;
  invokeTool(name: string, args: Record<string, unknown>): Promise<InvokeResult>;
}

export function createBackendClient(info: BackendInfo): BackendClient {
  return {
    get name() {
      return info.name;
    },

    get baseUrl() {
      return info.baseUrl;
    },

    async listTools(): Promise<BridgeToolDef[]> {
      const res = await fetch(`${info.baseUrl}/tools`, {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        signal: AbortSignal.timeout(5_000),
      });

      if (!res.ok) {
        throw new Error(`Backend ${info.name} returned HTTP ${res.status}: ${res.statusText}`);
      }

      const body = (await res.json()) as { tools: BridgeToolDef[]; connections: unknown[] };
      return body.tools ?? [];
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
}
