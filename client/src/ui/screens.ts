import { signal } from '@preact/signals';

export type Screen = 'boot' | 'menu' | 'browser' | 'faction' | 'deploy' | 'hud' | 'map' | 'pause' | 'downed';
export const SCREENS: readonly Screen[] = ['boot', 'menu', 'browser', 'faction', 'deploy', 'hud', 'map', 'pause', 'downed'];
export const uiScreen = signal<Screen>('boot');
