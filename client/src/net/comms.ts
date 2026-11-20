/** C0 comms tags and shared client types. */
export const C0_CHAT = 0x13;
export const C0_NOTICE = 0x1f;
export const CAP_TEAM_MARKERS = 1 << 2;

export const ChatChannel = { All: 0, Team: 1, Squad: 2, Command: 3 } as const;
export type ChatChannel = (typeof ChatChannel)[keyof typeof ChatChannel];

export interface ChatMessage {
  readonly channel: ChatChannel;
  readonly text: string;
  /** Authoritative player ID supplied by the server. */
  readonly sender: number;
}

export const PingKind = {
  Attention: 0, Enemy: 1, NeedMedic: 2, NeedAmmo: 3,
  NeedEngineer: 4, NeedAt: 5, NeedSupplies: 6, Thanks: 7,
} as const;
export type PingKind = (typeof PingKind)[keyof typeof PingKind];

export interface MarkerPosition {
  /** X/Z as 5 m cells relative to the map origin. */
  readonly pos: readonly [number, number];
  readonly ttl: number;
}

export interface LegacyTeamMarker extends MarkerPosition {
  /** Stable TeamMarkerKind tag. */
  readonly kind: number;
}

export interface TeamMarkersFrame {
  readonly tick: number;
  readonly markers: readonly LegacyTeamMarker[];
}
