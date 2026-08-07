import type { BackendInfo } from './discovery.js';

export type BridgeToolDef = {
  name: string;
  description: string;
  inputSchema: object;
  metadata?: {
    riskLevel?: 'safe' | 'elevated' | 'destructive';
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
      // Bridge InvokeRequest requires connection_id at top level, not inside args.
      // Normalize to string: dockit connection ids are numbers, bridge expects string.
      const { connection_id, ...restArgs } = args;
      const body =
        connection_id === undefined
          ? { name, args: restArgs }
          : { name, args: restArgs, connection_id: String(connection_id) };

      const res = await fetch(`${info.baseUrl}/invoke`, {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify(body),
        signal: AbortSignal.timeout(30_000),
      });

      return (await res.json()) as InvokeResult;
    },
  };
};

/** Any HTTP response (even 404) proves the port is listening — guards against stale port files. */
export const probeBackend = async (info: BackendInfo): Promise<boolean> => {
  try {
    await fetch(info.baseUrl, { signal: AbortSignal.timeout(1_500) });
    return true;
  } catch {
    return false;
  }
};
