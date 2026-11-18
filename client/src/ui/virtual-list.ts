/** Fixed-row-height window computation for `VirtualList`. */
export interface Window {
  start: number;
  end: number; // exclusive
  padTop: number;
  padBottom: number;
}

export function virtualWindow(count: number, rowH: number, viewH: number, scrollTop: number, overscan = 3): Window {
  if (count <= 0 || rowH <= 0) return { start: 0, end: 0, padTop: 0, padBottom: 0 };
  const first = Math.floor(Math.max(0, scrollTop) / rowH);
  const visible = Math.ceil(viewH / rowH) + 1;
  const start = Math.max(0, Math.min(count - 1, first) - overscan);
  const end = Math.min(count, first + visible + overscan);
  return { start, end, padTop: start * rowH, padBottom: (count - end) * rowH };
}
