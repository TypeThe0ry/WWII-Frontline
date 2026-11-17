import { describe, expect, it } from 'vitest';
import { assertLogicalId, ExitCode, GateError, isValidLogicalId } from '../src/index.ts';

describe('naming (ARCHITECTURE §11.2)', () => {
  it.each([
    'veh_us_m4a1_sherman',
    'wpn_ger_kar98k',
    'prop_building_farmhouse_01',
    'tex_terrain_grass_a',
    'veh_us_m4a1_sherman@lod0',
    'tex_terrain_grass_a@512',
    'chr_us_rifleman',
    'snd_wpn_garand_fire',
    'ui_icon_grenade',
    'vfx_muzzle_flash_a',
    'bld_church_01',
    'veg_hedgerow_a',
  ])('accepts %s', (id) => expect(isValidLogicalId(id)).toBe(true));

  it.each([
    'Veh_us_sherman',
    'veh-us-sherman',
    'sherman',
    'veh_',
    'veh',
    'weapon_kar98k',
    'wpn__kar98k',
    'wpn_kar98k_',
    'wpn_Kar98k',
    'wpn_kar98k.glb',
    'wpn kar98k',
    '',
    'wpn_kar98k@lodx',
  ])('rejects %j', (id) => expect(isValidLogicalId(id)).toBe(false));

  it('assertLogicalId throws exit code 7', () => {
    try {
      assertLogicalId('Bad Name');
      throw new Error('no throw');
    } catch (e) {
      expect(e).toBeInstanceOf(GateError);
      expect((e as GateError).code).toBe(ExitCode.Naming);
    }
  });
});
