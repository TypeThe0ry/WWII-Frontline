import type { PlacementKind } from '../../game/placement';
import type { SquadToolCommand } from '../../game/placement-actions';
import { t } from '../i18n';

export interface SquadToolPanelProps {
  tools: readonly { kind: PlacementKind; label: string; enabled: boolean; reasonKey?: string }[];
  activeTool?: PlacementKind | null;
  onSelectTool?: (kind: PlacementKind) => void;
  carryingSupply?: boolean;
  ammoCooldownS?: number;
  squad?: {
    locked: boolean;
    canManage: boolean;
    members: readonly { id: number; name: string; officer?: boolean; self?: boolean }[];
  };
  onSquadCommand?: (command: SquadToolCommand) => void;
  translate?: (key: string) => string;
}

/** Render only authoritative squad membership/lock state; callbacks request, not apply, changes. */
export function SquadToolPanel(props: SquadToolPanelProps) {
  const { tools, activeTool, onSelectTool, carryingSupply, ammoCooldownS, squad, onSquadCommand, translate = t } = props;
  const canManage = squad?.canManage && onSquadCommand;
  return (
    <section aria-label="Squad tools">
      <ul>
        {tools.map(({ kind, label, enabled, reasonKey }) => (
          <li key={kind}>
            <button type="button" disabled={!enabled || !onSelectTool} aria-pressed={activeTool === kind} onClick={() => onSelectTool?.(kind)}>{label}</button>
            {!enabled && reasonKey && <span>{' '}{translate(reasonKey)}</span>}
          </li>
        ))}
      </ul>
      {carryingSupply && <p>Carrying Supply Crate</p>}
      {ammoCooldownS !== undefined && ammoCooldownS > 0 && <p>Ammo Crate cooldown: {ammoCooldownS} s</p>}
      {squad && (
        <section aria-label="Squad members">
          <p>{squad.locked ? 'Locked squad' : 'Unlocked squad'}</p>
          {canManage && <button type="button" onClick={() => canManage({ kind: 'lock', locked: !squad.locked })}>{squad.locked ? 'Unlock' : 'Lock'}</button>}
          <ul>
            {squad.members.map((member) => (
              <li key={member.id}>
                {member.name}{member.officer && ' (Officer)'}{member.self && ' (You)'}
                {canManage && !member.self && !member.officer && (
                  <>
                    <button type="button" aria-label={`Kick ${member.name}`} onClick={() => canManage({ kind: 'kick', playerId: member.id })}>Kick</button>
                    <button type="button" aria-label={`Offer leadership to ${member.name}`} onClick={() => canManage({ kind: 'promote', playerId: member.id })}>Offer leadership</button>
                  </>
                )}
              </li>
            ))}
          </ul>
        </section>
      )}
    </section>
  );
}
