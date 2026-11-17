// Browser harness for client/e2e/net.spec.ts: connects with the real net module and the real sim-wasm
// package against a real frontline-server, and exposes the state on window.__net.
// Query: the usual net parameters (server, wtport, transport, certhash) plus
//   wasm=<url of frontline_sim_wasm.js>  badproto=1  baddata=1
import { EventBus, type GameEvents } from '../../src/game/event-bus';
import { connect, NetError, parseNetConfig, type Session } from '../../src/net';
import type { SimWasmModule } from '../../src/net/wasm';

interface WasmPkg extends SimWasmModule {
  default(): Promise<unknown>;
}

interface NetState {
  status: 'idle' | 'connecting' | 'connected' | 'failed' | 'disconnected';
  events: string[];
  log: string[];
  error: { code: string; reason: string | undefined; message: string } | undefined;
  session: Session | undefined;
  snapshots: number;
  info: Record<string, unknown> | undefined;
  connect(): Promise<void>;
  sendInput(): void;
  close(): void;
}

declare global {
  interface Window {
    __net: NetState;
  }
}

const q = new URLSearchParams(location.search);
const statusEl = document.getElementById('status') as HTMLElement;
const bus = new EventBus<GameEvents>();
let wasm: WasmPkg | undefined;
let seq = 0;

function render(): void {
  const s = window.__net;
  const lines = [`status=${s.status}`];
  if (s.info) for (const [k, v] of Object.entries(s.info)) lines.push(`${k}=${String(v)}`);
  if (s.error) lines.push(`error=${s.error.code}${s.error.reason ? `:${s.error.reason}` : ''} ${s.error.message}`);
  statusEl.textContent = lines.join('\n');
}

const state: NetState = {
  status: 'idle',
  events: [],
  log: [],
  error: undefined,
  session: undefined,
  snapshots: 0,
  info: undefined,
  async connect() {
    state.status = 'connecting';
    state.error = undefined;
    state.info = undefined;
    render();
    try {
      wasm ??= (await import(/* @vite-ignore */ q.get('wasm') ?? '')) as WasmPkg;
      if (!state.log.includes('wasm')) {
        await wasm.default();
        wasm.init();
        state.log.push('wasm');
      }
      const config = parseNetConfig(location.search, location);
      const hello = {
        ...(q.get('badproto') ? { protocolVersion: wasm.protocolVersion() + 1 } : {}),
        ...(q.get('baddata') ? { dataHash: wasm.dataHash() ^ 1n } : {}),
      };
      const s = await connect({ config, wasm, bus, hello, log: (m) => state.log.push(m) });
      state.session = s;
      state.status = 'connected';
      s.onSnapshot(() => {
        state.snapshots++;
      });
      s.onDisconnect((reason) => {
        state.status = 'disconnected';
        state.info = { ...state.info, disconnect: reason };
        render();
      });
      state.info = {
        transport: s.kind,
        tick_hz: s.welcome.tickHz,
        match_seed: s.welcome.matchSeed.toString(),
        player_id: s.welcome.playerId,
        connect_ms: s.connectMs.toFixed(1),
        handshake_rtt_ms: s.handshakeRttMs.toFixed(2),
        fallback_reason: s.fallbackReason ?? 'none',
      };
    } catch (e) {
      state.status = 'failed';
      state.error =
        e instanceof NetError ? { code: e.code, reason: e.reason, message: e.message } : { code: 'error', reason: undefined, message: e instanceof Error ? e.message : String(e) };
    }
    render();
  },
  sendInput() {
    if (!wasm || !state.session) return;
    const cmd = wasm.packInputCmd(seq++ & 0xffff, state.session.clock.estimatedServerTick(), 0, 0, 0, 0, 0, 0, 0, 0);
    state.session.sendInputBundle(wasm.encodeInputBundle(cmd, 0, 0));
  },
  close() {
    state.session?.close();
  },
};
window.__net = state;
bus.on('net.connected', (e) => state.events.push(`connected:${e.transport}:${e.rtt.toFixed(1)}`));
bus.on('net.disconnected', (e) => state.events.push(`disconnected:${e.reason}`));
if (!q.has('manual')) void state.connect();
