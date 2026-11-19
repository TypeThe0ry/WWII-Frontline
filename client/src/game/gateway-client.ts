import type { ServerInfo } from './server-list';

const TIMEOUT_MS = 5_000;
type JsonObject = Record<string, unknown>;
export interface GuestSession { token: string; nick: string }
export interface JoinTicket { ticket: string; host: string; wsPort: number; wtPort: number }

export class GatewayError extends Error {
  constructor(readonly code: string, readonly status?: number) {
    super(code);
    this.name = 'GatewayError';
  }
}

function object(value: unknown): JsonObject {
  if (!value || typeof value !== 'object' || Array.isArray(value)) throw new GatewayError('invalid_response');
  return value as JsonObject;
}
function text(data: JsonObject, key: string): string {
  const value = data[key];
  if (typeof value !== 'string' || !value) throw new GatewayError('invalid_response');
  return value;
}
function integer(data: JsonObject, key: string, min: number, max: number): number {
  const value = data[key];
  if (typeof value !== 'number' || !Number.isInteger(value) || value < min || value > max) throw new GatewayError('invalid_response');
  return value;
}

/** M1-029 wire contract: /v1/servers, /v1/guest and /v1/servers/{id}/ticket.
 * There is no /join alias, team selector, ping or password field in the current gateway.
 */
export class GatewayClient {
  constructor(private readonly base: string, private readonly fetcher: typeof fetch = globalThis.fetch.bind(globalThis)) {}

  private async json(path: string, init: RequestInit, signal?: AbortSignal): Promise<unknown> {
    const timeout = AbortSignal.timeout(TIMEOUT_MS);
    const requestSignal = signal ? AbortSignal.any([signal, timeout]) : timeout;
    try {
      const res = await this.fetcher(`${this.base.replace(/\/+$/, '')}${path}`, { ...init, signal: requestSignal, cache: 'no-store', redirect: 'error' });
      if (!res.ok) {
        const data = await res.json().catch(() => null) as { error?: unknown } | null;
        requestSignal.throwIfAborted();
        throw new GatewayError(typeof data?.error === 'string' ? data.error : 'http_error', res.status);
      }
      try { return await res.json() as unknown; } catch {
        requestSignal.throwIfAborted();
        throw new GatewayError('invalid_response');
      }
    } catch (err) {
      if (signal?.aborted) throw signal.reason;
      if (timeout.aborted) throw new GatewayError('timeout');
      if (err instanceof GatewayError) throw err;
      throw new GatewayError('network');
    }
  }

  async list(signal?: AbortSignal): Promise<ServerInfo[]> {
    const data = await this.json('/v1/servers', { method: 'GET' }, signal);
    if (!Array.isArray(data)) throw new GatewayError('invalid_response');
    return data.map((row: unknown) => {
      const r = object(row);
      if (typeof r.compatible !== 'boolean') throw new GatewayError('invalid_response');
      return { id: text(r, 'match_id'), name: text(r, 'name'), map: text(r, 'map'), mode: text(r, 'mode'), phase: text(r, 'phase'),
        region: text(r, 'region'), version: text(r, 'version'), compatible: r.compatible,
        humans: integer(r, 'humans', 0, 255), bots: integer(r, 'bots', 0, 255), cap: integer(r, 'cap', 1, 255) };
    });
  }

  async guest(signal?: AbortSignal): Promise<GuestSession> {
    const r = object(await this.json('/v1/guest', { method: 'POST', headers: { 'Content-Type': 'application/json' }, body: '{}' }, signal));
    return { token: text(r, 'token'), nick: text(r, 'nick_suggestion') };
  }

  async join(id: string, guest: GuestSession, nickname: string, signal?: AbortSignal): Promise<JoinTicket> {
    if (!/^[a-f\d]{32}$/i.test(id)) throw new GatewayError('invalid_match');
    const r = object(await this.json(`/v1/servers/${id}/ticket`, {
      method: 'POST', headers: { Authorization: `Bearer ${guest.token}`, 'Content-Type': 'application/json' }, body: JSON.stringify({ nick: nickname.trim() || guest.nick }),
    }, signal));
    const addr = text(r, 'addr');
    // Gateway advertises just a host, not a URL. Reject path/authority syntax before handing the ticket to a transport.
    if (!/^(?:[a-z\d.-]+|\[[a-f\d:]+\]|[a-f\d]*:[a-f\d:]+)$/i.test(addr)) throw new GatewayError('invalid_response');
    const host = addr.includes(':') && !addr.startsWith('[') ? `[${addr}]` : addr;
    const ticket = text(r, 'ticket');
    if (ticket.length > 256 || !/^[\w-]+\.[\w-]+$/.test(ticket)) throw new GatewayError('invalid_response');
    return { ticket, host, wsPort: integer(r, 'ws_port', 1, 65535), wtPort: integer(r, 'wt_port', 0, 65535) };
  }
}
