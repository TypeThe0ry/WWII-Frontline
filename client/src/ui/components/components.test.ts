import { describe, expect, it } from 'vitest';
import { wrapTarget } from './focus';
import { nextTab } from './tabs';
import { listWindow } from './virtual-list';

describe('tab activation', () => {
  const items = [
    { id: 'first', labelKey: 'a' }, { id: 'disabled', labelKey: 'b', disabled: true }, { id: 'last', labelKey: 'c' },
  ];
  it.each([
    ['first', 'ArrowRight', 'last'], ['first', 'ArrowLeft', 'last'], ['last', 'ArrowRight', 'first'],
    ['last', 'Home', 'first'], ['first', 'End', 'last'], ['first', 'Tab', undefined],
  ])('%s + %s targets %s', (from, key, expected) => expect(nextTab(items, from!, key!)).toBe(expected));
  it('cannot activate an all-disabled tablist', () => {
    expect(nextTab(items.map(i => ({ ...i, disabled: true })), 'first', 'End')).toBeUndefined();
  });
});

describe('focus trap boundaries', () => {
  const stops = ['first', 'middle', 'last'];
  it('wraps at both ends, without intercepting ordinary Tab navigation', () => {
    expect(wrapTarget(stops, 'last', false, 'root')).toBe('first');
    expect(wrapTarget(stops, 'first', true, 'root')).toBe('last');
    for (const reverse of [false, true]) expect(wrapTarget(stops, 'middle', reverse, 'root')).toBeUndefined();
  });
  it('recovers focus when the previous target was removed or disabled', () => {
    expect(wrapTarget(stops, null, false, 'root')).toBe('first');
    expect(wrapTarget(stops, 'disabled', true, 'root')).toBe('last');
  });
  it('keeps an empty or single-control trap reachable in either direction', () => {
    for (const reverse of [false, true]) {
      expect(wrapTarget([], null, reverse, 'root')).toBe('root');
      expect(wrapTarget(['only'], 'only', reverse, 'root')).toBe('only');
    }
  });
});

describe('fixed-row list', () => {
  it('fills the last viewport immediately after items shrink, even with zero overscan', () => {
    expect(listWindow(12, 10, 50, 9000, 0)).toEqual({ start: 7, end: 12, padTop: 70, padBottom: 0 });
  });
  it('bounds DOM work independently of item count and preserves total scroll height', () => {
    const { start, end, padTop, padBottom } = listWindow(100000, 20, 93, 2011, 2);
    expect(start).toBe(98);
    expect(end - start).toBeLessThanOrEqual(10);
    expect(padTop + (end - start) * 20 + padBottom).toBe(2000000);
  });
  it('handles an empty or shorter-than-viewport list', () => {
    expect(listWindow(0, 20, 94, 9, 2)).toEqual({ start: 0, end: 0, padTop: 0, padBottom: 0 });
    expect(listWindow(2, 20, 94, 9, -2)).toEqual({ start: 0, end: 2, padTop: 0, padBottom: 0 });
  });
});
