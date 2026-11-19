import { signal } from '@preact/signals';
import type { ReadonlySignal } from '@preact/signals';
import { BUTTONS } from '../input/buttons';
import type { InputCmdFields, InputSampler, InputSource } from '../input/sampler';
import { decodePlacementResult, placementCommit, placementPreview } from '../net/placement';
import { PLACE_ERROR_KEY as ERR, PLACEMENT_IDLE, placementNext, type PlacementKind } from './placement';
import type { PlacementView } from './placement-actions';

const SLOT = { garrison: 0, supply_crate: 1, ammo_crate: 2 } as const;
// Temporary bindings for the logic slice; the future tool panel can call select() directly.
const KEYS: Readonly<Record<string, PlacementKind>> = { Digit6: 'garrison', Digit7: 'supply_crate', Digit8: 'ammo_crate' };
const REASON = [
  '', ERR.NotAlive, ERR.WrongRole, 'ui.officer.tool.needSquad', 'ui.officer.tool.needSupport',
  'ui.officer.err.notCarrying', 'ui.officer.err.noTarget', ERR.InNoBuildZone, ERR.TooCloseHq,
  ERR.TooCloseOwnGarrison, ERR.TooCloseObjective, ERR.Slope, ERR.Water, ERR.NoSupplies, ERR.TeamCap,
  ERR.SectorCap, 'ui.officer.err.ammoLimit', 'ui.officer.err.ammoCooldown', 'ui.officer.err.ammoStacked',
  'ui.officer.err.placementRejected',
];
const MOUSE = BUTTONS.FIRE | BUTTONS.AIM;
const IDLE: PlacementView = { state: PLACEMENT_IDLE, target: null, preview: null };
type Aim = Pick<InputCmdFields, 'yaw' | 'pitch'>;
type Request = { id: number; at: number; aim: Aim };

export class LivePlacement implements InputSource {
  private readonly current = signal<PlacementView>(IDLE);
  readonly view: ReadonlySignal<PlacementView> = this.current;
  private nextId = 0;
  private pending?: Request;
  private token?: Request;
  private click = false;
  private suppressed = false;
  private nextPreviewAt = 0;

  constructor(private readonly sampler: InputSampler, private readonly send: (bytes: Uint8Array) => void, private readonly now = () => performance.now()) {
    sampler.onInputContext((code) => {
      if (Object.hasOwn(KEYS, code)) this.select(KEYS[code]!);
      if (this.current.peek().state.mode === 'idle') return;
      if (code === 'Mouse0' && this.token && !this.pending) this.click = true;
      if (code === 'Mouse2' && this.current.peek().state.mode !== 'committing') this.clear();
    }, () => this.clear());
  }

  select(kind: PlacementKind): void {
    if (this.current.peek().state.mode === 'committing') return;
    this.clear();
    this.suppressed = true;
    this.current.value = { ...IDLE, state: placementNext(PLACEMENT_IDLE, { t: 'enter', kind }) };
  }

  private clear(): void {
    this.pending = this.token = undefined;
    this.click = false;
    this.nextPreviewAt = 0;
    this.current.value = IDLE;
  }

  releaseAll(): void {
    this.sampler.releaseAll();
  }

  receive(bytes: Uint8Array): void {
    const response = decodePlacementResult(bytes);
    const pending = this.pending;
    if (!response || !pending || response.id !== pending.id) return;
    this.pending = undefined;
    const current = this.current.peek();
    const reasonKey = REASON[response.reason] || 'ui.officer.err.placementRejected';
    if (current.state.mode === 'committing') {
      if (!response.reason) this.clear();
      else this.reject(reasonKey);
      return;
    }
    const preview = !response.reason && response.target
      ? { legal: true, supplyCost: response.cost }
      : { legal: false, reasonKey };
    this.token = preview.legal ? pending : undefined;
    this.current.value = { state: placementNext(current.state, { t: 'check', result: preview }), preview, target: response.target };
  }

  private reject(reasonKey: string): void {
    const c = this.current.peek();
    this.current.value = { ...c, state: placementNext(c.state, { t: 'rejected', reasonKey }), preview: { legal: false, reasonKey } };
    this.nextPreviewAt = this.now() + 500;
  }

  build(seq: number, tick: number, viewTick: number): InputCmdFields {
    const cmd = this.sampler.build(seq, tick, viewTick);
    const state = this.current.peek().state;
    if (state.kind || this.suppressed) {
      const held = cmd.buttons & MOUSE;
      cmd.buttons &= ~MOUSE;
      if (!state.kind && !held) this.suppressed = false;
    }
    const click = this.click;
    this.click = false;
    if (!state.kind) return cmd;
    const now = this.now();
    if (this.pending && now - this.pending.at >= 1000) {
      this.pending = undefined;
      if (state.mode === 'committing') {
        this.reject('ui.officer.err.previewUnavailable');
        return cmd;
      }
    }
    if (state.mode === 'committing') return cmd;

    const token = this.token;
    const yawDelta = token ? ((cmd.yaw - token.aim.yaw + 32768) & 0xffff) - 32768 : 0;
    const moved = state.kind === 'garrison' && token && (Math.abs(yawDelta) > 256 || Math.abs(cmd.pitch - token.aim.pitch) > 256);
    if (token && (moved || now - token.at >= 750)) {
      const preview = { legal: false, reasonKey: 'ui.officer.err.previewUnavailable' };
      this.current.value = { ...IDLE, state: placementNext(state, { t: 'check', result: preview }), preview };
      this.token = undefined;
    }
    // Only use the latest confirmed token; an already-requested preview supersedes it on the server.
    if (this.pending) return cmd;
    const aim = { yaw: cmd.yaw, pitch: cmd.pitch };
    if (click && this.token) {
      const id = this.nextId++ & 0xffff;
      const previewId = this.token.id;
      this.token = undefined;
      this.pending = { id, at: now, aim };
      this.current.value = { ...this.current.peek(), state: placementNext(state, { t: 'commit' }) };
      this.send(placementCommit(id, previewId));
    } else if (now >= this.nextPreviewAt) {
      const id = this.nextId++ & 0xffff;
      this.pending = { id, at: now, aim };
      this.nextPreviewAt = now + 250;
      this.send(placementPreview(id, SLOT[state.kind], aim.yaw, aim.pitch));
    }
    return cmd;
  }
}
