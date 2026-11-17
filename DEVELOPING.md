# Developing Frontline

## Prerequisites

- Rust 1.97.0 (pinned in `rust-toolchain.toml`; rustup installs it plus `rustfmt`, `clippy` and the `wasm32-unknown-unknown` target automatically)
- Node >= 22.12 (developed on 22.23) and pnpm 10 (`packageManager` in root `package.json`)
- Python >= 3.11 (map pipeline, from M0-009)

## Rust

```sh
cargo build --workspace
cargo test --workspace
cargo fmt --check
cargo ci-lint      # = clippy --workspace --all-targets -- -D warnings
```

Crate dependency edges are fixed by ARCHITECTURE §4.2/§4.5; edges not listed there are forbidden.
The workspace forbids `unsafe` (`sim-wasm` alone relaxes it for wasm-bindgen glue).

## Client

```sh
pnpm install
pnpm -C client dev        # Vite dev server
pnpm -C client build      # tsc --noEmit + vite build -> client/dist
pnpm lint | pnpm typecheck | pnpm test
```

`@babylonjs/core` is pinned to an exact 8.x version (ARCHITECTURE §5.1); upgrade deliberately.

## Layout check

`node tools/ci/check-layout.mjs` verifies the ARCHITECTURE §4.1 directory layout.

## CI checks (M0-002)

Workflows live in `.github/workflows/` (`lint`, `test-rust`, `test-ts`, `wasm-build`, `data-validate`).
Run the repo-specific gates locally:

```sh
node tools/ci/check-deps.mjs          # §4.5 crate dependency matrix (data: tools/ci/dep-matrix.mjs)
node tools/ci/check-sim-purity.mjs    # §4.3.2 banned APIs in crates/sim/src; waive with `// purity-allow: <why>`
pnpm ci:test                          # node --test for the scripts above
cargo deny check                      # licenses (list pending H-037), bans, advisories, sources
```

## Map data (M0-009)

`data/maps/m0/{meta,spawns,manifest}.json` are tracked; the binaries (`heightmap_*.r16`, `chunks/*.bin`,
about 2.4 MB) are git-ignored and **deterministically regenerated** (needed by M0-010 sim collision and
M0-023 client terrain; run before their tests on a fresh checkout):

```sh
PYTHONPATH=tools/map-pipeline python3 -m map_pipeline m0 --seed 1 --out data/maps/m0   # ~1 s, numpy only
PYTHONPATH=tools/map-pipeline python3 -m map_pipeline validate data/maps/m0            # checks manifest hashes too
(cd tools/map-pipeline && pip install pytest numpy && python3 -m pytest -q)
```

## Directories

See `README.md`. `assets/incoming/` is created manually and never committed; `assets/processed/`
and `docs/measurements/raw/` are also gitignored.

## Task workflow

Work is organised as task cards in `tasks/` - see `tasks/README.md` for rules and the definition
of done, and `tasks/STATUS.md` for progress.
