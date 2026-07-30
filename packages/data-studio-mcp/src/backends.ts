import type { BackendInfo } from './discovery.js';

/**
 * Raw tool definition as returned by the bridge /tools endpoint.
 */
export interface BridgeToolDef {
  name: string;
  description: string;
  inputSchema: object;
  metadata?: {
    riskLevel?: string;
    requiredPermission?: string;
  };
}

/**
 * Response from the bridge /invoke endpoint.
 */
export interface InvokeResult {
  status: number;
  data?: unknown;
  message?: string;
}

/**
 * HTTP client for a single backend bridge.
 */
export class BackendClient {
  constructor(private readonly info: BackendInfo) {}

  get name(): string {
    return this.info.name;
  }

  get baseUrl(): string {
    return this.info.baseUrl;
  }

  /**
   * Fetch all tools from the bridge.
   */
  async listTools(): Promise<BridgeToolDef[]> {
    const res = await fetch(`${this.info.baseUrl}/tools`, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      signal: AbortSignal.timeout(5_000),
    });

    if (!res.ok) {
      throw new Error(
        `Backend ${this.info.name} returned HTTP ${res.status}: ${res.statusText}`,
      );
    }

    const body = (await res.json()) as {
      tools: BridgeToolDef[];
      connections: unknown[];
    };

    return body.tools ?? [];
  }

  /**
   * Invoke a tool by its internal capability name.
   */
  async invokeTool(
    name: string,
    args: Record<string, unknown>,
  ): Promise<InvokeResult> {
    const res = await fetch(`${this.info.baseUrl}/invoke`, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ name, args }),
      signal: AbortSignal.timeout(30_000),
    });

    const body = (await res.json()) as InvokeResult;
    return body;
  }
}
