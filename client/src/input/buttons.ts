/**
 * `InputCmd.buttons` bit positions and `aux` bits. Mirror of `crates/protocol/src/msg/input.rs`
 * `pub mod buttons` and `crates/sim/src/movement/mod.rs` `AUX_WALK`; `input.test.ts` parses the Rust
 * source and fails when the numbers drift (the wasm layout table does not export them, so the Rust
 * file is the single source and this table is checked against it, never edited independently).
 */
export const BUTTONS = {
  FIRE: 1 << 0,
  AIM: 1 << 1,
  RELOAD: 1 << 2,
  JUMP: 1 << 3,
  CROUCH: 1 << 4,
  PRONE: 1 << 5,
  SPRINT: 1 << 6,
  USE: 1 << 7,
  LEAN_L: 1 << 8,
  LEAN_R: 1 << 9,
  MELEE: 1 << 10,
  GADGET: 1 << 11,
  SWAP: 1 << 12,
  VEHICLE_SEAT: 1 << 13,
} as const;

/** `InputCmd.aux` bit 0: walk modifier (GAMEDESIGN 9.1, sim `AUX_WALK`). */
export const AUX_WALK = 1 << 0;

/** Largest per-axis magnitude of `move_x` / `move_z` (-128 is invalid on the wire). */
export const MOVE_MAX = 127;
