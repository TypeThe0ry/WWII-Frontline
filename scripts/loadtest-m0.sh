#!/usr/bin/env bash
# M0-027: N headless clients (+ M server-side bots) against a real frontline-server process.
#
#   scripts/loadtest-m0.sh --transport wt --duration 1800            # acceptance 1 (10 clients + 10 bots)
#   scripts/loadtest-m0.sh --transport ws --duration 600             # acceptance 2
#   scripts/loadtest-m0.sh --count 100 --bots 0 --duration 120 --out reports/m0/loadtest-wt-100.json
#
# Options: --transport wt|ws  --duration S  --count N  --bots N  --seed N  --warmup S
#          --sample-interval S (default 10)  --out FILE  --debug (use the debug build)  --no-build
#          --no-pin (by default, with `taskset` and >= 4 CPUs, the server is pinned to CPUs 0-1 and the
#          headless client to CPUs 2-3 so the two do not steal each other's cycles)
#
# Starts the server (random free ports, --dev-cert), the headless client, samples /metrics and the
# servers' RSS every --sample-interval seconds, stops everything, and merges the results with
# scripts/summarize-loadtest.mjs. Both processes run on THIS machine: record the host in the report
# (the summary does) and read the numbers with that in mind (README §5.5: same-host P99 is "partial").
set -euo pipefail
cd "$(dirname "$0")/.."

TRANSPORT=wt DURATION=60 COUNT=10 BOTS=10 SEED=1 WARMUP=5 INTERVAL=10 OUT="" PROFILE=release BUILD=1 PIN=1
while [ $# -gt 0 ]; do
  case "$1" in
    --transport) TRANSPORT="$2"; shift 2 ;;
    --duration) DURATION="$2"; shift 2 ;;
    --count) COUNT="$2"; shift 2 ;;
    --bots) BOTS="$2"; shift 2 ;;
    --seed) SEED="$2"; shift 2 ;;
    --warmup) WARMUP="$2"; shift 2 ;;
    --sample-interval) INTERVAL="$2"; shift 2 ;;
    --out) OUT="$2"; shift 2 ;;
    --debug) PROFILE=debug; shift ;;
    --no-build) BUILD=0; shift ;;
    --no-pin) PIN=0; shift ;;
    -h|--help) sed -n '2,15p' "$0"; exit 0 ;;
    *) echo "unknown option: $1" >&2; exit 2 ;;
  esac
done
case "$TRANSPORT" in wt|ws) ;; *) echo "--transport must be wt or ws" >&2; exit 2 ;; esac
[ -n "$OUT" ] || OUT="reports/m0/loadtest-$TRANSPORT.json"

export CARGO_INCREMENTAL=0 CARGO_PROFILE_DEV_DEBUG=0 CARGO_PROFILE_TEST_DEBUG=0
if [ "$PROFILE" = release ]; then FLAG=(--release); else FLAG=(); fi
if [ "$BUILD" = 1 ]; then
  cargo build "${FLAG[@]}" -p frontline-server -p frontline-headless-client
fi
SERVER="target/$PROFILE/frontline-server"
HEADLESS="target/$PROFILE/headless-client"
[ -x "$SERVER" ] && [ -x "$HEADLESS" ] || { echo "missing $SERVER or $HEADLESS (build first)" >&2; exit 2; }

SRV_PIN=() HL_PIN=()
if [ "$PIN" = 1 ] && command -v taskset >/dev/null && [ "$(nproc)" -ge 4 ]; then
  SRV_PIN=(taskset -c 0,1); HL_PIN=(taskset -c 2,3)
  echo "pinning: server -> CPUs 0,1; headless-client -> CPUs 2,3"
else
  echo "pinning: off (server and clients share all CPUs)"
fi

T="$(mktemp -d "${TMPDIR:-/tmp}/loadtest-XXXXXX")"
SERVER_PID="" HEADLESS_PID="" SAMPLER_PID=""
cleanup() {
  for p in "$SAMPLER_PID" "$HEADLESS_PID" "$SERVER_PID"; do
    [ -n "$p" ] && kill "$p" 2>/dev/null || true
  done
  wait 2>/dev/null || true
}
trap cleanup EXIT INT TERM

${SRV_PIN[@]+"${SRV_PIN[@]}"} "$SERVER" --ws-port 0 --wt-port 0 --metrics-port 0 --dev-cert --bots "$BOTS" \
  --report "$T/server.json" >"$T/server.log" 2>&1 &
SERVER_PID=$!
# wait for the three listening lines
for _ in $(seq 1 100); do
  if grep -q '"event":"metrics_listening"' "$T/server.log" 2>/dev/null; then break; fi
  kill -0 "$SERVER_PID" 2>/dev/null || { echo "server exited early:"; tail -5 "$T/server.log"; exit 1; }
  sleep 0.1
done
port_of() { grep "\"event\":\"$1\"" "$T/server.log" | head -1 | sed -n 's/.*"port":\([0-9]*\).*/\1/p'; }
WS_PORT=$(port_of ws_listening); WT_PORT=$(port_of wt_listening); METRICS_PORT=$(port_of metrics_listening)
CERT=$(grep '"event":"wt_listening"' "$T/server.log" | head -1 | sed -n 's/.*"cert_sha256_hex":"\([0-9a-f]*\)".*/\1/p')
[ -n "$WS_PORT" ] && [ -n "$METRICS_PORT" ] || { echo "could not read ports from server log" >&2; exit 1; }

if [ "$TRANSPORT" = wt ]; then
  URL="127.0.0.1:$WT_PORT"; EXTRA=(--cert-hash "$CERT")
else
  URL="127.0.0.1:$WS_PORT"; EXTRA=()
fi
echo "server pid $SERVER_PID: ws $WS_PORT wt $WT_PORT metrics $METRICS_PORT; $COUNT $TRANSPORT clients + $BOTS bots for ${DURATION}s ($PROFILE build)"

: >"$T/rss.tsv"
sample() { # $1 = label (elapsed seconds)
  local t=$1
  curl -s --max-time 3 "localhost:$METRICS_PORT/metrics" >"$T/metrics-$(printf '%06d' "$t").txt" || true
  local srv hl
  srv=$(awk '/^VmRSS/ {print $2}' "/proc/$SERVER_PID/status" 2>/dev/null || true)
  hl=$(awk '/^VmRSS/ {print $2}' "/proc/${HEADLESS_PID:-0}/status" 2>/dev/null || true)
  printf '%s\t%s\t%s\n' "$t" "${srv:-}" "${hl:-}" >>"$T/rss.tsv"
}

START=$(date +%s)
${HL_PIN[@]+"${HL_PIN[@]}"} "$HEADLESS" --url "$URL" --transport "$TRANSPORT" "${EXTRA[@]}" --count "$COUNT" --duration "$DURATION" \
  --seed "$SEED" --warmup "$WARMUP" --report "$T/headless.json" 2>"$T/headless.log" &
HEADLESS_PID=$!
(
  while kill -0 "$HEADLESS_PID" 2>/dev/null; do
    sample $(( $(date +%s) - START ))
    sleep "$INTERVAL"
  done
) &
SAMPLER_PID=$!

HL_STATUS=0
wait "$HEADLESS_PID" || HL_STATUS=$?
HEADLESS_PID=""
wait "$SAMPLER_PID" 2>/dev/null || true
SAMPLER_PID=""
# final scrape (clients just left: players gauge is 0, histograms are complete)
sample $(( $(date +%s) - START ))
kill -TERM "$SERVER_PID" 2>/dev/null || true
wait "$SERVER_PID" 2>/dev/null || true
SERVER_PID=""

cat "$T/headless.log" >&2
RC=0
node scripts/summarize-loadtest.mjs --headless "$T/headless.json" --server-report "$T/server.json" \
  --metrics-dir "$T" --rss "$T/rss.tsv" --out "$OUT" --transport "$TRANSPORT" --bots "$BOTS" \
  --profile "$PROFILE" --sample-interval "$INTERVAL" --pinned "$([ ${#SRV_PIN[@]} -gt 0 ] && echo 1 || echo 0)" || RC=$?
echo "(raw files kept in $T)"
exit "$RC"
