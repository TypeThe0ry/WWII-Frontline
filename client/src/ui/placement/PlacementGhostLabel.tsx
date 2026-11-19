import { ghostStyle } from '../../game/placement';
import type { PlacementResult } from '../../game/placement';
import { t } from '../i18n';

const icons = { check: '✓', warn: '⚠', minus: '−' } as const;

export interface PlacementGhostLabelProps {
  preview: PlacementResult;
  nearbySupplies?: number;
  capacity?: { used: number; limit: number };
  pending?: boolean;
  translate?: (key: string) => string;
}

/** Plain DOM label. The scene owner projects/attaches it and applies the shared visual tokens. */
export function PlacementGhostLabel({ preview, nearbySupplies, capacity, pending, translate = t }: PlacementGhostLabelProps) {
  const { color, pattern, icon } = ghostStyle(preview);
  const status = pending ? 'Placing…' : preview.legal ? 'Valid placement' : 'Invalid placement';
  const reason = preview.reasonKey && translate(preview.reasonKey);
  return (
    <div role="status" data-placement-color={color} data-placement-pattern={pattern} data-placement-icon={icon}>
      <span aria-hidden="true">{icons[icon]}</span>{' '}{status}
      {reason && <div>{reason}</div>}
      {preview.supplyCost !== undefined && <div>Cost: {preview.supplyCost} Supplies</div>}
      {nearbySupplies !== undefined && <div>Nearby Supplies: {nearbySupplies}</div>}
      {capacity && <div>Placed: {capacity.used}/{capacity.limit}</div>}
      {preview.remaining !== undefined && <div>Remaining: {preview.remaining}</div>}
    </div>
  );
}
