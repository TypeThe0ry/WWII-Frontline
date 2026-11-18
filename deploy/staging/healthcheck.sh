#!/usr/bin/env sh
# External probe: gateway /healthz and a Range request against a published .fpak (WT/WS handshakes are exercised by the browser tests).
set -eu
: "${GATEWAY_DOMAIN:?}"
curl -fsS "https://$GATEWAY_DOMAIN/healthz" | grep -qx ok && echo "gateway ok"
if [ -n "${FPAK_URL:-}" ]; then
  code=$(curl -s -o /dev/null -w '%{http_code}' -H 'Range: bytes=0-15' "$FPAK_URL")
  [ "$code" = 206 ] && echo "fpak range ok" || { echo "fpak range failed: $code" >&2; exit 1; }
fi
