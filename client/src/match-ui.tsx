import { computed, signal } from '@preact/signals';
import { render } from 'preact';
import type { AudioApi } from './audio/engine';
import type { EventBus, GameEvents } from './game';
import type { NetGameHandle } from './game/net-game';
import type { SettingsStore } from './game/settings';
import { Bindings, type Persist } from './input/bindings';
import { DownedDeath, createDownedUi } from './ui/downed';
import { Hud, createHudStore } from './ui/hud';
import { t } from './ui/i18n';
import { TacticalMap, type MapData } from './ui/map';
import { uiScreen } from './ui/screens';
import { mountPauseSettings } from './ui/settings';

export interface MatchUiOptions {
  canvas: HTMLCanvasElement;
  game: NetGameHandle;
  bus: EventBus<GameEvents>;
  settings: SettingsStore;
  persist: Persist;
  audio?: AudioApi;
  onFrame(callback: () => void): () => void;
  applyFov(degrees: number): void;
  onLeave(): void;
}

/** Browser composition only. Never infer authoritative team, squad, life or objectives from an AOI entity list. */
export function mountMatchUi({ canvas, game: net, bus, settings, persist, audio, onFrame, applyFov, onLeave }: MatchUiOptions): () => void {
  const host = document.createElement('div');
  const pauseHost = document.createElement('div');
  canvas.before(host, pauseHost);
  const state = net.gameState;
  const connected = signal(true);
  const mapOpen = signal(false);
  const mapData = signal<MapData>({});
  const serverClock = signal({ nowTick: net.session.welcome.serverTick, tickRate: net.session.welcome.tickHz });
  const { playerId } = net.session.welcome;
  const downed = createDownedUi({ inputs: { connected } });
  const hud = createHudStore({ game: net.game.state, bus, selfId: playerId, roster: state.roster, inputs: { phase: downed.phase, serverClock } });
  const down = computed(() => downed.phase.value === 'downed');
  let mapAt = -Infinity;
  let clockAt = -Infinity;
  let disposed = false;
  const listener = { x: 0, y: 0, z: 0 };
  const forward = { x: 0, y: 0, z: 1 };
  const up = { x: 0, y: 1, z: 0 };

  function changeMap(open: boolean) {
    if (disposed || mapOpen.peek() === open) return;
    if (open) {
      // Display the first available data immediately; subsequent positions are throttled independently of C2 reception.
      updateMap(performance.now());
      pause.releaseInput();
    }
    mapOpen.value = open;
    if (open && document.pointerLockElement === canvas) pause.suspendForMap();
    uiScreen.value = open ? 'map' : downed.phase.peek() === 'alive' ? 'hud' : 'downed';
  }

  function updateMap(now: number) {
    mapAt = now;
    const local = net.game.state.local;
    // The current WELCOME advertises M0; its positions are already expressed in map-world metres.
    const self = net.game.stats.snapshots > 0 && net.game.state.net.health.peek() > 0;
    mapData.value = { nowS: now / 1000, units: self ? [{ id: playerId, x: local.pos[0]!, z: local.pos[2]!, relation: 'self', yaw: local.yaw }] : [] };
  }

  const pause = mountPauseSettings({ root: pauseHost, canvas, sampler: net.sampler, settings, bindings: new Bindings(), persist,
    getInputContext: () => mapOpen.peek() ? 'map' : down.peek() ? 'downed' : 'onFoot',
    onAction: (action, pressed) => {
      if (action === 'map' && pressed) changeMap(!mapOpen.peek());
      else if (action === 'interact') {
        // The live transport supports only the existing C1 tap/hold bit, not a targeted seat RPC.
        if (pressed && !down.peek()) net.sampler.press('KeyF');
        else net.sampler.release('KeyF');
      }
    },
    isMapOpen: () => mapOpen.peek(), onCloseMap: () => changeMap(false), onLeave,
  });
  applyFov(settings.get<number>('video.fov'));
  const offSetting = settings.onChange(({ key, value }) => { if (key === 'video.fov' && typeof value === 'number') applyFov(value); });
  const offDeath = bus.on('death', (event) => {
    if (event.victim !== playerId || !downed.transition(event.canBeRevived ? 'down' : 'die')) return;
    pause.releaseInput();
    changeMap(false);
    uiScreen.value = 'downed';
  });
  const offRevive = bus.on('revive', ({ target }) => {
    if (target === playerId && downed.transition('revive')) uiScreen.value = mapOpen.peek() ? 'map' : 'hud';
  });
  // Use the reliable roster on the same channel as Death and Revive. A positive C2 health byte
  // may describe a downed body or arrive out of order with respect to C4 on WebTransport.
  const offAlive = bus.on('player.alive', ({ player }) => {
    if (player === playerId && downed.transition('respawn')) uiScreen.value = mapOpen.peek() ? 'map' : 'hud';
  });
  const offDisconnect = net.session.onDisconnect(() => { connected.value = false; pause.releaseInput(); changeMap(false); });
  const offFrame = onFrame(() => {
    const now = performance.now();
    if (now - clockAt >= 100) {
      clockAt = now;
      serverClock.value = { nowTick: net.session.clock.estimatedServerTick(now), tickRate: net.session.welcome.tickHz };
    }
    hud.update(now);
    downed.update(now);
    if (mapOpen.peek() && now - mapAt >= 100) updateMap(now);
    if (audio) {
      const { pos, yaw } = net.game.state.local;
      listener.x = pos[0]!; listener.y = pos[1]!; listener.z = pos[2]!;
      forward.x = Math.sin(yaw); forward.z = Math.cos(yaw);
      audio.setListener(listener, forward, up);
    }
  });

  function Match() {
    return <main>
      <nav>
        <button onClick={() => changeMap(!mapOpen.peek())}>{t('ui.match.map')}</button>
        <button onClick={() => { changeMap(false); pause.open(); }}>{t('ui.match.pause')}</button>
      </nav>
      {!connected.value && <p role="alert">{t('ui.match.disconnected')} <button onClick={onLeave}>{t('common.back')}</button></p>}
      {mapOpen.value ? <TacticalMap data={mapData.value} onClose={() => changeMap(false)} /> : <>
        <Hud store={hud} />
        <DownedDeath store={downed} />
      </>}
      {audio && new URLSearchParams(location.search).has('uiperf') && <output aria-label="Audio latency">
        {t('ui.match.audioLatency', { ms: (audio.stats().baseLatency * 1000).toFixed(2) })}
      </output>}
    </main>;
  }
  render(<Match />, host);
  uiScreen.value = 'hud';
  return () => {
    if (disposed) return;
    disposed = true;
    offFrame(); offDisconnect(); offAlive(); offRevive(); offDeath(); offSetting();
    pause.dispose(); downed.dispose(); hud.dispose();
    render(null, host);
    pauseHost.remove(); host.remove();
  };
}
