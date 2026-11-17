import { ExitCode, GateError } from './errors.ts';

/** ARCHITECTURE §11.2 category prefixes. */
export const NAME_PREFIXES = ['veh', 'wpn', 'chr', 'prop', 'bld', 'veg', 'tex', 'snd', 'vfx', 'ui'] as const;

const LOGICAL_ID = new RegExp(`^(?:${NAME_PREFIXES.join('|')})(?:_[a-z0-9]+)+(?:@(?:lod)?[0-9]+)?$`);

/** Returns a human-readable problem, or `undefined` if the logical ID is valid (`veh_us_m4a1_sherman`, `…@lod0`). */
export function namingProblem(id: string): string | undefined {
  if (id === '') return 'empty';
  if (id !== id.toLowerCase()) return 'must be all lower case';
  if (/[^a-z0-9_@]/.test(id)) return 'only a-z, 0-9 and "_" are allowed (plus an optional "@lodN" or "@512" suffix)';
  if (/__/.test(id) || id.startsWith('_') || id.endsWith('_')) return 'empty snake_case segment';
  if (!NAME_PREFIXES.some((p) => id.startsWith(p + '_'))) return `prefix must be one of ${NAME_PREFIXES.map((p) => p + '_').join(' ')}`;
  if (!LOGICAL_ID.test(id)) return 'does not match {prefix}_{snake_case_name}[@lodN|@N]';
  return undefined;
}

export function isValidLogicalId(id: string): boolean {
  return namingProblem(id) === undefined;
}

/** Exit 7 on a bad logical output ID (ARCHITECTURE §11.2). */
export function assertLogicalId(id: string): void {
  const p = namingProblem(id);
  if (p) throw new GateError(ExitCode.Naming, `logical ID "${id}": ${p} (ARCHITECTURE §11.2)`);
}
