import { batch, computed, effect, signal, type ReadonlySignal } from '@preact/signals';
import { countdownS } from '../../game/deploy-validate';
import type { DownedPhase } from '../../game/downed-state';
import type { EventBus, GameEvents } from '../../game/event-bus';
import type { GameState } from '../../game/state';
import type { Roster } from '../../game/state/roster';
import {
  ammoTier, captureState, compassOffsetPx, fxState, fxVignette, headingDeg, healthBand,
  KillFeedQueue, KILLFEED_MS, magEmpty, ringSegments, showExactAmmo, showExactDistance,
  staminaWarn, wedgeOpacity, wedgeStrength, WEDGE_MS, type FxState,
} from './hud-math';

export const HIT_CONFIRM_MS = 80;
const SNAP_INTERVAL_MS = 50;
const SUPPRESS_EVENT_MS = 3000;
const FEED_PENDING_MAX = 8;
const COMPASS_PX_PER_DEG = 2;

export interface HudAmmoContext {
  /** Initial magazine plus reserve for the current weapon/loadout; never inferred from the first packet. */
  initialTotal?: number;
  nearCrate?: boolean;
  inLoadout?: boolean;
}

export interface HudSquadMember {
  id: number;
  name: string;
  role?: string;
  status?: 'alive' | 'downed' | 'dead';
  distanceM?: number;
  bearingDeg?: number;
}

interface HudSources {
  phase: DownedPhase;
  cameraYawRad: number;
  stance: number;
  ammoContext: HudAmmoContext;
  /** Spread in pixels at the crosshair. */
  spread: number;
  reloadProgress: number;
  weapon: { name: string; slot?: number; mode?: string; throwables?: number };
  /** TEAM timestamp on the same monotonic clock as update(). */
  squad: { members: readonly HudSquadMember[]; teamUpdatedAtMs: number };
  capture: { name?: string; inZone: boolean; friendly: number; enemy: number; owner: 'friend' | 'enemy' | 'neutral'; progress: number };
  compassMarkers: readonly { id: string | number; label: string; bearingDeg: number }[];
  interaction: { label: string; key?: string; progress?: number };
  /** Authoritative normalized intensity, when the host has it. Otherwise use the timed event only. */
  suppression: number;
  bleeding: boolean;
  reduceMotion: boolean;
  /** Estimated current server tick and tick rate, for C4 match.phase's endsAt tick. */
  serverClock: { nowTick: number; tickRate: number };
  frontline: readonly { id: number; label: string; owner: 'friend' | 'enemy' | 'neutral' }[];
}
export type HudInputs = { readonly [K in keyof HudSources]?: ReadonlySignal<HudSources[K] | undefined> };

export interface HudOptions {
  game: GameState;
  bus: EventBus<GameEvents>;
  /** C4 player ID (not the interpolated entity slot). */
  selfId?: number;
  /** Optional C4 roster for kill-feed identity. No roster data is treated as a live pose/status. */
  roster?: Pick<Roster, 'get'>;
  /** Optional initial C4 match phase; further changes arrive on the bus. */
  matchEndsAtTick?: number;
  inputs?: HudInputs;
  /** Same monotonic clock as the timestamps passed to update(). */
  now?: () => number;
}

type HudKill = GameEvents['kill.feed'] & {
  id: number;
  killerName?: string;
  victimName?: string;
  friendlyFire: boolean;
  local: boolean;
};

export type HudStore = ReturnType<typeof createHudStore>;

/** Per-session HUD bindings. Call update() after GameLoop's frame update, and dispose() at session teardown. */
export function createHudStore({ game, bus, selfId, roster, matchEndsAtTick, inputs = {}, now = () => performance.now() }: HudOptions) {
  const initialized = signal(false);
  const mag = signal(0);
  const reserve = signal(0);
  const predictedStance = signal(0);
  const staminaQ = signal(0);
  const hp = signal(1);
  const heading = signal(headingDeg(inputs.cameraYawRad?.peek() ?? game.local.yaw));
  const matchEnd = signal(matchEndsAtTick);
  const feed = signal<readonly HudKill[]>([]);
  const queue = new KillFeedQueue<HudKill>();
  const hit = signal<'hit' | 'headshot' | undefined>(undefined);
  const damage = signal<{ angleRad: number; opacity: number } | undefined>(undefined);
  const eventSuppression = signal(0);
  const fx = signal<{ state: FxState; intensity: number; reducedMotion: boolean }>({ state: 'clear', intensity: 0, reducedMotion: false });
  const squadFresh = signal(false);
  let hitUntil = -Infinity;
  let suppressUntil = -Infinity;
  let damageAt = -Infinity;
  let damageStrength = 0;
  let nextSample = -Infinity;
  let killId = 0;
  let feedUntil = Infinity;
  let disposed = false;

  const phase = computed(() => inputs.phase?.value);
  const publishFeed = () => {
    feedUntil = (queue.visible[0]?.shownAt ?? Infinity) + KILLFEED_MS;
    feed.value = queue.visible.map(({ item }) => item);
  };
  const off = [
    bus.on('hit.confirm', ({ zone }) => {
      // The wire HitZone enum uses 0 for Head. Do not synthesize hits from fire.local.
      hitUntil = now() + HIT_CONFIRM_MS;
      hit.value = zone === 0 ? 'headshot' : 'hit';
    }),
    bus.on('damage.taken', ({ angle, amount }) => {
      // The event is already view-relative, with damage normalized to the wire's 100 HP full scale.
      damageAt = now();
      damageStrength = wedgeStrength(amount * 100);
      damage.value = { angleRad: angle, opacity: damageStrength };
    }),
    bus.on('suppress', ({ intensity }) => {
      suppressUntil = now() + SUPPRESS_EVENT_MS;
      eventSuppression.value = intensity;
    }),
    bus.on('kill.feed', (entry) => {
      const at = now();
      const changed = at >= feedUntil && queue.update(at);
      // Do not keep a personal kill detail from the downed stage for later display.
      if ((entry.victim === selfId && phase.peek() === 'downed') || queue.pending >= FEED_PENDING_MAX) {
        if (changed) publishFeed();
        return;
      }
      // 255 is the C4 environment sentinel, not a player identity.
      const killer = entry.killer === 255 ? undefined : roster?.get(entry.killer);
      const victim = roster?.get(entry.victim);
      const visibleBefore = queue.visible.length;
      queue.push({
        ...entry, id: ++killId, killerName: killer?.name, victimName: victim?.name,
        friendlyFire: !!killer && !!victim && killer.id !== victim.id && killer.team === victim.team,
        local: (entry.killer !== 255 && entry.killer === selfId) || entry.victim === selfId,
      }, at);
      if (changed || queue.visible.length !== visibleBefore) publishFeed();
    }),
    bus.on('match.phase', ({ endsAt }) => { matchEnd.value = endsAt; }),
    effect(() => {
      const suppression = unit(inputs.suppression?.value ?? eventSuppression.value);
      const reducedMotion = inputs.reduceMotion?.value === true;
      const bleeding = inputs.bleeding?.value === true;
      const previous = fx.peek();
      const state = fxState(previous.state, { downed: phase.value === 'downed', hpFrac: hp.value, suppression, bleeding });
      const intensity = fxVignette(state, suppression, Math.min(0.25, hp.value), reducedMotion);
      if (state !== previous.state || intensity !== previous.intensity || reducedMotion !== previous.reducedMotion) {
        fx.value = { state, intensity, reducedMotion };
      }
    }),
  ];

  function update(at = now()): void {
    if (disposed) return;
    batch(() => {
      // snapshotTick starts at 0 and snapshots without self leave it unchanged. Until a nonzero
      // accepted self tick is seen, zeros in net must not be presented as actual health/ammo.
      if (at >= nextSample) {
        nextSample = at + SNAP_INTERVAL_MS;
        const net = game.net;
        if (initialized.peek() || net.snapshotTick.peek() !== 0) {
          initialized.value = true;
          mag.value = net.ammoAuth.peek();
          reserve.value = net.reserveAuth.peek();
          hp.value = unit(net.health.peek() / 255);
          staminaQ.value = net.stamina.peek();
          predictedStance.value = game.local.stance;
        }
      }
      heading.value = headingDeg(inputs.cameraYawRad?.peek() ?? game.local.yaw);
      const team = inputs.squad?.peek();
      squadFresh.value = team !== undefined && showExactDistance(team.teamUpdatedAtMs, at);
      if (at >= hitUntil) hit.value = undefined;
      if (at >= suppressUntil) eventSuppression.value = 0;
      const wedge = damage.peek();
      if (wedge) {
        const age = Math.max(0, at - damageAt);
        if (age >= WEDGE_MS) damage.value = undefined;
        else if (age !== 0) damage.value = { angleRad: wedge.angleRad, opacity: wedgeOpacity(age, damageStrength) };
      }
      if (at >= feedUntil && queue.update(at)) publishFeed();
    });
  }

  update();
  return {
    phase,
    heading: computed(() => Math.floor(heading.value)),
    compassOffset: computed(() => compassOffsetPx(heading.value, COMPASS_PX_PER_DEG)),
    compassMarkers: computed(() => inputs.compassMarkers?.value?.slice(0, 8)),
    ammo: computed(() => {
      if (!initialized.value) return undefined;
      const ctx = inputs.ammoContext?.value;
      const tier = ctx?.initialTotal && ctx.initialTotal > 0 ? ammoTier(mag.value + reserve.value, ctx.initialTotal) : undefined;
      const exact = showExactAmmo(ctx?.nearCrate === true, ctx?.inLoadout === true);
      return { tier, emptyMag: magEmpty(mag.value), exact: exact ? { mag: mag.value, reserve: reserve.value } : undefined };
    }),
    stance: computed(() => {
      const authoritative = inputs.stance?.value;
      return authoritative !== undefined ? { value: authoritative, source: 'authoritative' as const }
        : initialized.value ? { value: predictedStance.value, source: 'predicted' as const } : undefined;
    }),
    stamina: computed(() => {
      if (!initialized.value) return undefined;
      const value = unit(staminaQ.value / 63);
      return { value, warn: staminaWarn(value) };
    }),
    healthBand: computed(() => initialized.value ? healthBand(hp.value) : undefined),
    spread: computed(() => inputs.spread?.value),
    reloadProgress: computed(() => fraction(inputs.reloadProgress?.value)),
    weapon: computed(() => inputs.weapon?.value),
    squad: computed(() => inputs.squad?.value?.members),
    squadFresh: computed(() => squadFresh.value),
    capture: computed(() => {
      const c = inputs.capture?.value;
      if (!c) return undefined;
      const state = captureState(c.inZone, c.friendly, c.enemy, c.owner, c.progress);
      return state === 'hidden' ? undefined : { name: c.name, state, segments: ringSegments(c.progress) };
    }),
    matchSeconds: computed(() => {
      const tick = matchEnd.value;
      const clock = inputs.serverClock?.value;
      return tick && clock && clock.tickRate > 0 ? countdownS(tick, clock.nowTick, clock.tickRate) : undefined;
    }),
    frontline: computed(() => inputs.frontline?.value?.slice(0, 5)),
    feed: computed(() => {
      if (phase.value !== 'downed') return feed.value;
      return selfId === undefined ? [] : feed.value.filter((e) => e.victim !== selfId);
    }),
    interaction: computed(() => inputs.interaction?.value),
    hit: computed(() => hit.value),
    damage: computed(() => damage.value),
    fx: computed(() => fx.value),
    update,
    dispose() {
      disposed = true;
      for (const unsubscribe of off.splice(0)) unsubscribe();
    },
  };
}

const unit = (value: number): number => fraction(value) ?? 0;
export function fraction(value: number | undefined): number | undefined {
  return value !== undefined && Number.isFinite(value) ? Math.min(1, Math.max(0, value)) : undefined;
}
