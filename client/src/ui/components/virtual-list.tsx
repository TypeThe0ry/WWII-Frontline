import type { ComponentChildren } from 'preact';
import { useState } from 'preact/hooks';
import { virtualWindow } from '../virtual-list';

export function listWindow(count: number, rowHeight: number, height: number, scrollTop: number, overscan: number) {
  // A list can shrink before the browser clamps scrollTop and emits its next scroll event.
  return virtualWindow(count, rowHeight, height, Math.min(scrollTop, count * rowHeight - height), Math.max(0, Math.floor(overscan)));
}

export interface VirtualListProps<T> {
  label: string;
  items: readonly T[];
  rowHeight: number;
  height: number;
  renderRow(item: T, index: number): ComponentChildren;
  itemKey?(item: T, index: number): string | number;
  overscan?: number;
}

export function VirtualList<T>({ label, items, rowHeight, height, renderRow, itemKey, overscan = 3 }: VirtualListProps<T>) {
  const [top, setTop] = useState(0);
  const { start, end, padTop, padBottom } = listWindow(items.length, rowHeight, height, top, overscan);
  return <div role="list" aria-label={label} tabIndex={0} style={{ height, overflowY: 'auto' }}
    onScroll={e => setTop(e.currentTarget.scrollTop)}>
    <div role="presentation" style={{ paddingTop: padTop, paddingBottom: padBottom }}>
      {items.slice(start, end).map((item, i) => <div role="listitem" key={itemKey?.(item, start + i) ?? start + i}
        aria-posinset={start + i + 1} aria-setsize={items.length} style={{ height: rowHeight }}>
        {renderRow(item, start + i)}
      </div>)}
    </div>
  </div>;
}
