import { describe, expect, it } from 'vitest';
import { parseNetConfig, wsUrl } from './config';

const page = (hostname: string, host = hostname, protocol = 'http:') => ({ hostname, host, protocol });

describe('parseNetConfig', () => {
  it('keeps ?server=host:port behaviour', () => {
    const c = parseNetConfig('?server=10.0.0.5:7100', page('example.com'));
    expect(c.sameOriginHost).toBeUndefined();
    expect(wsUrl(c)).toBe('ws://10.0.0.5:7100/');
    expect(c.wtPort).toBe(7101);
    expect(c.transport).toBe('auto');
  });

  it('defaults to page hostname:7000 on localhost', () => {
    for (const h of ['localhost', '127.0.0.1']) {
      const c = parseNetConfig('', page(h, `${h}:5173`));
      expect(wsUrl(c)).toBe(`ws://${h}:7000/`);
      expect(c.transport).toBe('auto');
    }
  });

  it('?server=same uses the page origin + /game-ws and forces ws', () => {
    const c = parseNetConfig('?server=same', page('127.0.0.1', '127.0.0.1:5173'));
    expect(wsUrl(c)).toBe('ws://127.0.0.1:5173/game-ws');
    expect(c.transport).toBe('ws');
    const s = parseNetConfig('?server=same&transport=auto', page('x.example.com', 'x.example.com', 'https:'));
    expect(wsUrl(s)).toBe('wss://x.example.com/game-ws');
    expect(s.transport).toBe('ws');
  });

  it('non-local page without ?server auto-selects same-origin', () => {
    const c = parseNetConfig('?scene=empty', page('preview.example.net', 'preview.example.net:8443', 'https:'));
    expect(wsUrl(c)).toBe('wss://preview.example.net:8443/game-ws');
    expect(c.transport).toBe('ws');
  });

  it('explicit ?server=host:port wins on a non-local page', () => {
    const c = parseNetConfig('?server=game.example.com:7000', page('preview.example.net'));
    expect(wsUrl(c)).toBe('ws://game.example.com:7000/');
    expect(c.transport).toBe('auto');
  });

  it('no location and no server falls back to localhost:7000', () => {
    expect(wsUrl(parseNetConfig(''))).toBe('ws://localhost:7000/');
  });
});
