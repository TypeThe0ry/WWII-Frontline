#!/usr/bin/env bash
# M1-068b: scripts/loadtest-m0.sh inside a private network namespace whose loopback has `tc netem` applied, so loss / delay /
# jitter only affect the load test (the host's loopback and every other local service stay untouched). Needs root (ip netns, tc).
#
#   scripts/loadtest-netem.sh --netem "delay 25ms 10ms loss 2%" -- --transport ws --count 20 --bots 12 --drive-vehicle --duration 300 --out reports/x.json
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
if ! command -v tc >/dev/null || ! command -v ip >/dev/null; then
  echo "tc / ip missing" >&2
  exit 3
fi
read -r -a netem_args <<< "$NETEM"
NS="lt-netem-$$"
ip netns add "$NS"
trap 'ip netns del "$NS" 2>/dev/null || true' EXIT
ip -n "$NS" link set lo up
ip netns exec "$NS" tc qdisc add dev lo root netem "${netem_args[@]}"
ip netns exec "$NS" tc qdisc show dev lo
LOADTEST_NETEM="$NETEM" ip netns exec "$NS" scripts/loadtest-m0.sh "$@"
