// Client networking (M0-021). Depends only on sim-wasm (injected) and the event bus; never on render/ui/audio.
export { connect, WT_TIMEOUT_MS, TOTAL_BUDGET_MS } from './connect';
export type { ConnectOptions, NetStats, Session } from './connect';
export { ClockSync, ewma, TICK_MS, TICK_HZ } from './clock';
export { parseNetConfig, wsUrl, wtUrl } from './config';
export type { NetConfig, TransportChoice } from './config';
export { NetError } from './handshake';
export type { NetErrorCode } from './handshake';
export { NetCodec, Channel } from './codec';
export { TeamBalanceCode, TeamMoveFlags, VoteKind } from '../game/state/c4';
export type { C4Event } from '../game/state/c4';
export type { ServerMsg, WelcomeMsg } from './codec';
export type { Transport, TransportKind } from './transport';

export { ChatChannel, PingKind } from './comms';
export type { ChatMessage } from './comms';
