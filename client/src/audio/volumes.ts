/** Bus volume mapping from settings (0..1 sliders → gain; perceptual curve, hard mute at 0). */
export type VolumeBus = 'master' | 'sfx' | 'ui' | 'voice' | 'music';
export const SETTING_TO_BUS: Readonly<Record<string, VolumeBus>> = { 'audio.master': 'master', 'audio.sfx': 'sfx', 'audio.ui': 'ui', 'audio.voice': 'voice', 'audio.music': 'music' };

/** Slider → linear gain with a squared curve (so the lower half still has fine control). */
export const sliderToGain = (v: number): number => {
  const c = Math.min(1, Math.max(0, Number.isFinite(v) ? v : 0));
  return c * c;
};

/** Effective gain of a bus under the master slider; muted when the window is unfocused and "mute on blur" is set. */
export function effectiveGain(bus: Exclude<VolumeBus, 'master'>, sliders: Readonly<Record<VolumeBus, number>>, muteOnBlur: boolean, focused: boolean): number {
  if (muteOnBlur && !focused) return 0;
  return sliderToGain(sliders.master) * sliderToGain(sliders[bus]);
}

/** Context states: nothing may sound before the first user gesture resumes the context. */
export type CtxState = 'suspended' | 'running' | 'closed';
export const canPlay = (state: CtxState, gestureSeen: boolean): boolean => gestureSeen && state === 'running';
