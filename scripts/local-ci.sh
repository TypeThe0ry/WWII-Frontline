#!/usr/bin/env bash
# Local merge gate (PI-078): GitHub Actions no longer run automatically; this script runs the same
# checks on the Linux workstation. Usage: scripts/local-ci.sh [group ...]   (default: all)
# Groups: lint rust ts data determinism wasm assets map
# A group whose tool is missing is reported as SKIPPED and makes the script exit 3 (not green).
set -uo pipefail
cd "$(dirname "$0")/.."
export CARGO_INCREMENTAL=${CARGO_INCREMENTAL:-0} CARGO_PROFILE_DEV_DEBUG=${CARGO_PROFILE_DEV_DEBUG:-0} CARGO_PROFILE_TEST_DEBUG=${CARGO_PROFILE_TEST_DEBUG:-0}
export PYTHONPATH=tools/map-pipeline

GROUPS_ALL=(lint rust ts data determinism wasm assets map)
SEL=("$@"); [ ${#SEL[@]} -eq 0 ] && SEL=("${GROUPS_ALL[@]}")
PASS=(); FAIL=(); SKIP=()

need() { for t in "$@"; do command -v "$t" >/dev/null 2>&1 || { echo "missing tool: $t"; return 1; }; done; }
run() { echo "+ $*"; "$@"; }

g_lint() {
  need cargo pnpm node || return 2
  run cargo fmt --all --check && run cargo clippy --workspace --all-targets -- -D warnings || return 1
  run pnpm -C client exec eslint . && run pnpm -C client exec tsc --noEmit || return 1
  if command -v cargo-deny >/dev/null 2>&1; then run cargo deny check || return 1; else echo "WARN: cargo-deny not installed, licence check not run"; fi
  run node tools/ci/check-deps.mjs && run node tools/ci/check-sim-purity.mjs && run node tools/ci/check-layout.mjs \
    && run node --test "tools/ci/test/*.test.mjs"
}
g_rust() {
  need cargo || return 2
  run cargo test --workspace --locked && run cargo test -p frontline-sim --features test-util --locked
}
g_ts() { need pnpm || return 2; run pnpm -C client test; }
g_data() {
  need pnpm node || return 2
  run pnpm -C tools/data-validate test && run node tools/data-validate/index.mjs --fixtures \
    && run node tools/data-validate/index.mjs --root data
}
g_determinism() {
  need cargo wasm-pack node pnpm python3 || return 2
  run python3 -m map_pipeline m0 --seed 1 --out data/maps/m0 || return 1
  run wasm-pack build crates/sim-wasm --target web --profile wasm-release || return 1
  run cargo test -p frontline-sim-wasm --test determinism_native || return 1
  run cargo run --release -p frontline-sim-wasm --example determinism || return 1
  run node tools/determinism/run-wasm.mjs || return 1
  run node --test tools/determinism/stats.test.mjs || return 1
  if [ "${FRONTLINE_DETERMINISM_STRICT:-}" = "1" ]; then run node tools/determinism/compare.mjs --strict; else run node tools/determinism/compare.mjs; fi || return 1
  run pnpm -C client exec vitest run src/render/terrain/terrain-vs-sim.test.ts
}
g_wasm() {
  need cargo wasm-pack node python3 || return 2
  command -v wasm-opt >/dev/null 2>&1 || echo "WARN: wasm-opt not on PATH; the size gate may use an unoptimised build"
  run wasm-pack build crates/sim-wasm --target web --profile wasm-release && run node tools/ci/check-wasm-size.mjs || return 1
  run python3 -m map_pipeline m0 --seed 1 --out data/maps/m0 && run cargo run --release -p frontline-sim-wasm --example trace || return 1
  export FRONTLINE_REQUIRE_TERRAIN=1
  run node crates/sim-wasm/tests/node-smoke.mjs || return 1
  run node crates/sim-wasm/tests/native-vs-wasm.mjs || echo "WARN: native vs wasm drift (non-blocking per R5)"
  run node crates/sim-wasm/tests/bench.mjs
}
g_assets() {
  need pnpm node ffmpeg || return 2
  local w; w=$(mktemp -d)
  run pnpm -C tools/asset-pipeline lint && run pnpm -C tools/asset-pipeline typecheck \
    && run pnpm -C tools/asset-pipeline test && run pnpm -C tools/asset-pipeline test:e2e || return 1
  run node tools/asset-pipeline/bin/asset-pipeline.mjs ledger-check && run node tools/asset-pipeline/bin/asset-pipeline.mjs ingest --work "$w" || return 1
  run node tools/asset-pipeline/bin/asset-pipeline.mjs gen-requirements --out "$w/requirements.json" && diff -u assets/requirements.json "$w/requirements.json" || return 1
  run node tools/asset-pipeline/bin/asset-pipeline.mjs report --milestone M1 --placeholders data/placeholders --gaps --strict --work "$w" --out "$w/asset-report.json" || return 1
  if [ -f client/dist/size-report.json ]; then
    git show "origin/${BASE_BRANCH:-claude/serene-wright-lc7k1n}:assets/manifests/manifest.json" > "$w/base-manifest.json" 2>/dev/null || rm -f "$w/base-manifest.json"
    run node tools/asset-pipeline/bin/asset-pipeline.mjs manifest-budget --base "$w/base-manifest.json" --core-size-file client/dist/size-report.json
  else echo "WARN: client/dist/size-report.json missing (build the client first); manifest-budget not run"; fi
}
g_map() { need python3 || return 2; (cd tools/map-pipeline && run python3 -m pytest -q) && run python3 -m map_pipeline validate-vectors data/maps/carentan; }

for g in "${SEL[@]}"; do
  declare -F "g_$g" >/dev/null || { echo "unknown group: $g (groups: ${GROUPS_ALL[*]})"; exit 64; }
  echo; echo "=== $g ==="
  "g_$g"; rc=$?
  case $rc in 0) PASS+=("$g");; 2) SKIP+=("$g");; *) FAIL+=("$g");; esac
done
echo; echo "local-ci: pass=[${PASS[*]:-}] fail=[${FAIL[*]:-}] skipped=[${SKIP[*]:-}]"
[ ${#FAIL[@]} -gt 0 ] && exit 1
[ ${#SKIP[@]} -gt 0 ] && exit 3
exit 0
