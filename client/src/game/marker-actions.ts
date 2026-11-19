import type { MarkerKind } from './markers';

export interface MarkerRequest {
  kind: MarkerKind;
  /** World metres, in map x/z coordinates. */
  pos: readonly [number, number];
}

export interface MarkerActions {
  readonly supported: false;
  readonly reason: string;
  place(request: MarkerRequest): { ok: false; reason: 'unsupported' };
}

/**
 * C0 defines MarkerPlace, but crates/server/src/c0.rs drops it without applying or relaying it.
 * Don't send an ignored command or populate a local marker book as if the server had accepted it.
 */
export const markerActions: MarkerActions = {
  supported: false,
  reason: 'This server does not support marker placement.',
  place() {
    return { ok: false, reason: 'unsupported' };
  },
};
