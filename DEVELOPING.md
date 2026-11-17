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

## Terrain in the client (M0-023)

The client fetches `/maps/m0/{meta.json,heightmap_1m.r16,heightmap_far_8m.r16}`. The binaries are git-ignored
(see above), so they are copied into the git-ignored `client/public/maps/m0/` by one repeatable command, which
first runs the map pipeline if `data/maps/m0` has no height maps (needs python3 + numpy):

```sh
pnpm -C client sync-map        # = node client/scripts/sync-map.mjs [mapId]; idempotent
```

`pnpm -C client dev`, `pnpm -C client build` and the Playwright `webServer` call it automatically. Vitest reads
`data/maps/m0/*.r16` directly (the real-data tests skip with the regeneration command if they are missing).
After regenerating the map with a different seed/tool version, refresh the numpy reference used by
`terrain-data.test.ts` with `python3 client/scripts/gen-height-ref.py`.

Open the terrain scene (software or real GPU):

```sh
pnpm -C client dev    # then   http://localhost:5173/?scene=terrain&debug=1
#   &renderer=webgpu|webgl2   &terrain_debug=lod|morph|normal|height|flat   &cam=0|1|2
#   &pos=x,y,z&target=x,y,z   &speed=m_per_frame   (WASD + mouse; click to lock the pointer)
# window.__terrain = { stats(), heightAt(x,z), nodes(), project(x,z), setPose(), setStitch(), setDebug() }
pnpm -C client test:e2e -- terrain.spec.ts            # software WebGL2 pixel checks + WebGPU compile check
SAVE_SHOTS=1 pnpm -C client test:e2e -- terrain.spec.ts   # also refresh docs/measurements/M0-023/*.png
```

## Vegetation in the client (M0-024)

Grey-box vegetation (procedural placeholders, no third-party assets) on top of the terrain: `?scene=vegetation` = terrain + vegetation
(`?scene=terrain` stays terrain only). Instance tables are scattered at start-up per 64 m chunk (`render/vegetation/scatter.ts`, 6 B
per entry, layout in PI-020); M1 replaces the source with baked tables, the table bytes are the interface.

```sh
pnpm -C client dev    # then   http://localhost:5173/?scene=vegetation&debug=1
#   &quality=high|low   &impostor=lowres (64 px views, faster bake)   &veg_seed=n   &veg_budget=ms|inf   &renderer=webgpu|webgl2
#   &cam=0|1|2 / pos= / target= as for the terrain scene
# window.__vegetation = { veg, perf(), loadMs, ... }; window.__perf.dump().vegetation = { drawCalls, instances, tris }
# After teleporting the camera call veg.settle(camera) (update() spreads re-bucketing over frames by a 0.45 ms budget).
pnpm -C client test:e2e -- vegetation.spec.ts                 # software WebGL2 pixel checks + WebGPU compile/orientation check
SAVE_SHOTS=1 pnpm -C client test:e2e -- vegetation.spec.ts    # also refresh docs/measurements/M0-024/*.png
```

`pnpm -C client sync-map` also copies `spawns.json` (keep-out discs around spawn points).

## Directories

See `README.md`. `assets/incoming/` is created manually and never committed; `assets/processed/`
and `docs/measurements/raw/` are also gitignored.

## Task workflow

Work is organised as task cards in `tasks/` - see `tasks/README.md` for rules and the definition
of done, and `tasks/STATUS.md` for progress.

## sim-wasm (M0-014)

Pinned tools: wasm-bindgen 0.2.130 (`cargo install wasm-bindgen-cli --version =0.2.130 --locked`), wasm-pack 0.15.1, binaryen `version_132` (`wasm-opt`).

```sh
wasm-pack build crates/sim-wasm --target web --profile wasm-release   # -> crates/sim-wasm/pkg (git-ignored)
node tools/ci/check-wasm-size.mjs                                     # wasm-opt -O3 in place; gate 1.2 MB / br 400 KB
cargo run --release -p frontline-sim-wasm --example trace             # native reference trace (needs terrain)
node crates/sim-wasm/tests/node-smoke.mjs                             # + native-vs-wasm.mjs, bench.mjs
cargo run -p frontline-sim-wasm --example gen_golden                  # regenerate tests/golden after protocol/data changes
```

## WebTransport development certificate (M0-017)

The server serves WebTransport (HTTP/3 over QUIC, UDP) on `--wt-port` (default 7001) only when it has a
certificate; without one it logs `wt_disabled` and serves WebSocket only. Browsers require a trusted
certificate, or - for development - a self-signed ECDSA P-256 certificate valid for at most 14 days that the
page pins with `serverCertificateHashes` (support in Firefox/Safari is unverified, see M0-028). A trusted
certificate / domain is the manual item H-017.

```sh
tools/dev/gen-dev-cert.sh                  # = cargo run -p frontline-server --bin gen_dev_cert; writes ./dev-cert/ (gitignored)
cargo run -p frontline-server -- --cert dev-cert/dev-cert.pem --key dev-cert/dev-key.pem
cargo run -p frontline-server -- --dev-cert    # alternative: throw-away in-memory certificate; its hash is in the `wt_listening` log line
```

`gen_dev_cert` (13 days, SAN `localhost`, `127.0.0.1`, `::1`) prints the SHA-256 fingerprint as hex and base64 and
stores it in `dev-cert/dev-cert.sha256`. The client receives the hex fingerprint as a query parameter,
`http://localhost:5173/?certhash=<64 hex chars>` (used by M0-021), and builds
`new WebTransport(url, { serverCertificateHashes: [{ algorithm: 'sha-256', value: bytes }] })`. Regenerate the
certificate every <= 13 days (the browser refuses expired ones; the hash changes each time).

Session URL: `https://<host>:<wt-port>/wt` (`/` is accepted too, other paths get 404). Wire format
(ARCHITECTURE §6.6.1): C0 = the client's first bidi stream, C4 = the server's first uni stream, both with
`[len u16 LE][payload]` frames (no channel byte; the stream is the channel); C1 (up) and C2/C3 (down) are
datagrams `[channel u8][payload]`. Read `datagram.maxDatagramSize` on the client; the server drops (and counts in
`net_datagram_oversize_total`) any C2/C3 packet larger than the live `max_datagram_size` (loopback: 1413 B).

Browser smoke page (`tools/dev/wt-smoke/index.html`, serve it with any static server; the page needs the encoded
Hello/C1 bytes, `gen_dev_cert` prints a ready-made URL in its last line). Server-side test commands:

```sh
cargo test -p frontline-server --test transport_matrix --test wt_loopback --test wt_stress   # ws+wt handshake matrix, loopback, binary smoke
cargo test --release -p frontline-server --test wt_stress -- --ignored --nocapture            # 100 clients x 120 s (WT_STRESS_SECS, WT_STRESS_CLIENTS)
```

Client networking (M0-021, `client/src/net`): URL parameters `?server=host[:wsport]` (default 7000), `?wtport=` (default ws port + 1),
`?transport=wt|ws|auto`, `?certhash=<hex|base64>`. `auto` tries WebTransport and falls back to WebSocket when `WebTransport` is undefined,
its constructor throws, `ready` rejects, or the session + handshake is not done within 3 s; a server Reject never falls back. Browser e2e against a real
server (headless Chromium, own harness page on localhost, needs `cargo build -p frontline-server` and the sim-wasm pkg):

```sh
pnpm -C client exec playwright test -c playwright.net.config.ts      # FRONTLINE_REQUIRE_SERVER=1 turns the missing-binary skip into a failure
```

## Determinism and native-vs-wasm drift (M0-029)

```sh
wasm-pack build crates/sim-wasm --target web --profile wasm-release       # pkg used by the wasm half
tools/determinism/run.sh [--strict]    # native x2 + 10-player World x2 -> wasm x2 -> reports/m0/determinism/summary.json (~15 s)
cargo test -p frontline-sim-wasm --test determinism_native                 # same checks in test form (+ fault injection, golden hash)
node --test tools/determinism/stats.test.mjs                               # drift statistics unit tests
pnpm -C client exec vitest run src/render/terrain/terrain-vs-sim.test.ts   # client heightAt vs sim height_at (wasm)
```

Native x2 / wasm x2 differences always fail. Native vs wasm differences are a warning by default (R5) and fail with
`--strict` / `FRONTLINE_DETERMINISM_STRICT=1` (CI: repository variable `DETERMINISM_STRICT=1`). Raw traces go to
the git-ignored `target/determinism/`; only `summary.json` is tracked.
