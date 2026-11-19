import { describe, expect, it } from 'vitest';
import { SettingsStore } from '../../game/settings';
import { Bindings, Persist, type Context } from '../../input/bindings';
import { InputSampler } from '../../input/sampler';
import { MatchInputAdapter } from './input-adapter';

function setup() {
  const sampler = new InputSampler();
  const bindings = new Bindings();
  const settings = new SettingsStore(new Persist(() => undefined));
  let context: Context = 'onFoot';
  const actions: unknown[][] = [];
  const input = new MatchInputAdapter(sampler, bindings, settings, () => context, (...event) => { actions.push(event); });
  return { sampler, settings, bindings, input, actions, context: (next: Context) => { context = next; } };
}

describe('settings input adapter', () => {
  it('rebinds physical codes without changing the sampler or releasing an action held by a second key', () => {
    const { bindings, sampler, input } = setup();
    bindings.rebind('reload', 'primary', 'KeyT');
    bindings.rebind('reload', 'secondary', 'Mouse4');
    expect(input.press('KeyR')).toBe(false);
    expect(input.press('KeyT')).toBe(true);
    input.press('KeyT');
    input.press('Mouse4');
    input.release('KeyT');
    expect(sampler.isHeld('reload')).toBe(true);
    input.release('Mouse4');
    expect(sampler.isHeld('reload')).toBe(false);
  });

  it('applies independent axis sensitivity, invert Y and the ADS multiplier before sampling', () => {
    const { settings, sampler, input } = setup();
    settings.set('input.sensH', 2);
    settings.set('input.sensV', 3);
    settings.set('input.adsMult', 0.5);
    settings.set('input.invertY', true);
    input.press('Mouse2');
    input.mouseMove(10, 10);
    input.release('Mouse2');
    input.mouseMove(10, 10);
    sampler.build(1, 1, 1);
    expect(sampler.yaw * 180 / Math.PI).toBeCloseTo(3);
    expect(sampler.pitch * 180 / Math.PI).toBeCloseTo(4.5);
  });

  it('routes unsampled actions and clears actions and accumulated motion on a context change', () => {
    const { context, sampler, input, actions } = setup();
    input.press('KeyM');
    input.press('KeyW');
    input.mouseMove(200, 100);
    context('menu');
    input.mouseMove(100, 100);
    expect(actions).toEqual([['map', true], ['map', false]]);
    const command = sampler.build(1, 1, 1);
    expect(command).toMatchObject({ moveX: 0, moveZ: 0, buttons: 0, yaw: 0 });
    expect(Math.abs(command.pitch)).toBe(0);
  });
});
