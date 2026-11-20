#!/usr/bin/env bash
# M0-027: N headless clients (+ M server-side bots) against a real frontline-server process.
#
#   scripts/loadtest-m0.sh --transport wt --duration 1800            # acceptance 1 (10 clients + 10 bots)
#   scripts/loadtest-m0.sh --transport ws --duration 600             # acceptance 2
#   scripts/loadtest-m0.sh --count 100 --bots 0 --duration 120 --out reports/m0/loadtest-wt-100.json
#
# Env: LOADTEST_SERVER_CPUS / LOADTEST_CLIENT_CPUS (default 0,1 / 2,3) pick the pinned CPUs on big shared machines.
#   FRONTLINE_SIM_CPU=<logical CPU> enables verified sim isolation. Set LOADTEST_SERVER_CPUS to
#   the non-sim CPU list; CPU lists in this mode accept comma-separated CPUs and inclusive ranges.
#   Check SMT siblings (/sys/devices/system/cpu/cpuN/topology/thread_siblings_list) and keep them
#   and the sim CPU free of other work, including the harness itself. This is not host CPU reservation.
#   LOADTEST_LOG_DIR=<existing writable tmpfs> keeps server.log and headless.log there, linked from
#   the normal raw directory. Both per-run directories are retained; remove both after archiving.
# Options: --transport wt|ws  --duration S  --count N  --bots N  --seed N  --warmup S
#          --sample-interval S (default 10)  --out FILE  --debug (use the debug build)  --no-build
#          --map NAME  --mode NAME  --data-dir PATH  --deploy-hq (C0 on tactical maps)
#          --drive-vehicle (client 0 drives; uses a local M0 test vehicle at its join point)
#          --no-pin (by default, with `taskset` and >= 4 CPUs, the server is pinned to CPUs 0-1 and the
#          headless client to CPUs 2-3 so the two do not steal each other's cycles)
#
# Starts the server (random free ports, --dev-cert), the headless client, samples /metrics and the
# servers' RSS every --sample-interval seconds, stops everything, and merges the results with
# scripts/summarize-loadtest.mjs. Both processes run on THIS machine: record the host in the report
# (the summary does) and read the numbers with that in mind (README §5.5: same-host P99 is "partial").
set -euo pipefail
cd "$(dirname "$0")/.."

TRANSPORT=wt DURATION=60 COUNT=10 BOTS=10 SEED=1 WARMUP=5 INTERVAL=10 OUT="" PROFILE=release BUILD=1 PIN=1 DRIVE=0 DEPLOY=0
MAP="${FRONTLINE_MAP:-m0}" MODE="${FRONTLINE_MODE:-sandbox}" DATA="${FRONTLINE_DATA_DIR:-$PWD/data}"
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
    --drive-vehicle) DRIVE=1; shift ;;
    --map) MAP="$2"; shift 2 ;;
    --mode) MODE="$2"; shift 2 ;;
    --data-dir) DATA="$2"; shift 2 ;;
    --deploy-hq) DEPLOY=1; shift ;;
    -h|--help) sed -n '2,/^# Starts the server/{ /^# Starts the server/!p; }' "$0"; exit 0 ;;
    *) echo "unknown option: $1" >&2; exit 2 ;;
  esac
done
case "$TRANSPORT" in wt|ws) ;; *) echo "--transport must be wt or ws" >&2; exit 2 ;; esac
[ -n "$OUT" ] || OUT="reports/m0/loadtest-$TRANSPORT.json"

die() { echo "loadtest: $*" >&2; exit 2; }

LOG_ROOT=""
if [ -n "${LOADTEST_LOG_DIR:-}" ]; then
  LOG_ROOT=$(realpath -e -- "$LOADTEST_LOG_DIR") || die "LOADTEST_LOG_DIR must already exist"
  [[ -d "$LOG_ROOT" && -w "$LOG_ROOT" && -x "$LOG_ROOT" ]] || die "LOADTEST_LOG_DIR must be a writable directory"
  [ "$(stat -f -c %T -- "$LOG_ROOT")" = tmpfs ] || die "LOADTEST_LOG_DIR must be on tmpfs: $LOG_ROOT"
fi

SIM_CPU=${FRONTLINE_SIM_CPU:-}
SERVER_CPUS=${LOADTEST_SERVER_CPUS:-0,1}
CLIENT_CPUS=${LOADTEST_CLIENT_CPUS:-2,3}
SRV_PIN=() HL_PIN=()
if [ -n "$SIM_CPU" ]; then
  [[ "$SIM_CPU" =~ ^(0|[1-9][0-9]*)$ ]] || die "FRONTLINE_SIM_CPU must be one logical CPU number"
  if [[ "$PIN" != 1 || ! -r /proc/self/status ]] || ! command -v taskset >/dev/null; then
    die "FRONTLINE_SIM_CPU requires Linux /proc and taskset, without --no-pin"
  fi
  [ -n "${LOADTEST_SERVER_CPUS:-}" ] || die "set LOADTEST_SERVER_CPUS to the non-sim CPU list"

  # /proc masks are canonical (sorted, merged ranges). Check coverage so a cpuset or
  # offline CPU cannot silently truncate a requested taskset mask.
  covers_cpus() {
    awk -v allowed="$1" -v requested="$2" 'BEGIN {
      n = split(allowed, a, ","); m = split(requested, r, ",")
      for (i = 1; i <= m; i++) {
        k = split(r[i], b, "-"); lo = b[1]+0; hi = (k == 2 ? b[2]+0 : lo)
        if (hi < lo) exit 1
        for (j = 1; j <= n; j++) {
          k = split(a[j], b, "-")
          if (lo >= b[1]+0 && hi <= (k == 2 ? b[2]+0 : b[1]+0)) break
        }
        if (j > n) exit 1
      }
    }'
  }
  effective_mask() {
    [[ "$1" =~ ^[0-9]+(-[0-9]+)?(,[0-9]+(-[0-9]+)?)*$ ]] || return 1
    local mask
    # shellcheck disable=SC2016 # The field references belong to awk.
    mask=$(taskset -c "$1" awk '$1 == "Cpus_allowed_list:" {print $2}' /proc/self/status) || return 1
    [ -n "$mask" ] && covers_cpus "$mask" "$1" || return 1
    printf '%s\n' "$mask"
  }
  SERVER_MASK=$(effective_mask "$SERVER_CPUS") || die "cannot apply LOADTEST_SERVER_CPUS=$SERVER_CPUS in full"
  CLIENT_MASK=$(effective_mask "$CLIENT_CPUS") || die "cannot apply LOADTEST_CLIENT_CPUS=$CLIENT_CPUS in full"
  if covers_cpus "$SERVER_MASK" "$SIM_CPU" || covers_cpus "$CLIENT_MASK" "$SIM_CPU"; then
    die "sim CPU $SIM_CPU overlaps LOADTEST_SERVER_CPUS or LOADTEST_CLIENT_CPUS"
  fi
  SRV_PIN=(taskset -c "$SERVER_CPUS"); HL_PIN=(taskset -c "$CLIENT_CPUS")
  echo "pinning: sim -> CPU $SIM_CPU; other server threads -> CPUs $SERVER_MASK; headless-client -> CPUs $CLIENT_MASK"
elif [ "$PIN" = 1 ] && command -v taskset >/dev/null && [ "$(nproc)" -ge 4 ]; then
  SRV_PIN=(taskset -c "$SERVER_CPUS"); HL_PIN=(taskset -c "$CLIENT_CPUS")
  echo "pinning: server -> CPUs $SERVER_CPUS; headless-client -> CPUs $CLIENT_CPUS (override with LOADTEST_SERVER_CPUS / LOADTEST_CLIENT_CPUS)"
else
  echo "pinning: off (server and clients share all CPUs)"
fi

export CARGO_INCREMENTAL=0 CARGO_PROFILE_DEV_DEBUG=0 CARGO_PROFILE_TEST_DEBUG=0
if [ "$PROFILE" = release ]; then FLAG=(--release); else FLAG=(); fi
if [ "$BUILD" = 1 ]; then
  cargo build "${FLAG[@]}" -p frontline-server -p frontline-headless-client
fi
SERVER="target/$PROFILE/frontline-server"
HEADLESS="target/$PROFILE/headless-client"
[ -x "$SERVER" ] && [ -x "$HEADLESS" ] || { echo "missing $SERVER or $HEADLESS (build first)" >&2; exit 2; }

T="$(mktemp -d "${TMPDIR:-/tmp}/loadtest-XXXXXX")"
if [ -n "$LOG_ROOT" ]; then
  LOG_RUN=$(mktemp -d "$LOG_ROOT/loadtest-logs-XXXXXX") || die "cannot create tmpfs log directory under $LOG_ROOT"
  echo "(raw files kept in $T; logs kept in $LOG_RUN; remove both directories after archiving)"
  for log in server.log headless.log; do
    # Use real files ahead of time so each link already resolves before launch.
    : >"$LOG_RUN/$log"
    ln -s -- "$LOG_RUN/$log" "$T/$log"
  done
fi
SERVER_PID="" HEADLESS_PID="" SAMPLER_PID=""
# shellcheck disable=SC2329 # Called by the EXIT trap.
cleanup() {
  for p in "$SAMPLER_PID" "$HEADLESS_PID" "$SERVER_PID"; do
    [ -n "$p" ] && kill "$p" 2>/dev/null || true
  done
  wait 2>/dev/null || true
}
trap cleanup EXIT
trap 'exit 130' INT
trap 'exit 143' TERM

# Use the offline join of decoded C2 records and same-tick server observer samples for E1-4.
export FL_AOI_TRACE=1
SERVER_EXTRA=(--map "$MAP" --mode "$MODE" --data-dir "$DATA")
CLIENT_EXTRA=(--map "$MAP" --data-dir "$DATA")
DRIVER=() CHECKS=()
if [ "$DEPLOY" = 1 ]; then
  CLIENT_EXTRA+=(--deploy-hq)
  CHECKS+=(--deploy-hq)
fi
if [ "$DRIVE" = 1 ]; then
  if [ "$MAP" != m0 ] || [ "$MODE" != sandbox ] || [ "$DEPLOY" = 1 ]; then
    echo "--drive-vehicle requires the immediate-spawn M0 sandbox" >&2; exit 2
  fi
  SERVER_EXTRA+=(--dev-spawn-vehicle)
  # Give client 0 a head start on admission so it spawns next to the fixture.
  CLIENT_EXTRA+=(--ramp-ms 250)
  DRIVER=(--drive-vehicle)
fi
${SRV_PIN[@]+"${SRV_PIN[@]}"} "$SERVER" --ws-port 0 --wt-port 0 --metrics-port 0 --dev-cert --bots "$BOTS" \
  "${SERVER_EXTRA[@]}" --report "$T/server.json" >"$T/server.log" 2>&1 &
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

read_affinity() { grep -H "^Cpus_allowed_list:" /proc/"$1"/task/*/status 2>/dev/null || true; }

verify_affinity() { # role PID ordinary-mask [sim-mask]
  awk -v role="$1" -v ordinary="$3" -v sim="${4:-}" '
    $1 == "Name:" { name = $2 }
    $1 == "Cpus_allowed_list:" {
      threads++
      is_sim = (role == "server" && name == "sim")
      if (is_sim) sims++
      if (name == "log-output") logs++
      if (name == "net") workers++
      expected = (is_sim ? sim : ordinary)
      printf "%s\t%s\t%s\t%s\t%s\n", role, FILENAME, name, $2, expected
      if ($2 != expected) bad = 1
    }
    END {
      if (threads < 2 || (role == "server" && (sims != 1 || logs != 1 || workers < 1))) {
        print "loadtest: incomplete " role " thread snapshot" > "/dev/stderr"
        bad = 1
      }
      if (bad) exit 1
    }
  ' /proc/"$2"/task/*/status
}

START=$(date +%s)
${HL_PIN[@]+"${HL_PIN[@]}"} "$HEADLESS" --url "$URL" --transport "$TRANSPORT" "${EXTRA[@]}" --count "$COUNT" --duration "$DURATION" \
  "${DRIVER[@]}" "${CLIENT_EXTRA[@]}" --seed "$SEED" --warmup "$WARMUP" --report "$T/headless.json" 2>"$T/headless.log" &
HEADLESS_PID=$!
if [ -n "$SIM_CPU" ]; then
  sleep 1 # Let the headless runtime start its workers before checking live masks.
  {
    printf 'role\tproc_status\tname\tCpus_allowed_list\texpected\n'
    AFFINITY_OK=1
    verify_affinity server "$SERVER_PID" "$SERVER_MASK" "$SIM_CPU" || AFFINITY_OK=0
    verify_affinity headless "$HEADLESS_PID" "$CLIENT_MASK" || AFFINITY_OK=0
  } >"$T/affinity.txt"
  [ "$AFFINITY_OK" = 1 ] || die "thread affinity not verified; see $T/affinity.txt and $T/server.log"
  echo "thread affinity verified: $T/affinity.txt"
fi
(
  if [ -z "$SIM_CPU" ]; then
    sleep 1 # Wait for Tokio to create its workers before recording thread affinity.
    { read_affinity "$SERVER_PID"; read_affinity "$HEADLESS_PID"; } >"$T/affinity.txt"
  fi
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

sed '/^\[aoi-\(wire\|pred\)\] /d' "$T/headless.log" >&2
echo "(raw files kept in $T)"
[ -s "$T/headless.json" ] && [ -s "$T/server.json" ] || { echo "missing headless or server report" >&2; exit 1; }
RC=0
node scripts/summarize-loadtest.mjs --headless "$T/headless.json" --server-report "$T/server.json" \
  --aoi-traces "$T" --metrics-dir "$T" --rss "$T/rss.tsv" --out "$OUT" --transport "$TRANSPORT" --bots "$BOTS" \
  --profile "$PROFILE" --sample-interval "$INTERVAL" --pinned "$([ ${#SRV_PIN[@]} -gt 0 ] && echo 1 || echo 0)" \
  --map "$MAP" --mode "$MODE" --server-cpus "${SRV_PIN[2]:-}" --client-cpus "${HL_PIN[2]:-}" \
  "${DRIVER[@]}" "${CHECKS[@]}" || RC=$?
[ "$HL_STATUS" = 0 ] || exit "$HL_STATUS"
exit "$RC"
