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

## Directories

See `README.md`. `assets/incoming/` is created manually and never committed; `assets/processed/`
and `docs/measurements/raw/` are also gitignored.

## Task workflow

Work is organised as task cards in `tasks/` - see `tasks/README.md` for rules and the definition
of done, and `tasks/STATUS.md` for progress.
