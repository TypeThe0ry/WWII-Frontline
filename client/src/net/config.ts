/**
 * Connection settings from the page URL (M0-021):
 *   ?server=host[:port]   WebSocket endpoint; port defaults to 7000. WebTransport uses `wtport`, default ws port + 1
 *                         (the server's 7000/7001 defaults). Host defaults to the page's hostname.
 *   ?wtport=<n>           explicit WebTransport (UDP) port.
 *   ?transport=wt|ws|auto auto (default): WebTransport first, fall back to WebSocket.
 *   ?server=same          same-origin WebSocket `<page origin>/game-ws` (for a dev-server / reverse-proxy that forwards only the
 *                         page port; see vite.config.ts). Also the default when no `server` is given and the page is not served
 *                         from localhost / 127.0.0.1. WebTransport (UDP) cannot go through such a proxy, so transport is forced to ws.
 *   ?certhash=<hash>      SHA-256 of the dev certificate (64 hex chars, or base64/base64url), M0-017.
 */
export type TransportChoice = 'wt' | 'ws' | 'auto';

export interface NetConfig {
  host: string;
  wsPort: number;
  wtPort: number;
  secure: boolean;
  /** Same-origin mode: `host:port` of the page (as in `location.host`); `wsUrl` then uses `/game-ws` on it. */
  sameOriginHost?: string;
  transport: TransportChoice;
  /** Raw SHA-256 bytes of the pinned certificate, when `certhash` was given. */
  certHash?: Uint8Array;
}

export const DEFAULT_WS_PORT = 7000;
export const SAME_ORIGIN_WS_PATH = '/game-ws';

function isLoopbackHost(h: string): boolean {
  return h === 'localhost' || h === '127.0.0.1' || h === '[::1]' || h === '::1';
}

export function parseCertHash(text: string): Uint8Array {
  const t = text.trim();
  if (/^[0-9a-fA-F]{64}$/.test(t)) {
    const out = new Uint8Array(32);
    for (let i = 0; i < 32; i++) out[i] = parseInt(t.slice(i * 2, i * 2 + 2), 16);
    return out;
  }
  const b64 = t.replace(/-/g, '+').replace(/_/g, '/');
  if (!/^[A-Za-z0-9+/]+=*$/.test(b64)) throw new Error(`certhash: not hex or base64: '${t.slice(0, 16)}...'`);
  const bin = atob(b64);
  if (bin.length !== 32) throw new Error(`certhash: expected 32 bytes, got ${bin.length}`);
  return Uint8Array.from(bin, (c) => c.charCodeAt(0));
}

function parsePort(v: string, what: string): number {
  const n = Number(v);
  if (!Number.isInteger(n) || n < 1 || n > 65535) throw new Error(`${what}: invalid port '${v}'`);
  return n;
}

/** `location` is only used for defaults (hostname, protocol); pass a stub in tests. */
export function parseNetConfig(
  search: string,
  location?: { hostname: string; protocol: string; host?: string },
): NetConfig {
  const q = new URLSearchParams(search);
  const serverParam = q.get('server')?.trim();
  const same = serverParam === 'same' || (!serverParam && !!location && !isLoopbackHost(location.hostname));
  if (same) {
    if (!location) throw new Error('server=same needs a page location');
    return {
      host: location.hostname,
      wsPort: DEFAULT_WS_PORT,
      wtPort: DEFAULT_WS_PORT + 1,
      secure: location.protocol === 'https:',
      sameOriginHost: location.host || location.hostname,
      transport: 'ws', // no UDP through a TCP-only proxy: never try WebTransport first
    };
  }
  let host = location?.hostname || 'localhost';
  let wsPort = DEFAULT_WS_PORT;
  const server = q.get('server');
  if (server) {
    const m = /^(\[[0-9a-fA-F:]+\]|[^:/\s]+)(?::(\d+))?$/.exec(server.trim());
    if (!m) throw new Error(`server: expected host[:port], got '${server}'`);
    host = m[1] as string;
    if (m[2]) wsPort = parsePort(m[2], 'server');
  }
  const wtp = q.get('wtport');
  const wtPort = wtp ? parsePort(wtp, 'wtport') : wsPort + 1;
  const t = q.get('transport') ?? 'auto';
  if (t !== 'wt' && t !== 'ws' && t !== 'auto') throw new Error(`transport: expected wt|ws|auto, got '${t}'`);
  const ch = q.get('certhash');
  return {
    host,
    wsPort,
    wtPort,
    secure: location?.protocol === 'https:',
    transport: t,
    ...(ch ? { certHash: parseCertHash(ch) } : {}),
  };
}

export function wsUrl(c: NetConfig): string {
  if (c.sameOriginHost) return `${c.secure ? 'wss' : 'ws'}://${c.sameOriginHost}${SAME_ORIGIN_WS_PATH}`;
  return `${c.secure ? 'wss' : 'ws'}://${c.host}:${c.wsPort}/`;
}

export function wtUrl(c: NetConfig): string {
  return `https://${c.host}:${c.wtPort}/wt`;
}
