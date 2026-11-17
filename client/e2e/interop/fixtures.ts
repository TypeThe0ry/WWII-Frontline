// Pure helpers for the browser interop matrix (M0-028): cell judgement, summary table, JSON <-> table consistency.
// No imports with side effects: also loaded by scripts/check-interop-table.mjs (node --experimental-strip-types).

export type BrowserName = 'chromium' | 'firefox' | 'webkit';
export const BROWSERS: readonly BrowserName[] = ['chromium', 'firefox', 'webkit'];
export const TRANSPORTS = ['wt', 'ws'] as const;
export type TransportName = (typeof TRANSPORTS)[number];

/** Pass criteria (task M0-028): handshake ok, >= 150 snapshots in the 10 s run (20 Hz ideal 200, 25 % tolerance), 0 disconnects. */
export const MIN_SNAPSHOTS = 150;
export const PROBE_SECONDS = 10;
export const FALLBACK_LIMIT_MS = 5000;

export interface ProbeResult {
  browser: BrowserName;
  version: string;
  transport: TransportName;
  /** `typeof WebTransport !== 'undefined'` in the page. */
  supported: boolean;
  /** There is no feature-detection API for the option: true when a WT session with a certificate hash was established. */
  serverCertificateHashesWorked: boolean;
  connected: boolean;
  /** Transport the session actually used (a WT cell must not pass over a silent WS fallback). */
  usedTransport?: string;
  snapshotsReceived: number;
  disconnects: number;
  rttMsMedian?: number;
  error?: string;
}

export interface FallbackResult {
  browser: BrowserName;
  version: string;
  /** Original text of what the page reported, or why the cell is empty. */
  connected: boolean;
  transport?: string;
  connectMs?: number;
  fallbackReason?: string;
  error?: string;
}

/** A browser that could not be run on this machine: every cell says why instead of staying empty. */
export interface Untested {
  browser: BrowserName;
  reason: string;
}

export interface Matrix {
  generatedAt: string;
  note: string;
  probes: ProbeResult[];
  /** `delete window.WebTransport` + `?transport=auto`. */
  fallbackNoWebTransport: FallbackResult[];
  /** Chromium: server serves no WebTransport port at all, `auto`. */
  fallbackWtPortClosed: FallbackResult[];
  untested: Untested[];
}

export function median(xs: readonly number[]): number | undefined {
  if (xs.length === 0) return undefined;
  const s = [...xs].sort((a, b) => a - b);
  const m = s.length >> 1;
  return s.length % 2 ? s[m] : (s[m - 1] + s[m]) / 2;
}

export function probePassed(p: ProbeResult): boolean {
  return p.connected && p.usedTransport === p.transport && p.snapshotsReceived >= MIN_SNAPSHOTS && p.disconnects === 0;
}

export function fallbackPassed(f: FallbackResult): boolean {
  return f.connected && f.transport === 'ws' && f.connectMs !== undefined && f.connectMs <= FALLBACK_LIMIT_MS;
}

function oneLine(s: string): string {
  return s.replace(/\s+/g, ' ').replace(/\|/g, '/').trim();
}

export function probeCell(p: ProbeResult | undefined, untested: Untested | undefined): string {
  if (untested) return `未验证（${oneLine(untested.reason)}）`;
  if (!p) return '缺失';
  const rtt = p.rttMsMedian === undefined ? 'n/a' : `${p.rttMsMedian.toFixed(1)} ms`;
  if (probePassed(p)) return `通过（${p.snapshotsReceived} 快照/${PROBE_SECONDS} s，RTT 中位 ${rtt}）`;
  if (!p.supported && p.transport === 'wt') return `失败：无 WebTransport（${p.snapshotsReceived} 快照）`;
  const why = p.error ? oneLine(p.error) : `connected=${String(p.connected)} via ${p.usedTransport ?? '-'}, ${p.snapshotsReceived} 快照, ${p.disconnects} 断线`;
  return `失败：${why}`;
}

export function fallbackCell(f: FallbackResult | undefined, untested: Untested | undefined): string {
  if (untested) return `未验证（${oneLine(untested.reason)}）`;
  if (!f) return '不适用';
  if (fallbackPassed(f)) return `通过（回退 WS，${Math.round(f.connectMs ?? 0)} ms）`;
  return `失败：${oneLine(f.error ?? `transport=${f.transport ?? '-'} ${f.connectMs === undefined ? '' : Math.round(f.connectMs) + ' ms'} ${f.fallbackReason ?? ''}`)}`;
}

/** The rows are always the three browsers and the columns always the same: no empty cell. */
export function renderTable(m: Matrix): string {
  const rows = ['| 浏览器 (Playwright 构建) | WT | WS | 回退（删除 WebTransport） | 回退（WT 端口关闭） | 备注 |', '| --- | --- | --- | --- | --- | --- |'];
  for (const b of BROWSERS) {
    const untested = m.untested.find((u) => u.browser === b);
    const version = m.probes.find((p) => p.browser === b)?.version ?? m.fallbackNoWebTransport.find((p) => p.browser === b)?.version ?? 'n/a';
    const cells = [
      probeCell(m.probes.find((p) => p.browser === b && p.transport === 'wt'), untested),
      probeCell(m.probes.find((p) => p.browser === b && p.transport === 'ws'), untested),
      fallbackCell(m.fallbackNoWebTransport.find((f) => f.browser === b), untested),
      b === 'chromium' ? fallbackCell(m.fallbackWtPortClosed.find((f) => f.browser === b), untested) : '未设计（任务仅要求 Chromium）',
    ];
    const note = b === 'webkit' ? 'WebKit 构建 ≠ Safari 正式版' : b === 'firefox' ? 'Playwright 打补丁的 Firefox 构建' : 'headless Chromium';
    rows.push(`| ${b} ${version} | ${cells.join(' | ')} | ${note} |`);
  }
  return rows.join('\n');
}
