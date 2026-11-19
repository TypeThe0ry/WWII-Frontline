/** C0 placement messages (crates/protocol/src/msg/c0.rs), MSB first like Rust BitWriter. */
const REQUEST = 0x17;
export const PLACEMENT_RESULT = 0x18;

export function placementPreview(id: number, slot: number, yaw: number, pitch: number): Uint8Array {
  const b = Uint8Array.of(REQUEST, 0, 0, slot, 0, 0, 0, 0);
  const v = new DataView(b.buffer);
  v.setUint16(1, id);
  v.setUint16(4, yaw);
  v.setInt16(6, pitch);
  return b;
}

export function placementCommit(id: number, previewId: number): Uint8Array {
  const b = Uint8Array.of(REQUEST, 0, 0, 3, 0, 0);
  const v = new DataView(b.buffer);
  v.setUint16(1, id);
  v.setUint16(4, previewId);
  return b;
}

export type PlacementTarget = readonly [number, number, number];
export function decodePlacementResult(b: Uint8Array): { id: number; reason: number; cost: number; target: PlacementTarget | null } | null {
  if (b[0] !== PLACEMENT_RESULT || !((b[5] === 0 && b.length === 6) || (b[5] === 1 && b.length === 18))) return null;
  const v = new DataView(b.buffer, b.byteOffset, b.byteLength);
  return {
    id: v.getUint16(1), reason: v.getUint8(3), cost: v.getUint8(4),
    target: b[5] === 1 ? [v.getInt32(6) / 100, v.getInt32(10) / 100, v.getInt32(14) / 100] : null,
  };
}
