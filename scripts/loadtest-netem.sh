#!/usr/bin/env bash
# M1-068b: scripts/loadtest-m0.sh inside a private network namespace whose loopback has `tc netem` applied, so loss / delay /
# jitter only affect the load test (the host's loopback and every other local service stay untouched). Needs root (ip netns, tc).
#
#   scripts/loadtest-netem.sh --netem "delay 25ms 10ms loss 2%" -- --transport ws --count 20 --bots 12 --drive-vehicle --duration 300 --out reports/x.json
#   scripts/loadtest-netem.sh --netem "loss 5% seed 26" --c4 wt  # M1-026b, 600 s (override FRONTLINE_C4_SOAK_SECS)
#
# The netem string is recorded in the report's `netem` field. One-way delay applies to every packet on lo, so RTT = 2 x delay.
set -euo pipefail
cd "$(dirname "$0")/.."
NETEM=""
C4=""
while [ $# -gt 0 ]; do
  case "$1" in
    --netem) NETEM="$2"; shift 2 ;;
    --c4) C4="$2"; shift 2 ;;
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
if [ -n "$C4" ]; then
  case "$C4" in ws|wt) ;; *) echo "--c4 requires ws or wt" >&2; exit 2 ;; esac
  [ "$#" -eq 0 ] || { echo "--c4 does not take loadtest-m0 options" >&2; exit 2; }
  command -v node >/dev/null || { echo "--c4 requires node" >&2; exit 3; }
  export CARGO_INCREMENTAL=0 CARGO_PROFILE_DEV_DEBUG=0 CARGO_PROFILE_TEST_DEBUG=0
  export CARGO_TARGET_DIR="${CARGO_TARGET_DIR:-$PWD/target}"
  [[ "$CARGO_TARGET_DIR" = /* ]] || CARGO_TARGET_DIR="$PWD/$CARGO_TARGET_DIR"
  cargo build --locked -p frontline-headless-client --example c4_probe
  cargo test --locked -p frontline-server --lib --no-run
fi
read -r -a netem_args <<< "$NETEM"
NS="lt-netem-$$"
ip netns add "$NS"
trap 'ip netns del "$NS" 2>/dev/null || true' EXIT
ip -n "$NS" link set lo up
ip netns exec "$NS" tc qdisc add dev lo root netem "${netem_args[@]}"
ip netns exec "$NS" tc qdisc show dev lo
if [ -n "$C4" ]; then
  status=0
  FRONTLINE_C4_PROBE="$CARGO_TARGET_DIR/debug/examples/c4_probe" \
    FRONTLINE_C4_TRANSPORT="$C4" FRONTLINE_C4_SOAK_SECS="${FRONTLINE_C4_SOAK_SECS:-600}" \
    ip netns exec "$NS" cargo test --locked --offline -p frontline-server --lib \
      c4_over_real_listeners -- --ignored --nocapture || status=$?
  stats=$(ip netns exec "$NS" tc -s -j qdisc show dev lo)
  printf 'netem stats (%s): %s\n' "$C4" "$stats"
  [ "$status" -eq 0 ] || exit "$status"
  node -e '
    const q = JSON.parse(process.argv[1]).find(q => q.kind === "netem" && q.root);
    if (!(q?.drops > 0)) { console.error("No observed netem drops"); process.exit(1); }
    const pct = (100 * q.drops / (q.packets + q.drops)).toFixed(3);
    console.log("netem: sent=%d, drops=%d, drop_pct=%s", q.packets, q.drops, pct);
  ' "$stats"
else
  LOADTEST_NETEM="$NETEM" ip netns exec "$NS" scripts/loadtest-m0.sh "$@"
fi
