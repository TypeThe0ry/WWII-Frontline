// Hand-written view of the wasm-pack package API (crates/sim-wasm/pkg/frontline_sim_wasm.d.ts), so
// the client type-checks without the generated package. Views returned by `step`/`state` and by
// SnapshotDecoder columns alias wasm memory: consume them before the next call into wasm.
export interface WasmPredictor {
  setEntity(entity: number): void;
  loadCollision(bx: number, bz: number, data: Uint8Array): void;
  unloadCollision(bx: number, bz: number): boolean;
  reset(authState: Uint8Array, tick: number, weapon?: Uint8Array): void;
  resetWeapon(weapon?: Uint8Array): void;
  equipWeapon(weaponId: string, reserve: number): void;
  /** Starts/resets prediction of the driven vehicle: `pos[3] vel[3] quat[4] (xyzw) yawRate` (11 floats). */
  resetVehicle(defId: string, kin: Float32Array): void;
  clearVehicle(): void;
  /** One vehicle tick: `pos vel quat yawRate forwardSpeed` (12 floats), or `undefined` without a vehicle. */
  stepVehicle(cmd: Uint8Array): Float32Array | undefined;
  step(cmd: Uint8Array): Float32Array;
  state(): Float32Array;
  weaponState(): Uint8Array;
  authState(): Uint8Array;
  tick(): number;
  stateHash(): bigint;
  free(): void;
}

export interface WasmSnapshotDecoder {
  decode(bytes: Uint8Array): number;
  reset(): void;
  tick(): number;
  ackInputSeq(): number;
  inputSlack(): number;
  count(): number;
  selfState(): Float32Array;
  ids(): Uint8Array;
  pos(): Float32Array;
  yaw(): Float32Array;
  pitch(): Float32Array;
  anim(): Uint8Array;
  stance(): Uint8Array;
  weapon(): Uint8Array;
  flags(): Uint8Array;
  health(): Uint8Array;
  removed(): Uint8Array;
  events(): Uint8Array;
  /** Vehicle entities of the last decode (wire ids 128..255): ids, `VEHICLE_STRIDE` floats and `VEHICLE_META_BYTES` bytes each. */
  vehicleIds(): Uint8Array;
  vehicleState(): Float32Array;
  vehicleMeta(): Uint8Array;
  free(): void;
}

export interface WasmServerC0 {
  kind: number;
  playerId: number;
  serverTick: number;
  tickHz: number;
  matchSeed: bigint;
  dataHash: bigint;
  mapId: number;
  mode: number;
  team: number;
  clientTickOffset: number;
  reason: number;
  ok: boolean;
  spawnTick: number;
  clientTime: number;
  serverTimeMs: number;
  free(): void;
}

export interface SimWasmModule {
  init(): void;
  protocolVersion(): number;
  dataHash(): bigint;
  layoutJson(): string;
  /** CRC-8 of the wire-quantised position (ARCHITECTURE 6.5.3, PI-023); the only implementation is Rust (frontline-protocol). */
  predHash(x: number, y: number, z: number): number;
  weaponDef(id: string): Uint8Array;
  roleDef(id: string): Uint8Array;
  weaponIndex(id: string): number;
  packInputCmd(seq: number, tick: number, moveX: number, moveZ: number, yaw: number, pitch: number, buttons: number, weaponSlot: number, aux: number, viewTick: number): Uint8Array;
  encodeInputCmd: SimWasmModule['packInputCmd'];
  packFireCmd(seq: number, viewTick: number, weapon: number, dirYaw: number, dirPitch: number, ox: number, oy: number, oz: number, spreadSeed: number): Uint8Array;
  encodeInputBundle(cmds: Uint8Array, ackTick: number, predHash: number, fire?: Uint8Array): Uint8Array;
  encodeHello(protocolVersion: number, dataHash: bigint, ticket: Uint8Array, clientBuild: number, caps: number): Uint8Array;
  encodePing(clientTime: number): Uint8Array;
  encodeDisconnect(reason: number): Uint8Array;
  encodeC0(kind: number, payload: Uint8Array): Uint8Array;
  decodeServerC0(bytes: Uint8Array): WasmServerC0;
  decodeC0(bytes: Uint8Array): WasmServerC0;
  /** One C4 payload as JSON in the shape of `C4Event` (roster entries have no `seq`; `NetCodec.decodeC4` numbers them). */
  decodeC4(bytes: Uint8Array): string;
  Predictor: new (matchSeed: bigint, mapId: number, myPlayerId: number) => WasmPredictor;
  SnapshotDecoder: new () => WasmSnapshotDecoder;
}
