#!/usr/bin/env bash
# M1-068b: scripts/loadtest-m0.sh inside a private network namespace whose loopback has `tc netem` applied, so loss / delay /
# jitter only affect the load test (the host's loopback and every other local service stay untouched). Needs root (ip netns, tc).
#
#   scripts/loadtest-netem.sh --netem "delay 25ms 10ms loss 2%" -- --transport ws --count 20 --bots 12 --duration 300 --out reports/x.json
#
# The netem string is recorded in the report's `netem` field. One-way delay applies to every packet on lo, so RTT = 2 x delay.
set -euo pipefail
cd "$(dirname "$0")/.."
NETEM=""
while [ $# -gt 0 ]; do
  case "$1" in
    --netem) NETEM="$2"; shift 2 ;;
    --) shift; break ;;
    -h|--help) sed -n '2,8p' "$0"; exit 0 ;;
    *) echo "unknown option: $1 (use -- before the loadtest-m0.sh options)" >&2; exit 2 ;;
  esac
done
[ -n "$NETEM" ] || { echo "--netem \"...\" required" >&2; exit 2; }
[ "$(id -u)" = 0 ] || { echo "needs root for ip netns / tc netem; not faking it" >&2; exit 3; }
command -v tc >/dev/null && command -v ip >/dev/null || { echo "tc / ip missing" >&2; exit 3; }
OUT=""
args=("$@")
for ((i = 0; i < ${#args[@]}; i++)); do [ "${args[i]}" = "--out" ] && OUT="${args[i + 1]}"; done
NS="lt-netem-$$"
ip netns add "$NS"
trap 'ip netns del "$NS" 2>/dev/null || true' EXIT
RC=0
ip netns exec "$NS" bash -c 'ip link set lo up && tc qdisc add dev lo root netem '"$NETEM"' && tc qdisc show dev lo && exec "$@"' _ scripts/loadtest-m0.sh "$@" || RC=$?
if [ -n "$OUT" ] && [ -f "$OUT" ] && command -v jq >/dev/null; then
  jq --arg n "$NETEM" '. + {netem: $n, netem_note: "private netns loopback; delay is one-way per packet (RTT = 2 x delay)"}' "$OUT" >"$OUT.tmp" && mv "$OUT.tmp" "$OUT"
fi
exit $RC
