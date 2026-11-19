#!/usr/bin/env bash
# M1-066: Self-check for the one-command local dev stack (./dev.sh).
# Verifies gateway healthz, match server registration, guest+ticket issuance,
# server /metrics, Vite dev server + /v1 proxy, dev-cert sha256, and reset idempotency.
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "$ROOT"

GATEWAY_PORT="${GATEWAY_PORT:-18080}"
WS_PORT="${FRONTLINE_WS_PORT:-17000}"
WT_PORT="${FRONTLINE_WT_PORT:-17001}"
METRICS_PORT="${FRONTLINE_METRICS_PORT:-19100}"
CLIENT_PORT="${CLIENT_PORT:-15173}"
CERT_DIR="${FRONTLINE_DEV_CERT_DIR:-$ROOT/.dev-stack-check-cert}"
STATE_DIR="${FRONTLINE_DEV_STATE_DIR:-$ROOT/.dev-stack-check}"

export GATEWAY_PORT FRONTLINE_WS_PORT="$WS_PORT" FRONTLINE_WT_PORT="$WT_PORT" \
  FRONTLINE_METRICS_PORT="$METRICS_PORT" CLIENT_PORT \
  FRONTLINE_DEV_CERT_DIR="$CERT_DIR" FRONTLINE_DEV_STATE_DIR="$STATE_DIR"

cleanup() {
  ./dev.sh down >/dev/null 2>&1 || true
  rm -rf "$CERT_DIR" "$STATE_DIR"
}
trap cleanup EXIT

log() { printf '[dev-stack-check] %s\n' "$*"; }
fail() { printf '[dev-stack-check] FAIL: %s\n' "$*" >&2; exit 1; }

assert_contains() {
  local haystack="$1" needle="$2" label="$3"
  printf '%s' "$haystack" | grep -q -- "$needle" || fail "$label (expected '$needle' in: $haystack)"
}

verify_round() {
  local round="$1"
  local t0 t1
  t0="$(date +%s)"
  log "round $round: starting stack via ./dev.sh up"
  ./dev.sh up

  t1="$(date +%s)"
  log "round $round: stack up in $((t1 - t0))s"

  local healthz
  healthz="$(curl -fsS "http://127.0.0.1:${GATEWAY_PORT}/healthz")"
  assert_contains "$healthz" "ok" "gateway /healthz"
  log "round $round: gateway /healthz -> $healthz"

  local servers
  servers="$(curl -fsS "http://127.0.0.1:${GATEWAY_PORT}/v1/servers")"
  assert_contains "$servers" '"match_id":' "gateway /v1/servers match_id"

  local match_id
  match_id="$(printf '%s' "$servers" | node -e "const d=JSON.parse(require('fs').readFileSync(0,'utf8')); const arr=d.servers??d; if(!arr.length) process.exit(1); console.log(arr[0].match_id);")" \
    || fail "could not parse match_id from /v1/servers: $servers"
  log "round $round: registered match_id=$match_id"

  local guest_json token
  guest_json="$(curl -fsS -X POST "http://127.0.0.1:${GATEWAY_PORT}/v1/guest" -H 'Content-Type: application/json' -d '{}')"
  token="$(printf '%s' "$guest_json" | node -e "const d=JSON.parse(require('fs').readFileSync(0,'utf8')); if(!d.token) process.exit(1); console.log(d.token);")" \
    || fail "guest auth failed: $guest_json"

  local ticket_json
  ticket_json="$(curl -fsS -X POST "http://127.0.0.1:${GATEWAY_PORT}/v1/servers/${match_id}/ticket" -H "Authorization: Bearer $token" -H 'Content-Type: application/json' -d '{"nick":"DevCheck"}')"
  assert_contains "$ticket_json" '"ticket":' "gateway join ticket"
  assert_contains "$ticket_json" "\"ws_port\":${WS_PORT}" "ticket ws_port"
  assert_contains "$ticket_json" "\"wt_port\":${WT_PORT}" "ticket wt_port"
  log "round $round: guest + ticket issued OK"

  local metrics
  metrics="$(curl -fsS "http://127.0.0.1:${METRICS_PORT}/metrics")"
  assert_contains "$metrics" "server_" "server /metrics"
  log "round $round: server /metrics OK"

  local index_html
  index_html="$(curl -fsS "http://127.0.0.1:${CLIENT_PORT}/")"
  assert_contains "$index_html" "<html" "client Vite /"
  local proxied_servers
  proxied_servers="$(curl -fsS "http://127.0.0.1:${CLIENT_PORT}/v1/servers")"
  assert_contains "$proxied_servers" "$match_id" "client Vite /v1/servers proxy"
  log "round $round: client Vite / and /v1 proxy OK"

  local sha_file="$CERT_DIR/dev-cert.sha256"
  [ -f "$sha_file" ] || fail "missing $sha_file"
  grep -Eq '^hex=[0-9a-f]{64}$' "$sha_file" || fail "invalid dev-cert.sha256 format"
  log "round $round: dev-cert.sha256 OK"

  ./dev.sh status >/dev/null
}

./dev.sh reset >/dev/null 2>&1 || true
verify_round 1
log "testing ./dev.sh reset + re-up idempotency..."
./dev.sh reset
export FRONTLINE_NO_BUILD=1
verify_round 2
./dev.sh down
log "ALL CHECKS PASSED (2 rounds + reset idempotency)"
