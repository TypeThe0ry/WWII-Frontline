import { describe, expect, it } from 'vitest';
import { fallbackPassed, type FallbackResult, type Matrix, median, probePassed, type ProbeResult, renderTable } from './fixtures';

const ok = (o: Partial<ProbeResult> = {}): ProbeResult => ({
  browser: 'chromium', version: '1', transport: 'wt', supported: true, serverCertificateHashesWorked: true, connected: true,
  usedTransport: 'wt', snapshotsReceived: 200, disconnects: 0, rttMsMedian: 1.5, ...o,
});

describe('interop matrix', () => {
  it('median', () => {
    expect(median([])).toBeUndefined();
    expect(median([3, 1, 2])).toBe(2);
    expect(median([4, 1, 3, 2])).toBe(2.5);
  });

  it('probe pass criteria: handshake, >= 150 snapshots, no disconnect, requested transport really used', () => {
    expect(probePassed(ok())).toBe(true);
    expect(probePassed(ok({ snapshotsReceived: 150 }))).toBe(true);
    expect(probePassed(ok({ snapshotsReceived: 149 }))).toBe(false);
    expect(probePassed(ok({ disconnects: 1 }))).toBe(false);
    expect(probePassed(ok({ connected: false }))).toBe(false);
    expect(probePassed(ok({ usedTransport: 'ws' }))).toBe(false); // silent fallback is not a WT pass
  });

  it('fallback criteria: ws and <= 5000 ms', () => {
    const f = (o: Partial<FallbackResult>): FallbackResult => ({ browser: 'chromium', version: '1', connected: true, transport: 'ws', connectMs: 100, ...o });
    expect(fallbackPassed(f({}))).toBe(true);
    expect(fallbackPassed(f({ connectMs: 5000 }))).toBe(true);
    expect(fallbackPassed(f({ connectMs: 5001 }))).toBe(false);
    expect(fallbackPassed(f({ transport: 'wt' }))).toBe(false);
    expect(fallbackPassed(f({ connected: false }))).toBe(false);
  });

  it('table has no empty cell, marks untested browsers with their reason and escapes pipes', () => {
    const m: Matrix = {
      generatedAt: 'x', note: '',
      probes: [ok(), ok({ transport: 'ws', usedTransport: 'ws' }), ok({ browser: 'firefox', transport: 'wt', connected: false, error: 'a | b\nc', usedTransport: undefined, snapshotsReceived: 0 })],
      fallbackNoWebTransport: [{ browser: 'chromium', version: '1', connected: true, transport: 'ws', connectMs: 123.4 }],
      fallbackWtPortClosed: [{ browser: 'chromium', version: '1', connected: true, transport: 'ws', connectMs: 50 }],
      untested: [{ browser: 'webkit', reason: 'not installed' }],
    };
    const t = renderTable(m).split('\n');
    expect(t).toHaveLength(5);
    for (const row of t) {
      const cells = row.split('|').slice(1, -1).map((c) => c.trim());
      expect(cells).toHaveLength(6);
      for (const c of cells) expect(c).not.toBe('');
    }
    expect(t[2]).toContain('通过（200 快照/10 s');
    expect(t[2]).toContain('123 ms');
    expect(t[3]).toContain('失败：a / b c');
    expect(t[4]).toContain('未验证（not installed）');
  });
});
