# data

JSON game data. **The numeric single source of truth** (ARCHITECTURE §4.4, GAMEDESIGN §0.2).

- `crates/sim` embeds these files at build time (`build.rs`) and computes `DATA_HASH`.
- The client never reads this directory for logic; it queries `GameData` through `sim-wasm`
  (display fields only).
- Schemas: `schemas/{weapon,role,loadout,vehicle,map_meta,spawns,rules,bots}.schema.json` (JSON Schema 2020-12).
  Test fixtures: `schemas/fixtures/valid/*.json`, `schemas/fixtures/invalid/*.json`.
- Not yet covered by a schema (hashed only): `maps/*/map.json` and the other map files. Their schemas arrive with their tasks.

## Layout and file kinds

| Path | Kind | Notes |
|---|---|---|
| `weapons/{id}.json` | weapon | `id` must equal the file name |
| `roles/{id}.json` | role | |
| `loadouts/{id}.json` | loadout | |
| `vehicles/{id}.json` | vehicle | no instances before M3 |
| `rules/{name}.json` | rules | respawn timing (`respawn.json`, GAMEDESIGN §10.6); no `id`, `_meta` required |
| `rules/bots.json` | bots | bot difficulty tiers + behaviour tuning (GAMEDESIGN §20.6, M0-026); schema `schemas/bots.schema.json`; read by `crates/bots` through `GameData::rules.bots` |
| `maps/{map}/meta.json`, `maps/{map}/spawns.json` | map_meta, spawns | MAPS §6.2; `_meta` optional here |
| `schemas/**` | schemas and fixtures | never part of `DATA_HASH` |

Fixtures are flat files named `<kind>.<name>.json`. Every invalid fixture carries a top-level
`_expect_error` (a substring the validator output must contain); the valid set is the context in
which references of an invalid fixture are resolved.

## Conventions

- `id`: stable snake_case slug. IDs follow GAMEDESIGN §0.3 (`us_m1_garand`, `ger_kar98k`, loadout
  `us_rifleman_1`); ARCHITECTURE §8.5.1's `m1_garand` example is superseded (PLAN_ISSUES).
- Display names are i18n keys (`name_key`), never literal text.
- **Every numeric field carries a unit suffix**: `_m _mm _ms _s _mps _mps2 _rpm _rpm_practical _kg
  _deg _deg_s _hp _per_m _per_s _nm _n_per_m _ns_per_m _mult`. Dimensionless counts/scores are an
  explicit list in `tools/data-validate/rules/units.mjs` (`ammo_mags`, `pen_power`, ...). Children
  of a unit-suffixed container inherit it (`armor_mm.front`, `part_mult.head`). Non-SI suffixes
  (`_kph`, `_ft`, `_lb`, ...) are rejected with a specific message.
- `historical` blocks hold public-record parameters and need `_source` (or mark a single value
  `"未核实"`); `gameplay` blocks hold tunable game values. Once `status` is `reviewed`/`measured`,
  rule `hist-source-required` (M0-031) demands a structured `_source` array of
  `{field, value, unit, source_id, title, locator, url, accessed, note}` records, one per
  `historical` field, or an entry in `_meta.unverified[]` (`"historical.<field>"`).
  `_meta.deviation_reason{path: text}` explains gameplay values that deviate from history.
- `_meta.status ∈ {initial, reviewed, measured}` (R8: `initial` until reviewed);
  `_meta.tunable` lists dotted paths of `【待调参】` fields; every path must exist.
- Adding a field: **edit the schema first** (optional fields only), then the typed struct in
  `crates/sim/src/data_defs.rs`, and log a "细化" entry in `docs/PLAN_ISSUES.md`.

## Who validates what

| Layer | Tool | Checks |
|---|---|---|
| Full validation (CI `data-validate`, <2 min) | `node tools/data-validate/index.mjs [--root data] [--json]` | JSON Schema, unit suffixes, references, `_meta`/tunable, ranges. One error per line `path:jsonpointer: message`, exit 1 on any error. `--fixtures` runs the fixture suite. |
| Build time | `crates/sim/build.rs` | Parses every weapon/role/loadout/vehicle file and `rules/respawn.json` into `deny_unknown_fields` typed structs (`src/data_defs.rs`); a missing/unknown field fails `cargo build`. Computes `DATA_HASH`. |

Tests: `pnpm data:test` (rules, fixtures, CLI per fixture), `cargo test -p frontline-sim data_`.

## DATA_HASH

All `.json` under `data/` except `schemas/`, sorted by relative path; each file parsed and
re-serialised canonically (keys sorted, no whitespace); SHA-256 over `path 0x00 canonical 0x0A`
per file; first 8 bytes little-endian as `u64` (`frontline_sim::data::DATA_HASH`). Re-indenting a
file never changes it; any value or path change does (ARCHITECTURE §18.5: data change = version
change). Note `1` and `1.0` are different values.

## Field-name mapping (docs → data)

| Source | Docs wrote | Data uses |
|---|---|---|
| ARCH §8.5.1 | `damage_torso/head/limb` (absolute) | `damage_torso_hp` + `part_mult{head,neck,torso,limb}` (GAMEDESIGN §6.6) |
| ARCH §8.5.1 | `drag_k_per_m`; GAMEDESIGN §6.4 `k_drag` | `ballistics.drag_k_per_m` |
| ARCH §8.5.1 | `spread_hip_deg`, `reload_ms`, `reload_empty_ms` | `spread{hip_deg,ads_deg}`, `reload{tactical_ms,empty_ms,per_round_ms?}` |
| GAMEDESIGN §6.1 | `rpm_cyclic`, `cycle_time_s` | `fire{mode, cap_rpm \| bolt_cycle_ms}` |
| GAMEDESIGN §6.1 | `ads_time_s`, `recov_deg_s` | `ads_ms`, `recoil.recover_deg_s` |
| ARCH §8.5.2 | `health` | `health_hp` |
| ARCH §8.5.4 | `max_speed_kph`, `hull_health`, `armor_mm{...}`, `stiffness`, `damping`, `max_torque_nm`, `turret_traverse_dps` | `max_speed_mps`, `hull_health_hp`, `armor_mm` (unchanged), `stiffness_n_per_m`, `damping_ns_per_m`, `max_torque_nm`, `turret_traverse_deg_s` |
| ARCH §8.5.3 | `gadget_1/2 {id,count}`, `melee` | same; ids refer to weapon ids (grenades etc. are weapons per GAMEDESIGN §6.1) |
