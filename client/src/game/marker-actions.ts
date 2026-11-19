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
 * This UI does not yet negotiate or consume the C3 marker extension.
 * Do not populate a local marker book as if an authoritative server had accepted it.
 */
export const markerActions: MarkerActions = {
  supported: false,
  reason: 'This client does not yet support networked marker placement.',
  place() {
    return { ok: false, reason: 'unsupported' };
  },
};
