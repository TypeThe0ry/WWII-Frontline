import { useEffect, useRef } from 'preact/hooks';
import { format, locale, type Params } from '../i18n';
import en from '../i18n/en.json';
import zh from '../i18n/zh-Hans.json';
import { compassLabel, headingDeg } from '../hud/hud-math';
import type { DownedUi } from './downed-ui';

type Props = { store: DownedUi };
interface DownedDeathProps extends Props {
  /** Optional game-data label for a weapon ID. */
  weaponName?: (weapon: number) => string | undefined;
}
const msg = (key: keyof typeof en, params?: Params) => format(locale.value, (locale.value === 'zh-Hans' ? zh : en)[key], params);
const text = (en: string, zh: string) => locale.value === 'zh-Hans' ? zh : en;
const UNAVAILABLE = () => text('Unavailable', '不可用');
const ARROWS = ['↑', '↗', '→', '↘', '↓', '↙', '←', '↖'];

/** The host owns screen focus/pointer lock and global binding dispatch. All controls use the same store gates. */
export function DownedDeath({ store, weaponName }: DownedDeathProps) {
  const phase = store.phase.value;
  if (phase === 'alive') return null;
  return <section data-downed-death="" data-phase={phase} aria-label={phase === 'downed' ? msg('ui.downed.title') : msg('ui.dead.title')}>
    {phase === 'downed' ? <DownedPanel store={store} /> : <DeathPanel store={store} weaponName={weaponName} />}
  </section>;
}

function DownedPanel({ store }: Props) {
  const connection = store.connection.value;
  useEffect(() => {
    const cancel = () => store.endGiveUp();
    const onVisibility = () => { if (document.hidden) cancel(); };
    window.addEventListener('blur', cancel);
    document.addEventListener('visibilitychange', onVisibility);
    return () => {
      window.removeEventListener('blur', cancel);
      document.removeEventListener('visibilitychange', onVisibility);
      cancel();
    };
  }, [store]);
  return <>
    <h2>{msg('ui.downed.title')}</h2>
    {connection === false ? <p role="status">{msg('ui.downed.reason.reconnecting')}</p> : null}
    <BleedoutRing store={store} />
    <NearestMedic store={store} />
    <ReviveProgress store={store} />
    <CallMedicButton store={store} />
    <GiveUpButton store={store} />
  </>;
}

function BleedoutRing({ store }: Props) {
  const bleed = store.bleedout.value;
  if (!bleed) return null;
  const label = text('Bleedout remaining', '流血剩余');
  return <div data-downed-part="bleedout">
    <meter min={0} max={1} value={bleed.fraction} aria-label={label} />
    {' '}{label}: <time dateTime={`PT${bleed.seconds}S`}>{bleed.seconds} s</time>
  </div>;
}

function NearestMedic({ store }: Props) {
  const medic = store.medic.value;
  if (medic === undefined) return null;
  if (medic === null) return <p data-downed-part="nearest-medic">{msg('ui.downed.noMedic')}</p>;
  const bearing = headingDeg(medic.bearingRad);
  const arrow = medic.relativeBearingRad === undefined ? undefined : ARROWS[Math.round(headingDeg(medic.relativeBearingRad) / 45) % 8];
  return <p data-downed-part="nearest-medic" data-bearing-rad={medic.bearingRad} data-relative-bearing-rad={medic.relativeBearingRad}>
    {text('Nearest medic:', '最近的医疗兵：')} {medic.distanceM} m · {compassLabel(bearing, locale.value === 'zh-Hans')}
    {arrow ? <span aria-hidden="true"> {arrow}</span> : null}
    {medic.enRoute ? ` · ${text('En route', '正在赶来')}` : null}
  </p>;
}

function ReviveProgress({ store }: Props) {
  const p = store.reviveProgress.value;
  if (p === undefined) return null;
  const label = text('Being revived', '正在救治');
  return <div data-downed-part="revive">{label} <progress value={p} max={1} aria-label={label} /></div>;
}

function CallMedicButton({ store }: Props) {
  const { available, enabled, cooldownS } = store.callMedicState.value;
  return <button type="button" data-downed-action="call-medic" disabled={!enabled} onClick={() => store.callMedic()}>
    {msg('ui.downed.callMedic')}{!available ? ` · ${UNAVAILABLE()}` : cooldownS > 0 ? ` · ${cooldownS} s` : ''}
  </button>;
}

function GiveUpButton({ store }: Props) {
  const { available, enabled, waitS } = store.giveUpState.value;
  const progress = store.giveUpProgress.value;
  const hold = useRef<number | 'key' | undefined>(undefined);
  const cancel = () => {
    if (hold.current === undefined) return;
    hold.current = undefined;
    store.endGiveUp();
  };
  const reason = !available || store.connection.value === undefined ? UNAVAILABLE()
    : waitS > 0 ? msg('ui.downed.reason.giveUpWait', { seconds: waitS }) : undefined;
  const label = msg('ui.downed.giveUp');
  return <div data-downed-part="give-up">
    <button type="button" data-downed-action="give-up" disabled={!enabled}
      onPointerDown={(e) => {
        if (!e.isPrimary || e.button !== 0 || !store.beginGiveUp()) return;
        hold.current = e.pointerId;
        e.currentTarget.setPointerCapture(e.pointerId);
      }}
      onPointerUp={(e) => { if (hold.current === e.pointerId) cancel(); }}
      onPointerCancel={(e) => { if (hold.current === e.pointerId) cancel(); }}
      onLostPointerCapture={(e) => { if (hold.current === e.pointerId) cancel(); }}
      onKeyDown={(e) => {
        if (e.key !== ' ' && e.key !== 'Enter') return;
        e.preventDefault();
        // Prevent a host-delegated key binding from handling the focused button a second time.
        e.stopPropagation();
        if (!e.repeat && store.beginGiveUp()) hold.current = 'key';
      }}
      onKeyUp={(e) => {
        if (e.key !== ' ' && e.key !== 'Enter') return;
        e.preventDefault();
        e.stopPropagation();
        if (hold.current === 'key') cancel();
      }}
      onBlur={cancel}>
      {label}{reason ? ` · ${reason}` : ''}
    </button>
    {progress === undefined ? null : <progress value={progress} max={1} aria-label={label} />}
  </div>;
}

function DeathPanel({ store, weaponName }: DownedDeathProps) {
  const seconds = store.respawnSeconds.value;
  const cycle = store.canCycleSpectate.value;
  return <>
    <h2>{msg('ui.dead.title')}</h2>
    <KillerDetails store={store} weaponName={weaponName} />
    {seconds === undefined ? null : <p data-downed-part="respawn-clock">
      {text('Respawn:', '重生：')} <time dateTime={`PT${seconds}S`}>{seconds} s</time>
    </p>}
    <button type="button" data-downed-action="redeploy" disabled={!store.canRedeploy.value} onClick={() => store.redeploy()}>
      {text('Redeploy', '重新部署')}
    </button>
    {cycle ? <div data-downed-part="spectate">
      <button type="button" data-downed-action="spectate-prev" onClick={() => store.cycleSpectate(-1)}>{text('Previous teammate', '上一名队友')}</button>
      <button type="button" data-downed-action="spectate-next" onClick={() => store.cycleSpectate(1)}>{text('Next teammate', '下一名队友')}</button>
    </div> : null}
  </>;
}

function KillerDetails({ store, weaponName }: DownedDeathProps) {
  const killer = store.killer.value;
  if (!killer) return null;
  const weapon = weaponName?.(killer.weapon) ?? `${text('Weapon', '武器')} #${killer.weapon}`;
  return <p data-downed-part="killer">
    {killer.name ? msg('ui.dead.killedBy', { name: killer.name, weapon, dist: killer.distM }) : `${weapon} · ${killer.distM} m`}
    {killer.tagKeys.map((key) => key === 'ui.downed.tag.ai' || key === 'ui.downed.tag.friendlyFire'
      ? <span key={key}> · {msg(key)}</span> : null)}
  </p>;
}
