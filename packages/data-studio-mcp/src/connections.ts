import type { BackendClient } from './backends.js';

export type MergedConnection = {
  backend: string;
  id: unknown;
  name: string;
  type: string;
};

export const listConnections = async (
  clients: Map<string, BackendClient>,
): Promise<MergedConnection[]> => {
  const results = await Promise.all(
    [...clients.entries()].map(async ([backend, client]) => {
      try {
        const { connections } = await client.listTools();
        return connections.map(conn => ({ backend, ...conn }));
      } catch {
        return [];
      }
    }),
  );
  return results.flat();
};
