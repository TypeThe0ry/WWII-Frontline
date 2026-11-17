#!/usr/bin/env bash
# M0-027: the 10 / 20 / 100 headless-client matrix (with and without server-side bots), 
# one summary per cell under reports/m0/matrix/, then a one-line-per-cell table on stdout.
#   scripts/loadtest-m0-matrix.sh [--duration 120] [--no-build]
set -euo pipefail
cd "$(dirname "$0")/.."
DURATION=120 PASS=()
while [ $# -gt 0 ]; do
  case "$1" in
    --duration) DURATION="$2"; shift 2 ;;
    --no-build) PASS+=(--no-build); shift ;;
    *) echo "unknown option: $1" >&2; exit 2 ;;
  esac
done
mkdir -p reports/m0/matrix
# transport clients bots
CELLS=("wt 10 0" "wt 10 10" "wt 20 0" "wt 20 10" "wt 100 0" "wt 100 10" "ws 10 10" "ws 100 10")
for cell in "${CELLS[@]}"; do
  set -- $cell
  out="reports/m0/matrix/$1-c$2-b$3.json"
  scripts/loadtest-m0.sh --transport "$1" --count "$2" --bots "$3" --duration "$DURATION" \
    --warmup 10 --out "$out" ${PASS[@]+"${PASS[@]}"} >"reports/m0/matrix/$1-c$2-b$3.log" 2>&1 || true
  PASS=(--no-build)
done
node -e '
const fs=require("fs");
const rows=fs.readdirSync("reports/m0/matrix").filter(f=>f.endsWith(".json")).sort();
console.log("| transport | clients | bots | tick P50 ms | tick P99 ms | max ms | overruns (max consec) | down P50 / P99 kbps (worst client P50) | up P50 kbps | decode err | disconnects | corrections | kills | client CPU % |");
console.log("|---|---|---|---|---|---|---|---|---|---|---|---|---|---|");
for(const f of rows){const s=JSON.parse(fs.readFileSync("reports/m0/matrix/"+f));const c=s.clients,v=s.server;
console.log(`| ${s.transport} | ${s.count} | ${s.bots} | ${v.tick_ms.p50} | ${v.tick_ms.p99} | ${v.tick_ms.max} | ${v.overrun_total} (${v.overrun_consecutive_max}) | ${c.down_kbps.p50} / ${c.down_kbps.p99} (${c.down_kbps_p50_worst_client}) | ${c.up_kbps.p50} | ${c.decode_errors} | ${c.disconnects} | ${c.corrections.total} | ${v.events?.kills ?? "?"} | ${c.client_cpu_pct_of_one_core} |`)}'
