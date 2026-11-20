import { describe, expect, it } from 'vitest';
import { BUTTONS } from '../input/buttons';
import { InputSampler } from '../input/sampler';
import { decodePlacementResult, facilityAction, placementCommit, placementPreview } from '../net/placement';
import { LivePlacement } from './live-placement';

function fixture() {
  const sent: Uint8Array[] = [];
  const sampler = new InputSampler();
  let now = 0;
  const placement = new LivePlacement(sampler, (b) => sent.push(b), () => now);
  const step = () => placement.build(0, 0, 0);
  const reply = (id: number, reason = 0, target = true) => {
    placement.receive(target
      ? Uint8Array.of(0x18, 0, id, reason, 50, 1, 0xff, 0xff, 0xff, 0x9c, 0, 0, 1, 0xf4, 0, 0, 9, 0x60)
      : Uint8Array.of(0x18, 0, id, reason, 0, 0));
  };
  return { sent, sampler, placement, step, reply, advance: (ms: number) => { now += ms; } };
}

describe('live placement input and C0', () => {
  it('matches the Rust C0 encoding and rejects invalid response lengths', () => {
    expect([...placementPreview(0x1234, 2, 0x4321, -4096)]).toEqual([0x17, 0x12, 0x34, 2, 0x43, 0x21, 0xf0, 0]);
    expect([...placementCommit(1, 0x1234)]).toEqual([0x17, 0, 1, 3, 0x12, 0x34]);
    expect(decodePlacementResult(Uint8Array.of(0x18, 0x12, 0x34, 7, 0, 1, 0xff, 0xff, 0xff, 0x9c, 0, 0, 1, 0xf4, 0, 0, 9, 0x60)))
      .toEqual({ id: 0x1234, reason: 7, cost: 0, target: [-1, 5, 24] });
    expect(decodePlacementResult(Uint8Array.of(0x18, 0, 0, 0, 0, 1))).toBeNull();
    expect([...facilityAction(1, 'reorient', 0x1234, 0x4321)]).toEqual([0x17, 0, 1, 7, 2, 0x12, 0x34, 0x43, 0x21]);
  });

  it.each([['Digit9', 4], ['Digit0', 5], ['BracketRight', 6]] as const)('previews facility with %s', (key, slot) => {
    const f = fixture();
    f.sampler.press(key);
    f.step();
    expect(f.sent).toEqual([placementPreview(0, slot, 0, 0)]);
  });

  it('commits only a matched legal preview and suppresses firing through release', () => {
    const f = fixture();
    f.sampler.press('Digit6');
    f.step();
    expect(f.sent).toEqual([placementPreview(0, 0, 0, 0)]);
    f.reply(9);
    expect(f.placement.view.value.preview).toBeNull();
    f.reply(0);
    expect(f.placement.view.value.target).toEqual([-1, 5, 24]);
    f.sampler.press('Mouse0');
    expect(f.step().buttons & BUTTONS.FIRE).toBe(0);
    expect(f.sent[1]).toEqual(placementCommit(1, 0));
    expect(f.placement.view.value.state.mode).toBe('committing');
    f.reply(1, 0, false);
    expect(f.placement.view.value.state.mode).toBe('idle');
    expect(f.step().buttons & BUTTONS.FIRE).toBe(0);
    f.sampler.release('Mouse0');
    f.step();
    f.sampler.press('Mouse0');
    expect(f.step().buttons & BUTTONS.FIRE).toBe(BUTTONS.FIRE);
  });

  it('preserves the refusal, never auto-commits, and cancels on right click or input clear', () => {
    const f = fixture();
    f.sampler.press('Digit8'); f.step(); f.reply(0);
    f.sampler.press('Mouse0'); f.sampler.release('Mouse0'); f.step();
    f.reply(1, 7, false);
    expect(f.placement.view.value.state.reasonKey).toBe('ui.officer.err.noBuildZone');
    f.advance(500); f.step(); f.reply(2);
    expect(f.sent).toHaveLength(3);
    f.sampler.press('Mouse2');
    expect(f.step().buttons & BUTTONS.AIM).toBe(0);
    expect(f.placement.view.value.state.mode).toBe('idle');
    f.sampler.press('Digit7'); f.step();
    f.sampler.releaseAll(); f.reply(3);
    expect(f.placement.view.value.state.mode).toBe('idle');
  });

  it('does not commit a stale aim and keeps USE separate from the vehicle binding', () => {
    const f = fixture();
    f.sampler.press('Digit6'); f.step(); f.reply(0);
    f.sampler.setView(1, -0.6);
    f.sampler.press('Mouse0'); f.sampler.press('KeyJ');
    const cmd = f.step();
    expect(cmd.buttons & (BUTTONS.FIRE | BUTTONS.USE | BUTTONS.VEHICLE_SEAT)).toBe(BUTTONS.USE);
    expect(f.sent).toHaveLength(1);
    expect(f.placement.view.value.preview?.legal).toBe(false);
  });

  it('expires an old preview and never replays a commit whose reply timed out', () => {
    const f = fixture();
    f.sampler.press('Digit8'); f.step(); f.reply(0);
    f.advance(751);
    f.sampler.press('Mouse0'); f.sampler.release('Mouse0'); f.step();
    expect(f.sent[1]).toEqual(placementPreview(1, 2, 0, 0));
    f.reply(1);
    f.sampler.press('Mouse0'); f.step();
    expect(f.sent[2]).toEqual(placementCommit(2, 1));
    f.advance(1001); f.step();
    expect(f.placement.view.value.state.reasonKey).toBe('ui.officer.err.previewUnavailable');
    f.reply(2, 0, false);
    f.advance(500); f.step();
    expect(f.sent[3]).toEqual(placementPreview(3, 2, 0, 0));
  });
});
