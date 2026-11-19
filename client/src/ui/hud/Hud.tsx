import { memo } from 'preact/compat';
import { locale } from '../i18n';
import { ammoWarn, compassLabel, SQUAD_ROWS } from './hud-math';
import type { HudStore } from './hud-signals';

type Props = { store: HudStore };
const text = (en: string, zh: string) => locale.value === 'zh-Hans' ? zh : en;
const localized = ([en, zh]: readonly [string, string]) => text(en, zh);
const TICKS = Array.from({ length: 36 }, (_, i) => i * 10);
const CARDINALS = Array.from({ length: 8 }, (_, i) => i * 45);
const AMMO = { full: ['Full', '满'], high: ['High', '多'], mid: ['Medium', '中'], low: ['Low', '少'], empty: ['Near empty', '空'] } as const;
const STANCE = [['Standing', '站立'], ['Crouched', '蹲伏'], ['Prone', '卧倒']] as const;
const HEALTH = [['Healthy', '健康'], ['Hurt', '受伤'], ['Badly hurt', '重伤'], ['Critical', '危急']] as const;
const LIFE = { alive: ['Alive', '存活'], downed: ['Downed', '倒地'], dead: ['Dead', '阵亡'] } as const;

/** Presentational only. The host owns mounting, CSS/positioning, and pointer lock. */
export function Hud({ store }: Props) {
  const phase = store.phase.value;
  if (phase === 'dead' || phase === 'spectating') return null;
  return <div data-hud="" aria-label={text('Combat HUD', '战斗界面')}>
    <div data-hud-region="center">
      <Crosshair store={store} />
      <Reload store={store} />
      <DamageIndicator store={store} />
    </div>
    <div data-hud-region="top"><Compass store={store} /></div>
    <div data-hud-region="top-left">
      <MatchClock store={store} />
      <CapturePanel store={store} />
    </div>
    <div data-hud-region="top-right"><KillFeed store={store} /></div>
    <div data-hud-region="left"><SquadList store={store} /></div>
    <div data-hud-region="bottom"><InteractPrompt store={store} /></div>
    <div data-hud-region="bottom-right">
      <AmmoCounter store={store} />
      <WeaponSlots store={store} />
      <StanceIndicator store={store} />
      <StaminaBar store={store} />
    </div>
    <HealthBand store={store} />
    <ScreenFx store={store} />
  </div>;
}

function Crosshair({ store }: Props) {
  const hit = store.hit.value;
  return <span data-hud-part="crosshair" data-level="0" data-hit={hit} data-spread={store.spread.value} aria-hidden="true">
    {hit === 'headshot' ? '✦' : hit === 'hit' ? '×' : '+'}
  </span>;
}

function Compass({ store }: Props) {
  return <div data-hud-part="compass" aria-label={text('Compass', '指南针')}>
    <CompassHeading store={store} />
    <CompassTape store={store} />
  </div>;
}

function CompassHeading({ store }: Props) {
  return <span data-compass-heading="">{compassLabel(store.heading.value, locale.value === 'zh-Hans')}</span>;
}

function CompassTape({ store }: Props) {
  return <div data-compass-tape="" data-offset-px={store.compassOffset.value} aria-hidden="true">
    <CompassMarks store={store} />
  </div>;
}

const CompassMarks = memo(function CompassMarks({ store }: Props) {
  const zh = locale.value === 'zh-Hans';
  const markers = store.compassMarkers.value;
  return <>
    <span data-compass-ticks="">{TICKS.map((deg) => <span key={deg} data-bearing-deg={deg}>{deg}° </span>)}</span>
    <span data-compass-cardinals="">{CARDINALS.map((deg) => <span key={deg} data-bearing-deg={deg}>{compassLabel(deg, zh)} </span>)}</span>
    {markers?.map((m) => <span key={m.id} data-compass-marker="" data-bearing-deg={m.bearingDeg}>{m.label}</span>)}
  </>;
});

function AmmoCounter({ store }: Props) {
  const ammo = store.ammo.value;
  if (!ammo) return null;
  const warn = ammo.emptyMag || (ammo.tier !== undefined && ammoWarn(ammo.tier));
  return <div data-hud-part="ammo" data-level="0" data-tier={ammo.tier} data-empty-mag={ammo.emptyMag} data-warn={warn}>
    {text('Ammo:', '弹药：')} {' '}
    {warn ? <span aria-hidden="true">⚠ </span> : null}
    {ammo.exact ? <span data-ammo-exact="">{ammo.exact.mag} / {ammo.exact.reserve} </span> : null}
    {ammo.tier ? localized(AMMO[ammo.tier]) : !ammo.emptyMag && !ammo.exact ? '—' : null}
    {ammo.emptyMag ? <span> {text('Empty magazine', '空仓')}</span> : null}
  </div>;
}

function Reload({ store }: Props) {
  const p = store.reloadProgress.value;
  return p === undefined ? null : <progress data-hud-part="reload" data-level="0" value={p} max={1} aria-label={text('Reloading', '换弹')} />;
}

function WeaponSlots({ store }: Props) {
  const weapon = store.weapon.value;
  if (!weapon) return null;
  return <div data-hud-part="weapon" data-weapon-slot={weapon.slot}>
    {weapon.name}{weapon.mode ? ` · ${weapon.mode}` : ''}
    {weapon.throwables !== undefined ? <> · {text('Throwables:', '投掷物：')} {weapon.throwables}</> : null}
  </div>;
}

function StanceIndicator({ store }: Props) {
  const stance = store.stance.value;
  const label = stance === undefined ? undefined : STANCE[stance.value];
  return label && stance ? <span data-hud-part="stance" data-stance={stance.value} data-source={stance.source}>{localized(label)}</span> : null;
}

function StaminaBar({ store }: Props) {
  const stamina = store.stamina.value;
  if (!stamina) return null;
  return <div data-hud-part="stamina" data-warn={stamina.warn}>
    <meter value={stamina.value} min={0} max={1} low={0.2} aria-label={text('Stamina', '体力')} />
    {stamina.warn ? <span>{stamina.value === 0 ? text('Exhausted', '体力耗尽') : text('Low stamina', '体力不足')}</span> : null}
  </div>;
}

function HealthBand({ store }: Props) {
  const band = store.healthBand.value;
  return band === undefined ? null : <span data-hud-part="health" data-health-band={band}>{localized(HEALTH[band])}</span>;
}

function SquadList({ store }: Props) {
  const members = store.squad.value;
  if (!members?.length) return null;
  const fresh = store.squadFresh.value;
  const zh = locale.value === 'zh-Hans';
  return <ul data-hud-part="squad" data-stale={!fresh} aria-label={text('Squad', '小队')}>
    {members.slice(0, SQUAD_ROWS).map((m) => <li key={m.id} data-member-id={m.id} data-member-status={m.status}>
      {m.name}{m.role ? ` · ${m.role}` : ''}{m.status ? ` · ${localized(LIFE[m.status])}` : ''}
      {m.bearingDeg !== undefined ? ` · ${compassLabel(m.bearingDeg, zh)}` : ''}
      {fresh && m.distanceM !== undefined ? ` · ${Math.round(m.distanceM)} m` : ''}
    </li>)}
  </ul>;
}

function CapturePanel({ store }: Props) {
  const capture = store.capture.value;
  if (!capture) return null;
  const { friend, neutral, enemy } = capture.segments;
  return <div data-hud-part="capture" data-state={capture.state}>
    {capture.name ?? text('Objective', '据点')} · {capture.state === 'contested' ? text('Contested', '争夺中')
      : capture.state === 'captured' ? text('Captured', '已占领') : text('Capture zone', '占领圈')}
    <meter data-segment="friend" value={friend} max={1} aria-label={text('Friendly control', '我方控制')} />
    <meter data-segment="neutral" value={neutral} max={1} aria-label={text('Neutral control', '中立')} />
    <meter data-segment="enemy" value={enemy} max={1} aria-label={text('Enemy control', '敌方控制')} />
  </div>;
}

function MatchClock({ store }: Props) {
  const seconds = store.matchSeconds.value;
  const frontline = store.frontline.value;
  if (seconds === undefined && !frontline?.length) return null;
  return <div data-hud-part="match">
    {seconds === undefined ? null : <time data-match-clock="" dateTime={`PT${seconds}S`} aria-label={text('Match time remaining', '对局剩余时间')}>
      {Math.floor(seconds / 60)}:{String(seconds % 60).padStart(2, '0')}
    </time>}
    {frontline?.length ? <ul data-frontline="">{frontline.map((lane) => <li key={lane.id} data-owner={lane.owner}>{lane.label}</li>)}</ul> : null}
  </div>;
}

function KillFeed({ store }: Props) {
  const entries = store.feed.value;
  if (!entries.length) return null;
  return <ul data-hud-part="kill-feed" data-level="1" aria-label={text('Kill feed', '击杀播报')}>
    {entries.map((e) => <li key={e.id} data-local={e.local} data-friendly-fire={e.friendlyFire} data-headshot={e.headshot}>
      {e.friendlyFire ? <span>◆ {text('Friendly fire', '友军误伤')} · </span> : null}
      {e.killer === 255 ? text('Environment', '环境') : e.killerName ?? `#${e.killer}`}
      {' → '}{e.victimName ?? `#${e.victim}`} · {text('Weapon', '武器')} #{e.weapon}
    </li>)}
  </ul>;
}

function InteractPrompt({ store }: Props) {
  const prompt = store.interaction.value;
  if (!prompt) return null;
  return <div data-hud-part="interaction">
    <kbd>{prompt.key || text('Unbound', '未绑定')}</kbd> {prompt.label}
    {prompt.progress === undefined ? null : <progress max={1} value={prompt.progress} aria-label={prompt.label} />}
  </div>;
}

function DamageIndicator({ store }: Props) {
  const damage = store.damage.value;
  return damage ? <span data-hud-part="damage" data-level="0" data-angle-rad={damage.angleRad} data-opacity={damage.opacity} aria-hidden="true">▲</span> : null;
}

function ScreenFx({ store }: Props) {
  const fx = store.fx.value;
  return <span data-hud-part="fx" data-state={fx.state} data-intensity={fx.intensity} data-reduce-motion={fx.reducedMotion} aria-hidden="true" />;
}
