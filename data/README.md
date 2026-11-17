# data

JSON game data. **The numeric single source of truth** (ARCHITECTURE §4.4, GAMEDESIGN §0.2).

- `crates/sim` embeds these files at build time (`build.rs`) and computes `DATA_HASH`.
- The client never reads this directory for logic; it queries `GameData` through `sim-wasm`
  (display fields only).
- Schema and validation tooling arrive with M0-004.
