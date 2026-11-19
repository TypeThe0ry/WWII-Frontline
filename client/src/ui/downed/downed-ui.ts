import { batch, computed, effect, signal, type ReadonlySignal } from '@preact/signals';
import { countdownS } from '../../game/deploy-validate';
import {
  bleedoutFraction, bleedoutRemainingS, callCooldownLeftS, canSpectate, DEFAULT_DOWNED_RULES,
  downedNext, giveUpProgress, giveUpState, killerView, nearestMedic, nextSpectateTarget,
  type DownedEvent, type DownedPhase, type DownedRules, type KillerInfo, type MedicInfo,
} from '../../game/downed-state';

interface DownedSources {
  /** Only true when combat actions can be sent safely. Unknown is not treated as connected. */
  connected: boolean;
  /** Authoritative downed HP. Do not pass the generic, quantized M0 health byte here. */
  downedHp: number;
  /** Caller filters to permitted friendly medics from TEAM / AOI. An empty list is different from no data. */
  medics: { px: number; pz: number; yawRad?: number; candidates: readonly MedicInfo[] };
  reviveProgress: number;
  hideNames: boolean;
  /** Caller supplies eligible teammates present in AOI, not an arbitrary entity list. */
  spectateCandidates: readonly number[];
  respawnClock: { spawnTick: number; nowTick: number; tickRate: number };
}
export type DownedInputs = { readonly [K in keyof DownedSources]?: ReadonlySignal<DownedSources[K] | undefined> };

/** Return true only if the adapter dispatched the request; this is not a server/camera acknowledgement. */
export interface DownedActions {
  callMedic?: () => boolean;
  giveUp?: () => boolean;
  redeploy?: () => boolean;
  spectate?: (targetId: number) => boolean;
}

export interface DownedOptions {
  inputs?: DownedInputs;
  actions?: DownedActions;
  /** Game rules for this session. Defaults come from the existing GD policy. */
  rules?: DownedRules;
  /** Monotonic clock; update() and transition() may also be given timestamps on this timebase. */
  now?: () => number;
}

export type DownedUi = ReturnType<typeof createDownedUi>;

/** The host dispatches authoritative lifecycle events. No snapshot/transport assumptions are made here. */
export function createDownedUi({ inputs = {}, actions = {}, rules = DEFAULT_DOWNED_RULES, now = () => performance.now() }: DownedOptions = {}) {
  const phase = signal<DownedPhase>('alive');
  const uiTime = signal(now());
  const downedAt = signal(0);
  const lastCallAt = signal<number | undefined>(undefined);
  const heldSince = signal<number | undefined>(undefined);
  const killer = signal<KillerInfo | undefined>(undefined);
  let cycleFrom: number | undefined;
  let disposed = false;

  const connection = computed(() => inputs.connected?.value);
  const connected = () => connection.peek() === true;
  const isDowned = () => !disposed && phase.peek() === 'downed';
  const afterDeath = () => !disposed && canSpectate(phase.peek());
  const secondsDown = (at: number) => Math.max(0, at - downedAt.value) / 1000;
  const canGiveUp = (at: number) => isDowned() && !!actions.giveUp && giveUpState(secondsDown(at), connected(), rules).enabled;
  const endGiveUp = () => { heldSince.value = undefined; };
  const offConnection = effect(() => { if (connection.value !== true) endGiveUp(); });

  function transition(event: DownedEvent, at = now()): boolean {
    if (disposed) return false;
    const current = phase.peek();
    const next = downedNext(current, event);
    if (next === current) return false;
    batch(() => {
      phase.value = next;
      uiTime.value = at;
      endGiveUp();
      if (next === 'downed') downedAt.value = at;
      if (!canSpectate(next)) {
        killer.value = undefined;
        cycleFrom = undefined;
      }
    });
    return true;
  }

  function update(at = now()): void {
    if (!isDowned()) return;
    const held = heldSince.peek();
    batch(() => {
      // 10 Hz for the countdown/hold display, but check the actual action deadline every update.
      if (at - uiTime.peek() >= 100) uiTime.value = at;
      if (held === undefined) return;
      if (!canGiveUp(at)) endGiveUp();
      else if (giveUpProgress((at - held) / 1000, rules) >= 1) {
        // Clear before dispatch: never repeat on held keys, or substitute a local death for the request.
        endGiveUp();
        actions.giveUp?.();
      }
    });
  }

  const callCooldown = (at: number, last: number | undefined) => last === undefined ? 0 : callCooldownLeftS(0, (at - last) / 1000, rules);
  return {
    phase: computed(() => phase.value),
    connection,
    bleedout: computed(() => {
      const hp = inputs.downedHp?.value;
      return phase.value === 'downed' && hp !== undefined && Number.isFinite(hp)
        ? { seconds: Math.ceil(bleedoutRemainingS(hp, rules)), fraction: bleedoutFraction(hp, rules) } : undefined;
    }),
    medic: computed(() => {
      const source = inputs.medics?.value;
      if (phase.value !== 'downed' || !source) return undefined;
      const nearest = nearestMedic(source.px, source.pz, source.candidates);
      return nearest === null ? null : {
        id: nearest.medic.id, distanceM: Math.round(nearest.distM), bearingRad: nearest.bearingRad,
        relativeBearingRad: source.yawRad === undefined ? undefined : nearest.bearingRad - source.yawRad,
        enRoute: nearest.medic.enRoute,
      };
    }),
    reviveProgress: computed(() => {
      const value = inputs.reviveProgress?.value;
      return phase.value === 'downed' && value !== undefined && Number.isFinite(value) ? Math.max(0, Math.min(1, value)) : undefined;
    }),
    callMedicState: computed(() => {
      const cooldownS = callCooldown(uiTime.value, lastCallAt.value);
      const available = !!actions.callMedic;
      return { available, cooldownS, enabled: available && phase.value === 'downed' && connection.value === true && cooldownS === 0 };
    }),
    giveUpState: computed(() => {
      const gate = giveUpState(secondsDown(uiTime.value), connection.value === true, rules);
      const available = !!actions.giveUp;
      return { ...gate, available, enabled: available && phase.value === 'downed' && gate.enabled };
    }),
    giveUpProgress: computed(() => {
      const held = heldSince.value;
      return held === undefined ? undefined : giveUpProgress((uiTime.value - held) / 1000, rules);
    }),
    killer: computed(() => {
      const hideNames = inputs.hideNames?.value === true;
      const view = killerView(phase.value, killer.value, hideNames);
      return view && hideNames ? { ...view, tagKeys: [] } : view;
    }),
    respawnSeconds: computed(() => {
      const clock = inputs.respawnClock?.value;
      return canSpectate(phase.value) && clock && clock.tickRate > 0 ? countdownS(clock.spawnTick, clock.nowTick, clock.tickRate) : undefined;
    }),
    canRedeploy: computed(() => !!actions.redeploy && canSpectate(phase.value) && connection.value === true),
    canCycleSpectate: computed(() => !!actions.spectate && canSpectate(phase.value) && connection.value === true && !!inputs.spectateCandidates?.value?.length),
    transition,
    update,
    /** Drop pre-death detail rather than saving it for later. Copy only the allowed fields. */
    setKiller(info: KillerInfo | undefined): boolean {
      if (!afterDeath()) return false;
      killer.value = info && { name: info.name, weapon: info.weapon, distM: info.distM, isBot: info.isBot, friendly: info.friendly, hidden: info.hidden };
      return true;
    },
    callMedic(): boolean {
      const at = now();
      if (!isDowned() || !connected() || callCooldown(at, lastCallAt.peek()) !== 0 || actions.callMedic?.() !== true) return false;
      batch(() => {
        lastCallAt.value = at;
        uiTime.value = at;
      });
      return true;
    },
    beginGiveUp(): boolean {
      const at = now();
      if (heldSince.peek() !== undefined || !canGiveUp(at)) return false;
      batch(() => {
        uiTime.value = at;
        heldSince.value = at;
      });
      return true;
    },
    endGiveUp,
    redeploy(): boolean {
      return afterDeath() && connected() && actions.redeploy?.() === true;
    },
    cycleSpectate(dir: 1 | -1): boolean {
      if (!afterDeath() || !connected()) return false;
      const target = nextSpectateTarget(cycleFrom, inputs.spectateCandidates?.peek() ?? [], dir);
      if (target === undefined || actions.spectate?.(target) !== true) return false;
      cycleFrom = target;
      return true;
    },
    dispose() {
      disposed = true;
      endGiveUp();
      offConnection();
    },
  };
}
