import { useState } from 'preact/hooks';
import type { TargetedKeyboardEvent } from 'preact';
import { markerActions } from '../../game/marker-actions';
import { MapCanvas } from './MapCanvas';
import { availableLayers, MAP_LAYERS } from './map-renderer';
import type { LayerFlags, MapData, MapLayer } from './map-renderer';
import { unitStaleness } from './map-redraw';

const STORAGE_KEY = 'frontline.tacticalMap.layers';
const LABELS: Record<MapLayer, string> = {
  base: 'Base', grid: 'Grid', objectives: 'Objectives', spawnPoints: 'Spawn points', units: 'Friendly units', markers: 'Markers',
};

function readLayers(): LayerFlags {
  const layers = availableLayers('full');
  try {
    const saved: Partial<LayerFlags> | null = JSON.parse(localStorage.getItem(STORAGE_KEY) ?? 'null');
    if (saved) for (const layer of MAP_LAYERS) if (typeof saved[layer] === 'boolean') layers[layer] = saved[layer];
  } catch { /* Storage and malformed preferences must not prevent opening the map. */ }
  return layers;
}

export interface TacticalMapProps {
  data: MapData;
  onClose: () => void;
}

/** Mount only while open. The owner handles routing and pointer lock; this component never pauses play. */
export function TacticalMap({ data, onClose }: TacticalMapProps) {
  const [layers, setLayers] = useState(readLayers);
  function toggle(layer: MapLayer) {
    const next = { ...layers, [layer]: !layers[layer] };
    setLayers(next);
    try { localStorage.setItem(STORAGE_KEY, JSON.stringify(next)); } catch { /* preferences are optional */ }
  }
  function key(e: TargetedKeyboardEvent<HTMLElement>) {
    if (e.altKey || e.ctrlKey || e.metaKey || e.defaultPrevented) return;
    if (e.key === 'Escape' || e.key.toLowerCase() === 'm') {
      e.preventDefault(); e.stopPropagation();
      if (!e.repeat) onClose();
    } else if (/^[1-6]$/.test(e.key)) {
      const layer = MAP_LAYERS[Number(e.key) - 1]!;
      e.preventDefault(); e.stopPropagation();
      if (!e.repeat) toggle(layer);
    }
  }
  const self = data.units?.find((unit) => unit.relation === 'self');
  const nowS = data.nowS;
  let oldest = -Infinity;
  if (layers.units && nowS !== undefined) for (const unit of data.units ?? []) {
    if (unit.relation === 'self' || unit.updatedAtS === undefined) continue;
    const age = unitStaleness(unit.updatedAtS, nowS);
    if (age.stale) oldest = Math.max(oldest, age.agoS);
  }
  return <section aria-label="Tactical map" onKeyDown={key}>
    <h2>Tactical map</h2>
    <button type="button" onClick={onClose}>Close map</button>
    <MapCanvas data={data} mode="full" layers={layers} center={self} focusOnMount />
    <fieldset>
      <legend>Map layers</legend>
      {MAP_LAYERS.map((layer) => <label key={layer}>
        <input type="checkbox" checked={layers[layer]} onChange={() => toggle(layer)} /> {LABELS[layer]}
      </label>)}
    </fieldset>
    {Number.isFinite(oldest) && <p>Friendly unit data is up to {oldest} s old.</p>}
    <p>Shift-drag to measure distance and bearing. The match continues while the map is open.</p>
    <button type="button" disabled title={markerActions.reason}>Place marker</button>
    <p>{markerActions.reason}</p>
  </section>;
}
