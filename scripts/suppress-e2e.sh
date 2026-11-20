#!/usr/bin/env bash
# Run E1-13 over plain WS, then over WS on loopback in a private 5% loss network namespace.
set -euo pipefail
cd "$(dirname "$0")/../client"
export CUDA_VISIBLE_DEVICES=""
FRONTLINE_SUPPRESS_LINK=plain pnpm exec playwright test -c playwright.suppress.config.ts
# Only code enclosed by unshare can apply a qdisc. The namespace is released on exit.
unshare --net -- "$BASH" -euo pipefail -c '
  ip link set lo up
  tc qdisc add dev lo root netem loss 5% seed 26
  tc qdisc show dev lo
  rc=0
  FRONTLINE_SUPPRESS_LINK=netem pnpm exec playwright test -c playwright.suppress.config.ts || rc=$?
  stats=$(tc -s -j qdisc show dev lo)
  printf "netem (private loopback): %s\n" "$stats"
  node -e '\''if (!JSON.parse(process.argv[1]).some(q => q.root && q.kind === "netem" && q.drops > 0)) process.exit(1)'\'' "$stats"
  exit "$rc"
'
