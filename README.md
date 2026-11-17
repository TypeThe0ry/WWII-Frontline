# Frontline

A browser-based, server-authoritative WWII multiplayer combined-arms shooter (Rust simulation shared between server and WASM client, TypeScript + Babylon.js front end).

## Layout

- `client/` - TypeScript + Vite + Babylon.js (`src/{render,ui,audio,input,net,game}`)
- `crates/` - Rust workspace: `protocol`, `sim`, `sim-wasm`, `server`, `gateway`, `bots`, `headless-client`, `balance-sim`
- `tools/` - `asset-pipeline` (Node), `map-pipeline` (Python), `ci` (repo checks)
- `data/` - JSON game data (numeric source of truth)
- `assets/` - `incoming/` (manual, gitignored) and `processed/` (generated, gitignored)
- `docs/` - design and architecture documents; `tasks/` - task cards and `STATUS.md`
- `LICENSES.md` - third-party asset/font license register
- Current progress: [docs/PROGRESS.md](docs/PROGRESS.md) (overview and task log) and [tasks/STATUS.md](tasks/STATUS.md) (per-task status and measurements)

## Build and test

```sh
cargo build --workspace && cargo test --workspace
cargo ci-lint                      # clippy, warnings denied
pnpm install
pnpm -C client dev                 # dev server
pnpm build && pnpm lint && pnpm typecheck && pnpm test
node tools/ci/check-layout.mjs     # repo layout check
```

See [DEVELOPING.md](DEVELOPING.md) for details, `docs/` for the design, and `tasks/README.md` for the task workflow.
