/** UI_SPEC §6.1–6.3 scale / breakpoint / safe-area math (pure). */
export const UI_BASE_W = 1920;
export const UI_BASE_H = 1080;

export function uiScale(vw: number, vh: number, userScale = 1): number {
  const raw = Math.min(vw / UI_BASE_W, vh / UI_BASE_H);
  const clamped = Math.min(2.0, Math.max(0.667, raw));
  const s = clamped * userScale;
  // Large (>= 2560 wide) caps HUD at 1.5x (§6.2).
  return vw >= 2560 ? Math.min(s, 1.5) : s;
}

export type Breakpoint = 'compact' | 'standard' | 'large' | 'ultrawide';

export function breakpoint(vw: number, vh: number): Breakpoint {
  if (vw / vh >= 2.1) return 'ultrawide';
  if (vh < 800 || vw < 1500) return 'compact';
  if (vw >= 2560) return 'large';
  return 'standard';
}

/** Font floor (§6.1): below 11 px (12 px for CJK) the size is clamped up. */
export function scaledFontPx(basePx: number, scale: number, cjk: boolean): number {
  return Math.max(cjk ? 12 : 11, basePx * scale);
}

/** HUD safe box in CSS px (§6.3), `marginPct` is the user slider 0–10 (% added to the default). */
export function safeBox(vw: number, vh: number, scale: number, marginPct = 0): { x: number; y: number } {
  const extra = marginPct / 100;
  return {
    x: Math.max(48 * scale, 0.025 * vw) + extra * vw,
    y: Math.max(32 * scale, 0.03 * vh) + extra * vh,
  };
}
