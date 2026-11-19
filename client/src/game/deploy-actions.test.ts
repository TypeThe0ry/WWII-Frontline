import { afterEach, describe, expect, it, vi } from 'vitest';
import { Handlers } from '../net/transport';
import { createDeployActions, decodeDeployResult, HQ_SPAWN_ID } from './deploy-actions';
import { markerActions } from './marker-actions';

// MSB-first protocol fixtures: [0x11][ok:1][tick:32][reason:8][pad:7].
const accepted = Uint8Array.of(0x11, 0x89, 0x1a, 0x2b, 0x3c, 0x80, 0);
const refused = Uint8Array.of(0x11, 0, 0, 0, 0, 10, 0x80);
const request = { spawnId: HQ_SPAWN_ID, role: 3, loadout: 12, squadId: null } as const;

function fixture(timeoutMs?: number) {
  const c0 = new Handlers<[Uint8Array]>();
  const close = new Handlers<[string]>();
  const sendC0 = vi.fn<(bytes: Uint8Array) => void>();
  const free = vi.fn();
  const codec = {
    encodeC0: vi.fn((kind: number, payload: Uint8Array) => Uint8Array.of(kind, ...payload)),
    decodeServerC0: vi.fn((bytes: Uint8Array) => {
      if (bytes.length !== 7) throw new Error('bad payload');
      return { kind: 0x11, ok: (bytes[1]! & 0x80) !== 0, spawnTick: 0x12345679, reason: 21, free };
    }),
  };
  const actions = createDeployActions({ sendC0, onC0: (cb) => c0.add(cb), onClose: (cb) => close.add(cb) }, codec, timeoutMs);
  return { actions, c0, close, sendC0, codec, free };
}

afterEach(() => vi.useRealTimers());

describe('deploy commands over raw C0 transport', () => {
  it('sends BE spawn and unassigned sentinel, ignores unrelated/malformed C0, and rejects duplicates', async () => {
    const f = fixture();
    const pending = f.actions.deploy(request);
    expect(f.codec.encodeC0).toHaveBeenCalledWith(0x10, Uint8Array.of(0xff, 0xff, 3, 12, 0));
    expect(f.sendC0).toHaveBeenCalledExactlyOnceWith(Uint8Array.of(0x10, 0xff, 0xff, 3, 12, 0));
    f.c0.emit(Uint8Array.of(0x05));
    expect(f.codec.decodeServerC0).not.toHaveBeenCalled();
    f.c0.emit(Uint8Array.of(0x11));
    await expect(f.actions.deploy(request)).rejects.toThrow(/pending/);
    f.c0.emit(accepted);
    await expect(pending).resolves.toEqual({ ok: true, spawnTick: 0x12345679 });
    expect(f.free).toHaveBeenCalledTimes(1);
    f.actions.dispose();
  });

  it('reads the explicit ok bit, maps server reason codes, and allows retries after a known rejection', async () => {
    const f = fixture();
    const pending = f.actions.deploy({ ...request, spawnId: 0x1234, squadId: 7 });
    expect(f.sendC0).toHaveBeenCalledWith(Uint8Array.of(0x10, 0x12, 0x34, 3, 12, 7));
    f.c0.emit(refused);
    await expect(pending).resolves.toMatchObject({ ok: false, reason: 21, reasonKey: 'ui.deploy.err.pointBlocked' });
    const retry = f.actions.deploy(request);
    f.c0.emit(accepted);
    await expect(retry).resolves.toMatchObject({ ok: true });
    expect(f.free).toHaveBeenCalledTimes(2);
    f.actions.dispose();
  });

  it('does not infer acceptance from a zero reason', () => {
    const f = fixture();
    f.codec.decodeServerC0.mockReturnValueOnce({ kind: 0x11, ok: false, spawnTick: 0, reason: 0, free: f.free });
    expect(decodeDeployResult(Uint8Array.of(0x11, 0, 0, 0, 0, 0, 0), f.codec)).toMatchObject({ ok: false, reason: 0 });
    f.actions.dispose();
  });

  it('quarantines deploy after timeout since replies cannot be correlated', async () => {
    vi.useFakeTimers();
    const f = fixture(50);
    const pending = expect(f.actions.deploy(request)).rejects.toThrow(/No deploy result/);
    await vi.advanceTimersByTimeAsync(50);
    await pending;
    f.c0.emit(accepted);
    await expect(f.actions.deploy(request)).rejects.toThrow(/outcome is unknown/);
    expect(f.sendC0).toHaveBeenCalledTimes(1);
    f.actions.dispose();
  });

  it('cleans up on transport close and rejects requests after disposal', async () => {
    const f = fixture();
    const pending = expect(f.actions.deploy(request)).rejects.toThrow(/closed/);
    f.close.emit('lost');
    await pending;
    f.c0.emit(accepted);
    expect(f.codec.decodeServerC0).not.toHaveBeenCalled();
    expect(() => f.actions.leaveSquad()).toThrow(/closed/);
    f.actions.dispose();
  });

  it('uses the server create kind codes and does not send invalid input', async () => {
    const f = fixture();
    f.actions.createSquad('infantry', 'Alpha');
    f.actions.createSquad('armor_crew', 'Armor-1');
    f.actions.createSquad('recon', 'Recon-1');
    f.actions.joinSquad(0x34);
    f.actions.leaveSquad();
    f.actions.lockSquad(true);
    f.actions.lockSquad(false);
    expect(f.sendC0.mock.calls.map(([bytes]) => [...bytes])).toEqual([
      [0x12, 1, 0, 0, 5, 65, 108, 112, 104, 97],
      [0x12, 1, 1, 0, 7, 65, 114, 109, 111, 114, 45, 49],
      [0x12, 1, 2, 0, 7, 82, 101, 99, 111, 110, 45, 49],
      [0x12, 2, 0x34, 0, 0], [0x12, 3, 0, 0, 0], [0x12, 7, 0, 0, 0], [0x12, 8, 0, 0, 0],
    ]);
    expect(() => f.actions.createSquad('infantry', 'Free text')).toThrow(/preset/);
    expect(() => f.actions.joinSquad(256)).toThrow(/Invalid squad/);
    await expect(f.actions.deploy({ ...request, role: 256 })).rejects.toThrow(/Invalid role/);
    expect(f.sendC0).toHaveBeenCalledTimes(7);
    f.actions.dispose();
  });
});

it('exposes marker placement as unsupported (no optimistic marker)', () => {
  expect(markerActions.supported).toBe(false);
  expect(markerActions.place({ kind: 'attention', pos: [10, 10] })).toEqual({ ok: false, reason: 'unsupported' });
});
