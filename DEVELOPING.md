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

## Browser interop matrix (M0-028)

`scripts/run-interop.sh` runs 10 s probes (handshake, `InputBundle` at 20 Hz, count of C2 packets, RTT median) for WT and WS in every
Playwright browser that is installed, the "delete `WebTransport` -> `auto` falls back to WS <= 5 s" case in each of them, and (Chromium)
the "server has no WT port" fallback. It reuses the M0-021 harness page (`client/e2e/net-harness`) and process helpers
(`client/e2e/net-support.ts`); the spec is `client/e2e/interop/interop.spec.ts`. Output: `reports/m0/browser-matrix.{json,md}`.
A browser whose binary is missing is recorded as "未验证 (browser binary not installed)", never as passed or failed. Only Chromium asserts.

```sh
scripts/run-interop.sh                              # installed browsers; FRONTLINE_INTEROP_BROWSERS=chromium,firefox restricts
INSTALL_BROWSERS=1 scripts/run-interop.sh           # CI / a machine with network: playwright install --with-deps first
node --experimental-strip-types scripts/check-interop-table.mjs   # JSON == .md == the table cells in the tasks/STATUS.md M0-028 row
pnpm -C client exec vitest run e2e/interop          # unit tests of the summary functions
```

Not verifiable on the dev VM (only Chromium 153 is installed): Firefox, WebKit (a WebKit build is not Safari), Safari proper.
Manual check on a real Firefox / Safari (H-103), no Playwright needed:

1. `cargo build -p frontline-server && tools/dev/gen-dev-cert.sh`, then
   `target/debug/frontline-server --cert dev-cert/dev-cert.pem --key dev-cert/dev-key.pem --dev-echo` (note the `ws_listening` / `wt_listening` ports).
2. `pnpm -C client dev`, open `http://localhost:5173/e2e/net-harness/index.html?server=127.0.0.1:7000&wtport=7001&wasm=<url of frontline_sim_wasm.js>&transport=wt&certhash=<hex from dev-cert/dev-cert.sha256>`
   (the harness needs the built `crates/sim-wasm/pkg`; `/@fs/<abs path>/crates/sim-wasm/pkg/frontline_sim_wasm.js` works with the dev server).
   Expected: the page shows `status=connected`, `transport=wt`, `fallback_reason=none`. Anything else: copy the page text (the error is
   shown verbatim) into the matrix. `transport=ws` must show `transport=ws`; `transport=auto` on a browser without `serverCertificateHashes`
   shows `transport=ws` with the fallback reason (record it as "WT 无法测试：缺受信任证书，见 H-017", not as a failure).
3. In the console, `__net.sendInput()` in a loop (every 50 ms for 10 s) and read `__net.snapshots` (expect >= 150 with `--dev-echo`) and
   `__net.session.clock.rtt()`.

The manual URL above is untested on the dev server (the e2e starts its own Vite server for the harness page, which is what is verified).

## Server sessions and input (M0-018)

The server loads the map at startup (`--map m0`, directory `data/maps/m0` found by walking up from the working directory
and the executable, or `--data-dir <dir containing maps/>`): 16 collision chunks `chunks/c_{bx}_{bz}.bin` plus the HQ points of
`spawns.json` (`set_team_spawn` per team, `set_map_center`). A missing or damaged map is a fatal startup error that prints the
regeneration command (`PYTHONPATH=tools/map-pipeline python3 -m map_pipeline m0 --seed 1 --out data/maps/m0`). Hidden test-only flags:
`--empty-world` (no terrain, players cannot move) and `--dev-echo` (C1 -> C2 echo instead of the session layer; implies `--empty-world`).

Input path (all on the sim thread): `InboundQueue` (C1 bundles) -> `SimBridge::ingest_bundle` -> per command `PlayerInput::ingest` =
window check (`now-2 ..= now+10`) -> `validate_input` (the one entry point, also for bots) -> 8-slot ring buffer; each tick
`SimBridge::resolve(tick)` yields one command per joined player, ascending `PlayerId` (missing input: repeat movement <= 5 ticks with
edge bits cleared, then zero movement keeping the view; >= 9 silent ticks = `session_stall_total`). Teams are assigned at the handshake
(fewer players first, US on a tie), `PlayerId`s are not reused for 1 s, C0 `Ping` is limited to 5/s (excess dropped, `input_rejected_total{reason="c0_rate"}`),
30 violating commands in a row close the connection. Metrics: `input_rejected_total{reason}`, `input_accepted_total`, `input_missing_ticks_total`,
`session_stall_total`, `session_kicked_total`.

```sh
cargo test -p frontline-server --test input_buffer --test session_flow --test session_load   # needs the terrain (see "Map data")
cargo test --release -p frontline-server --test session_load -- --ignored --nocapture        # 20 and 100 clients x 60 s (FRONTLINE_LOAD_SECS)
```

## Bots (M0-026)

`frontline-server --bots N` hosts N soldier bots in the sim thread (teams alternate US, GER, ...; `PlayerId`s count down from 127 and are
reserved against human joins; nicknames `[BOT] 01`...; gauge `server_bots`; histograms `bot_think_ms` / `bot_path_ms`, both per tick). A bot is a
client without a connection: `crates/bots` reads the world only through the read-only `WorldView` and returns one `InputCmd` per tick; the host
(`crates/server/src/bot_host.rs`) pushes it through `SimBridge::ingest_cmd` (window check + `validate_input` + input buffer, the human path).
Tuning (difficulty tiers, FOV, aim error, ranges) lives in `data/rules/bots.json` (schema `data/schemas/bots.schema.json`; M0 uses the
`default_difficulty` tier = Regular). Navigation is a 4 m walkability grid + A* (no navmesh before M1-036). Needs the terrain.

```sh
cargo test -p frontline-bots                                    # A*, perception/no-wallhack, state machine, golden input hash
cargo test -p frontline-server --test bots_soak --test bots_validation
cargo test --release -p frontline-server --test bots_soak -- --ignored --nocapture   # 5 min soak (FRONTLINE_SOAK_SECS / FRONTLINE_SOAK_BOTS)
cargo bench -p frontline-bots --bench think                     # think_10 / think_60 / think_100 / path_query -> reports/baselines/bots-m0.json
```

## Headless client and load tests (M0-027)

`headless-client` (`crates/headless-client`) runs N synthetic players over the real WebSocket / WebTransport paths (30 Hz waypoint walking
from `spawns.json` `patrol_points`, a burst every 2-6 s, the odd jump/crouch), decodes C2 snapshots, predicts with `sim::PredictSim` and
reconciles on `pos_corr` exactly like the browser client (PI-023), and writes a JSON report. It never links `server`. The server allows 4
connections per IP, so on loopback each client binds its own `127.x.y.z` source address (`--no-bind-spread` turns that off).

```sh
cargo build --release -p frontline-server -p frontline-headless-client
scripts/loadtest-m0.sh --transport wt --duration 1800            # acceptance 1: 10 headless + 10 bots, writes reports/m0/loadtest-wt.json
scripts/loadtest-m0.sh --transport ws --duration 600             # acceptance 2
scripts/loadtest-m0-matrix.sh --duration 120                     # 10 / 20 / 100 clients, with and without bots -> reports/m0/matrix/
cargo test -p frontline-headless-client                          # units + smoke_ws/smoke_wt (need `cargo build -p frontline-server`; FRONTLINE_REQUIRE_SERVER=1 fails instead of skipping)
cargo test -p frontline-headless-client --test smoke -- --ignored --nocapture   # loadtest-smoke: 60 s x 3 clients, WT + WS
target/release/headless-client --hash-only 6000 --count 5 --seed 1   # input-sequence fingerprints (equal for equal seed)
HEADLESS_TRACE=1 target/release/headless-client ...              # print every rollback (time, error, health, ack)
```

The script starts its own server on random ports (never 7000/7001/9100), pins server and client to separate CPU pairs when `taskset` and 4
CPUs exist, samples `/metrics` and RSS every 10 s and stops only the PIDs it started. `scripts/summarize-loadtest.mjs` merges the server's
`--report` (exact tick percentiles, event tally), the final `/metrics` scrape (bucket upper bounds for AOI / bytes) and the headless report
and judges the acceptance numbers. Bandwidth is reported on the line-estimate basis (payload + 58 B per packet) with payload-only next to it.
Timing judgements are for release builds.

## Local demo / remote preview

Server plus a Vite dev server whose `/game-ws` path proxies to the game server's WebSocket port, so a preview that
forwards only the page port (TCP, no UDP) can still play. `?server=same` (also the default when the page is not opened
from localhost / 127.0.0.1) connects to `<page origin>/game-ws` over WebSocket only (no WebTransport attempt).

```sh
PYTHONPATH=tools/map-pipeline python3 -m map_pipeline m0 --seed 1 --out data/maps/m0   # terrain (skip if already generated)
cargo build --release -p frontline-server
target/release/frontline-server --bots 10 --ws-port 7000                               # WebSocket only
pnpm -C client sync-map                                                                 # terrain + collision + wasm into client/public
pnpm -C client dev --host 0.0.0.0 --port 5173                                           # FRONTLINE_WS_TARGET=ws://host:port overrides the proxy target
```

Recommended URLs: `/?server=same&scene=vegetation&debug=1` (terrain + vegetation; software rendering is slow, 1-4 fps) and the
light `/?server=same&empty=1&debug=1`. Add `&renderer=webgl2` to force WebGL2.

Keys (defaults from `client/src/input/keymap.ts`; click the canvas for pointer lock): W/A/S/D move, mouse look, Left Shift
sprint (hold), C crouch, Space jump, left mouse fire, right mouse aim, R reload. Walk is unbound by default (debug: hold CapsLock).

## Asset pipeline (M1-001)

`tools/asset-pipeline` = steps 1–2 of ARCHITECTURE §11.1 plus the missing-asset report. Reads the ledger in `LICENSES.md` §3 and
refuses anything that is unregistered (exit 2), not `OK` (3, `FLAGGED` counts as `PENDING`), a D-LIC channel without a confirmation (4),
or missing archive files (5). Intermediate products go to `work/` (git-ignored).

```
A=tools/asset-pipeline/bin/asset-pipeline.mjs
node $A ledger-check                       # LICENSES §4.4: counts per status, missing archive files
node $A ingest                             # assets/incoming/** -> work/{asset_id}/raw/ (+ sha256, ingest.json)
node $A normalize --asset AST-... --id prop_crate_01 [--unit-scale 0.01 --up Z --forward -Y --origin bottom-center]
node $A report --milestone M1 --placeholders data/placeholders --gaps    # real / placeholder / missing per requirement ID
node $A gen-requirements                   # refresh assets/requirements.json from docs/ASSET_SHOPPING_LIST.md §4
pnpm -C tools/asset-pipeline lint | typecheck | test
```
