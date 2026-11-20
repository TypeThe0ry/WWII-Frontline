import { signal } from '@preact/signals';
import { render } from 'preact';
import { EventBus } from '../../game/event-bus';
import { GameState } from '../../game/state';
import { createHudStore, Hud } from '../hud';

// Vite dev fixture only. A real store and HUD, without the game loop, network, or renderer.
if (import.meta.env.DEV) {
  const game = new GameState();
  game.net.snapshotTick.value = 1;
  game.net.ammoAuth.value = 3;
  game.net.health.value = 128;
  game.net.stamina.value = 32;
  const phase = signal<'alive' | 'spectating'>('alive');
  const store = createHudStore({ game, bus: new EventBus(), now: () => 1000, inputs: {
    phase, weapon: signal({ name: 'Fixture' }), ammoContext: signal({ initialTotal: 20 }),
    squad: signal({ members: Array.from({ length: 8 }, (_, id) => ({ id, name: `Member ${id}` })), teamUpdatedAtMs: 1000 }),
  } });
  const host = document.querySelector('[data-hud-host]')!;
  render(<Hud store={store} />, host);
  // Same host and same mount; verify the populated HUD before requesting the no-content phase.
  host.addEventListener('perf:spectate', () => { phase.value = 'spectating'; }, { once: true });
}
