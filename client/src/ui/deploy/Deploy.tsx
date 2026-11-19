import { useLayoutEffect, useMemo, useRef, useState } from 'preact/hooks';
import type { TargetedKeyboardEvent } from 'preact';
import { SQUAD_NAMES } from '../../game/deploy-actions';
import type { DeployActions, DeployResult } from '../../game/deploy-actions';
import { countdownS } from '../../game/deploy-validate';
import type { SquadKind } from '../../game/deploy-validate';
import { MapCanvas } from '../map';
import type { MapData, MapSpawn } from '../map';
import { makeDeployRequest, pickDefaultSpawn, recommendSpawn, spawnIssue } from './model';
import type { DeployLoadout, DeployRole, DeploySquad } from './model';

type Accepted = Extract<DeployResult, { ok: true }>;
type Status = { phase: 'browsing' | 'requesting' } | { phase: 'accepted'; tick: number }
  | { phase: 'rejected'; message: string; refusedId?: number };

export interface DeployProps {
  /** Stable per session. Omit for read-only deployment. */
  actions?: Pick<DeployActions, 'deploy' | 'createSquad' | 'joinSquad' | 'leaveSquad' | 'lockSquad'>;
  /** Friendly, world-space points only; the HQ option must use HQ_SPAWN_ID. */
  spawnPoints: readonly MapSpawn[];
  /** null = confirmed Unassigned; undefined = roster membership unknown. */
  squadId: number | null | undefined;
  squads?: readonly DeploySquad[];
  roles: readonly DeployRole[];
  /** Already filtered to the player's faction; locked options may set unavailableReason. */
  loadouts: readonly DeployLoadout[];
  map?: Omit<MapData, 'spawnPoints'>;
  nowTick?: number;
  tickHz?: number;
  unstable?: boolean;
  deployable?: boolean;
  /** The owner may begin prefetch here. Close/change screens on authoritative self-state, not the countdown. */
  onAccepted?: (result: Accepted, point: MapSpawn) => void;
  /** Optional prefetch completion fraction (0–1). */
  prefetchProgress?: number;
  onClose?: () => void;
}

const SQUAD_KIND_LABEL: Record<SquadKind, string> = { infantry: 'Infantry', armor_crew: 'Armor crew', recon: 'Recon' };

export function Deploy({ actions, spawnPoints, squadId, squads = [], roles, loadouts, map, nowTick, tickHz,
  unstable = false, deployable = true, onAccepted, prefetchProgress, onClose }: DeployProps) {
  const [spawnChoice, setSpawn] = useState<number>();
  const [roleChoice, setRole] = useState<number>();
  const [loadoutChoice, setLoadout] = useState<number>();
  const [squadKind, setSquadKind] = useState<SquadKind>('infantry');
  const [squadName, setSquadName] = useState<string>(SQUAD_NAMES.infantry[0]);
  const [squadNotice, setSquadNotice] = useState('');
  const [status, setStatus] = useState<Status>({ phase: 'browsing' });
  const inFlight = useRef(false);
  const generation = useRef(0);
  const acceptedCallback = useRef(onAccepted);
  useLayoutEffect(() => { acceptedCallback.current = onAccepted; }, [onAccepted]);
  useLayoutEffect(() => {
    inFlight.current = false;
    setStatus({ phase: 'browsing' });
    setSquadNotice('');
    return () => { generation.current++; };
  }, [actions]);
  useLayoutEffect(() => { setSquadNotice(''); }, [squadId]);

  const mapData = useMemo(() => ({ ...map, spawnPoints }), [map, spawnPoints]);
  const selectedSpawnId = spawnChoice ?? pickDefaultSpawn(spawnPoints, squadId);
  const spawn = spawnPoints.find((point) => point.id === selectedSpawnId);
  const selectedSpawnIssue = spawn && spawnIssue(spawn, squadId);
  const visibleRoles = squadId === null ? roles.filter((role) => role.id === 'rifleman') : roles;
  const role = visibleRoles.find((option) => option.index === roleChoice) ?? visibleRoles.find((option) => !option.unavailableReason);
  const visibleLoadouts = loadouts.filter((loadout) => loadout.roleId === role?.id);
  const loadout = visibleLoadouts.find((option) => option.index === loadoutChoice) ?? visibleLoadouts.find((option) => !option.unavailableReason);
  const currentSquad = squads.find((squad) => squad.id === squadId);
  const request = makeDeployRequest(spawn, role, loadout, squadId);
  const busy = status.phase === 'requesting' || status.phase === 'accepted';
  const canSend = Boolean(actions) && !unstable && !busy && squadId !== undefined;
  const canDeploy = canSend && deployable && request !== undefined;
  const refused = status.phase === 'rejected' ? status.refusedId : undefined;
  const recommended = refused === undefined ? undefined : recommendSpawn(spawnPoints, refused, squadId);
  const countdown = status.phase === 'accepted' && nowTick !== undefined && tickHz !== undefined && tickHz > 0
    ? countdownS(status.tick, nowTick, tickHz) : undefined;

  function chooseSpawn(point: MapSpawn) {
    if (busy || inFlight.current) return;
    setSpawn(point.id);
    if (status.phase === 'rejected') setStatus({ phase: 'browsing' });
  }
  function chooseRole(index: number) {
    setRole(index);
    setLoadout(undefined);
  }
  function squadCommand(send: (commands: NonNullable<DeployProps['actions']>) => void) {
    if (!actions || !canSend || inFlight.current) return;
    try {
      send(actions);
      setSquadNotice('Squad command sent. Waiting for server data.');
    } catch (error) { setSquadNotice(error instanceof Error ? error.message : 'Could not send squad command.'); }
  }
  async function submit() {
    if (!actions || !canDeploy || !request || !spawn || inFlight.current) return;
    inFlight.current = true;
    const requestGeneration = generation.current;
    setSpawn(spawn.id); // Keep the submitted point selected even if a subsequent server update removes it.
    setStatus({ phase: 'requesting' });
    let result: DeployResult;
    try { result = await actions.deploy(request); }
    catch (error) {
      if (requestGeneration !== generation.current) return;
      inFlight.current = false;
      setStatus({ phase: 'rejected', message: error instanceof Error ? error.message : 'Could not request deployment.' });
      return;
    }
    if (requestGeneration !== generation.current) return;
    if (!result.ok) {
      inFlight.current = false;
      setStatus({ phase: 'rejected', message: result.message, refusedId: request.spawnId });
      return;
    }
    setStatus({ phase: 'accepted', tick: result.spawnTick });
    acceptedCallback.current?.(result, spawn);
  }

  function createSquad() { squadCommand((commands) => commands.createSquad(squadKind, squadName)); }
  function toggleLock() {
    if (currentSquad?.isLeader && currentSquad.locked !== undefined) {
      const locked = !currentSquad.locked;
      squadCommand((commands) => commands.lockSquad(locked));
    }
  }
  function key(e: TargetedKeyboardEvent<HTMLElement>) {
    if (e.altKey || e.ctrlKey || e.metaKey || e.defaultPrevented) return;
    if (e.key === 'Escape' && onClose) {
      e.preventDefault(); e.stopPropagation();
      if (!e.repeat) onClose();
      return;
    }
    if (e.repeat || busy || (e.target instanceof HTMLElement && e.target.closest('select, input, textarea'))) return;
    const k = e.key.toLowerCase();
    if (/^[1-9]$/.test(k)) {
      const point = spawnPoints[Number(k) - 1];
      if (point) chooseSpawn(point);
    } else if (k === 'c' && squadId === null) createSquad();
    else if (k === 'l') toggleLock();
    else if (k === 'q' || k === 'e') {
      const options = visibleRoles.filter((item) => !item.unavailableReason);
      const i = options.findIndex((item) => item.index === role?.index);
      const next = options[i < 0 ? 0 : (i + options.length + (k === 'q' ? -1 : 1)) % options.length];
      if (next) chooseRole(next.index);
    } else if (k === 'enter' && !(e.target instanceof HTMLElement && e.target.closest('button, a'))) void submit();
    else return;
    e.preventDefault(); e.stopPropagation();
  }

  return <section aria-label="Deploy" tabIndex={-1} onKeyDown={key}>
    <h2>Deploy</h2>
    {onClose && <button type="button" onClick={onClose}>Close deploy</button>}
    <MapCanvas mode="deploy" data={mapData} selectedSpawnId={spawn?.id} recommendedSpawnId={recommended}
      center={spawnPoints.find((point) => point.id === recommended) ?? spawn} onPickSpawn={chooseSpawn} focusOnMount />
    <fieldset disabled={busy}>
      <legend>Spawn point</legend>
      {spawnPoints.length === 0 ? <p>No spawn data available.</p> : <ul>{spawnPoints.map((point, i) => {
        const issue = spawnIssue(point, squadId);
        return <li key={point.id}>
          <button type="button" aria-pressed={spawn?.id === point.id} onClick={() => chooseSpawn(point)}>
            {i < 9 ? `${i + 1}. ` : ''}{point.label} — {issue ?? 'Available'}{point.id === recommended ? ' (Recommended)' : ''}
          </button>
        </li>;
      })}</ul>}
      {spawn && <p>Selected: {spawn.label}{selectedSpawnIssue && ` — ${selectedSpawnIssue}`}</p>}
    </fieldset>
    <fieldset disabled={!canSend}>
      <legend>Squad</legend>
      <p>{squadId === undefined ? 'Waiting for your roster entry.' : squadId === null ? 'Unassigned' : currentSquad?.label ?? `Squad ${squadId}`}</p>
      {squads.length > 0 && <ul>{squads.map((squad) => <li key={squad.id}>
        {squad.label}{squad.kind && ` (${SQUAD_KIND_LABEL[squad.kind]})`}{squad.members !== undefined && ` — ${squad.members}${squad.capacity === undefined ? ' members' : `/${squad.capacity}`}`}
        {squad.locked === true && ' — Locked'}
        {squadId === null && <button type="button" onClick={() => squadCommand((commands) => commands.joinSquad(squad.id))}
          disabled={squad.locked === true || squad.id < 1 || squad.id > 255 ||
            (squad.members !== undefined && squad.capacity !== undefined && squad.members >= squad.capacity)}>Join {squad.label}</button>}
      </li>)}</ul>}
      {squadId === null && <div>
        <label>Squad type <select value={squadKind} onChange={(e) => {
          const kind = e.currentTarget.value as SquadKind;
          setSquadKind(kind);
          setSquadName(SQUAD_NAMES[kind][0]);
        }}>{(Object.keys(SQUAD_NAMES) as SquadKind[]).map((kind) => <option key={kind} value={kind}>{SQUAD_KIND_LABEL[kind]}</option>)}</select></label>
        <label>Preset suffix <select value={squadName} onChange={(e) => setSquadName(e.currentTarget.value)}>
          {SQUAD_NAMES[squadKind].map((name) => <option key={name}>{name}</option>)}
        </select></label>
        <button type="button" onClick={createSquad}>Create squad</button>
      </div>}
      {squadId != null && <button type="button" onClick={() => squadCommand((commands) => commands.leaveSquad())}>Leave squad</button>}
      {currentSquad?.isLeader && currentSquad.locked !== undefined && <button type="button" onClick={toggleLock}>
        {currentSquad.locked ? 'Unlock squad' : 'Lock squad'}
      </button>}
    </fieldset>
    {squadNotice && <p role="status">{squadNotice}</p>}
    <fieldset disabled={busy}>
      <legend>Role and loadout</legend>
      <label>Role <select value={role?.index ?? ''} onChange={(e) => chooseRole(Number(e.currentTarget.value))}>
        {!role && <option value="" disabled>No eligible role</option>}
        {visibleRoles.map((item) => <option key={item.index} value={item.index} disabled={Boolean(item.unavailableReason)}>
          {item.label}{item.quota && ` (${item.quota.used}/${item.quota.limit})`}{item.unavailableReason && ` — ${item.unavailableReason}`}
        </option>)}
      </select></label>
      <label>Loadout <select value={loadout?.index ?? ''} onChange={(e) => setLoadout(Number(e.currentTarget.value))}>
        {!loadout && <option value="" disabled>No eligible loadout</option>}
        {visibleLoadouts.map((item) => <option key={item.index} value={item.index} disabled={Boolean(item.unavailableReason)}>
          {item.label}{item.unavailableReason && ` — ${item.unavailableReason}`}
        </option>)}
      </select></label>
      {loadout?.items && <ul>{loadout.items.map((item, i) => <li key={i}>
        {item.slot}: {item.label}{item.magazine !== undefined && ` · ${item.magazine} rounds`}
        {item.rpm !== undefined && ` · ${item.rpm} rpm`}{item.weightKg !== undefined && ` · ${item.weightKg} kg`}
      </li>)}</ul>}
    </fieldset>
    <button type="button" disabled={!canDeploy} onClick={() => { void submit(); }}>Deploy</button>
    <div role="status">
      {!actions ? 'Deploy commands unavailable.' : unstable ? 'Connection unstable.' : !deployable && !busy ? 'Deployment is not available yet.' : null}
      {status.phase === 'requesting' && <p>Requesting deployment…</p>}
      {status.phase === 'rejected' && <p>{status.message}</p>}
      {status.phase === 'accepted' && <p>{countdown === undefined ? `Deployment accepted for server tick ${status.tick}.`
        : countdown > 0 ? `Spawning in ${countdown} s.` : 'Deployment accepted. Awaiting spawn confirmation.'}</p>}
    </div>
    {status.phase === 'accepted' && prefetchProgress !== undefined && Number.isFinite(prefetchProgress) && <label>
      Spawn data prefetch <progress max={1} value={Math.min(1, Math.max(0, prefetchProgress))} />
    </label>}
  </section>;
}
