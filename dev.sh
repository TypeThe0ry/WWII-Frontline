#!/usr/bin/env bash
# M1-066: One-command local development stack (gateway + match server + client dev server).
# Usage: ./dev.sh {up|down|status|logs|reset|check} [options]
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
cd "$ROOT"

STATE_DIR="${FRONTLINE_DEV_STATE_DIR:-$ROOT/.dev-stack}"
CERT_DIR="${FRONTLINE_DEV_CERT_DIR:-$ROOT/dev-cert}"

GATEWAY_PORT="${GATEWAY_PORT:-8080}"
WS_PORT="${FRONTLINE_WS_PORT:-7000}"
WT_PORT="${FRONTLINE_WT_PORT:-7001}"
METRICS_PORT="${FRONTLINE_METRICS_PORT:-9100}"
CLIENT_PORT="${CLIENT_PORT:-5173}"
MAP_ID="${FRONTLINE_MAP:-m0}"
MODE="${FRONTLINE_MODE:-sandbox}"
BOTS="${FRONTLINE_BOTS:-0}"
PROFILE="${FRONTLINE_PROFILE:-debug}" # debug | release
NO_BUILD="${FRONTLINE_NO_BUILD:-0}"
FOREGROUND=0

usage() {
  cat <<'EOF'
Usage: ./dev.sh <command> [options]

Commands:
  up          Start gateway, match server, and client Vite dev server
  down        Stop all stack processes started by dev.sh
  status      Show process status and gateway/server health
  logs [svc]  Show logs (svc: gateway | server | client, default: all)
  reset       Stop stack and remove .dev-stack/ and dev-cert/ state
  check       Run the end-to-end self-check script (tools/dev/check-dev-stack.sh)

Options for 'up':
  --foreground   Run in foreground and stop on Ctrl-C
  --release      Build/use target/release binaries instead of target/debug
  --no-build     Skip cargo/wasm-pack builds if binaries already exist
  -h, --help     Show this help
EOF
}

log() { printf '[dev-stack] %s\n' "$*"; }
die() { printf '[dev-stack] ERROR: %s\n' "$*" >&2; exit 1; }

need_cmd() {
  local c
  for c in "$@"; do
    command -v "$c" >/dev/null 2>&1 || die "missing required command: $c"
  done
}

pid_alive() {
  local pid="$1"
  [ -n "$pid" ] && kill -0 "$pid" 2>/dev/null
}

read_pid() {
  local f="$STATE_DIR/$1.pid"
  [ -f "$f" ] && cat "$f" 2>/dev/null || true
}

port_in_use() {
  local port="$1"
  if command -v ss >/dev/null 2>&1; then
    ss -H -ltn "sport = :$port" 2>/dev/null | grep -q . && return 0
    ss -H -lun "sport = :$port" 2>/dev/null | grep -q . && return 0
  elif command -v lsof >/dev/null 2>&1; then
    lsof -iTCP:"$port" -sTCP:LISTEN >/dev/null 2>&1 && return 0
    lsof -iUDP:"$port" >/dev/null 2>&1 && return 0
  fi
  return 1
}

check_ports_free() {
  local busy=0 p
  for p in "$GATEWAY_PORT" "$WS_PORT" "$WT_PORT" "$METRICS_PORT" "$CLIENT_PORT"; do
    if port_in_use "$p"; then
      printf '[dev-stack] port %s is already in use\n' "$p" >&2
      busy=1
    fi
  done
  [ "$busy" -eq 0 ] || die "free occupied ports or override via GATEWAY_PORT/FRONTLINE_WS_PORT/FRONTLINE_WT_PORT/FRONTLINE_METRICS_PORT/CLIENT_PORT"
}

gen_hex_secret() {
  if command -v openssl >/dev/null 2>&1; then
    openssl rand -hex 24
  else
    od -An -tx1 -N24 /dev/urandom | tr -d ' \n'
  fi
}

ensure_secrets() {
  mkdir -p "$STATE_DIR"
  local env_file="$STATE_DIR/env"
  if [ -z "${INTERNAL_API_TOKEN:-}" ] || [ -z "${TOKEN_SECRET:-}" ]; then
    if [ -f "$env_file" ]; then
      # shellcheck disable=SC1090
      . "$env_file"
    fi
  fi
  INTERNAL_API_TOKEN="${INTERNAL_API_TOKEN:-$(gen_hex_secret)}"
  TOKEN_SECRET="${TOKEN_SECRET:-$(gen_hex_secret)}"
  umask 077
  cat >"$env_file" <<EOF
INTERNAL_API_TOKEN=$INTERNAL_API_TOKEN
TOKEN_SECRET=$TOKEN_SECRET
EOF
  export INTERNAL_API_TOKEN TOKEN_SECRET
}

ensure_cert() {
  local cert="$CERT_DIR/dev-cert.pem"
  local key="$CERT_DIR/dev-key.pem"
  local sha="$CERT_DIR/dev-cert.sha256"
  local regen=0

  if [ ! -f "$cert" ] || [ ! -f "$key" ] || [ ! -f "$sha" ]; then
    regen=1
  elif command -v openssl >/dev/null 2>&1; then
    # Regenerate if certificate expires within 24 hours (browsers cap WT self-signed certs at 14 days).
    if ! openssl x509 -in "$cert" -noout -checkend 86400 >/dev/null 2>&1; then
      log "dev certificate expires within 24h; regenerating"
      regen=1
    fi
  fi

  if [ "$regen" -eq 1 ]; then
    log "generating 13-day WebTransport dev certificate in $CERT_DIR"
    local gen_bin="$ROOT/target/$PROFILE/gen_dev_cert"
    if [ -x "$gen_bin" ]; then
      "$gen_bin" --out "$CERT_DIR" --days 13 >/dev/null
    else
      "$ROOT/tools/dev/gen-dev-cert.sh" --out "$CERT_DIR" --days 13 >/dev/null
    fi
  fi
}

ensure_wasm() {
  local pkg_js="$ROOT/crates/sim-wasm/pkg/frontline_sim_wasm.js"
  local pkg_wasm="$ROOT/crates/sim-wasm/pkg/frontline_sim_wasm_bg.wasm"
  if [ ! -f "$pkg_js" ] || [ ! -f "$pkg_wasm" ]; then
    need_cmd wasm-pack
    log "building crates/sim-wasm (wasm-release)..."
    wasm-pack build "$ROOT/crates/sim-wasm" --target web --profile wasm-release
  fi
}

ensure_bins() {
  local gw_bin="$ROOT/target/$PROFILE/frontline-gateway"
  local srv_bin="$ROOT/target/$PROFILE/frontline-server"
  local cert_bin="$ROOT/target/$PROFILE/gen_dev_cert"
  if [ "$NO_BUILD" = "1" ] && [ -x "$gw_bin" ] && [ -x "$srv_bin" ] && [ -x "$cert_bin" ]; then
    return 0
  fi
  local flags=(-p frontline-gateway -p frontline-server --bin frontline-gateway --bin frontline-server --bin gen_dev_cert)
  if [ "$PROFILE" = "release" ]; then
    flags=(--release "${flags[@]}")
  fi
  log "building Rust binaries ($PROFILE)..."
  cargo build --locked "${flags[@]}"
}

wait_http_200() {
  local url="$1" name="$2" timeout_s="${3:-30}"
  local i=0
  while [ "$i" -lt "$((timeout_s * 5))" ]; do
    if curl -fsS --max-time 2 "$url" >/dev/null 2>&1; then
      return 0
    fi
    sleep 0.2
    i=$((i + 1))
  done
  die "$name did not become ready at $url within ${timeout_s}s (see $STATE_DIR/*.log)"
}

wait_server_registered() {
  local url="http://127.0.0.1:${GATEWAY_PORT}/v1/servers" timeout_s="${1:-20}"
  local i=0
  while [ "$i" -lt "$((timeout_s * 5))" ]; do
    if curl -fsS --max-time 2 "$url" 2>/dev/null | grep -q '"match_id"'; then
      return 0
    fi
    sleep 0.2
    i=$((i + 1))
  done
  die "match server did not register with gateway at $url within ${timeout_s}s (see $STATE_DIR/server.log)"
}

stop_service() {
  local name="$1"
  local pid_file="$STATE_DIR/$name.pid"
  local pid
  pid="$(read_pid "$name")"
  if [ -n "$pid" ] && pid_alive "$pid"; then
    log "stopping $name (pid $pid)"
    # Processes are started in their own session via setsid when available.
    kill -TERM -- "-$pid" 2>/dev/null || kill -TERM "$pid" 2>/dev/null || true
    local i=0
    while pid_alive "$pid" && [ "$i" -lt 25 ]; do
      sleep 0.2
      i=$((i + 1))
    done
    if pid_alive "$pid"; then
      kill -KILL -- "-$pid" 2>/dev/null || kill -KILL "$pid" 2>/dev/null || true
    fi
  fi
  rm -f "$pid_file"
}

spawn_bg() {
  local name="$1"
  shift
  local log_file="$STATE_DIR/$name.log"
  local pid_file="$STATE_DIR/$name.pid"
  if command -v setsid >/dev/null 2>&1; then
    setsid "$@" >"$log_file" 2>&1 < /dev/null &
  else
    nohup "$@" >"$log_file" 2>&1 < /dev/null &
  fi
  echo "$!" >"$pid_file"
}

cmd_down() {
  stop_service client
  stop_service server
  stop_service gateway
}

cmd_up() {
  while [ $# -gt 0 ]; do
    case "$1" in
      --foreground) FOREGROUND=1 ;;
      --release) PROFILE="release" ;;
      --no-build) NO_BUILD=1 ;;
      -h|--help) usage; exit 0 ;;
      *) die "unknown option for up: $1" ;;
    esac
    shift
  done

  need_cmd cargo pnpm node python3 curl

  if pid_alive "$(read_pid gateway)" || pid_alive "$(read_pid server)" || pid_alive "$(read_pid client)"; then
    die "dev stack already running (run './dev.sh down' or './dev.sh status')"
  fi

  check_ports_free
  ensure_secrets
  ensure_bins
  ensure_cert
  ensure_wasm

  log "syncing map/wasm assets for client..."
  pnpm -C "$ROOT/client" sync-map "$MAP_ID" >/dev/null

  mkdir -p "$STATE_DIR/spool"
  local gw_bin="$ROOT/target/$PROFILE/frontline-gateway"
  local srv_bin="$ROOT/target/$PROFILE/frontline-server"

  log "starting gateway on 127.0.0.1:${GATEWAY_PORT}..."
  env \
    GATEWAY_BIND="127.0.0.1:${GATEWAY_PORT}" \
    TICKET_SIGNING_KEY_FILE="$STATE_DIR/ticket-key.bin" \
    ALLOWED_ORIGINS="http://127.0.0.1:${CLIENT_PORT},http://localhost:${CLIENT_PORT}" \
    RUST_LOG="${RUST_LOG:-info}" \
    "$gw_bin" &
  # Re-launch via spawn_bg for clean session/log handling
  kill "$!" 2>/dev/null || true
  wait "$!" 2>/dev/null || true

  spawn_bg gateway env \
    GATEWAY_BIND="127.0.0.1:${GATEWAY_PORT}" \
    TICKET_SIGNING_KEY_FILE="$STATE_DIR/ticket-key.bin" \
    ALLOWED_ORIGINS="http://127.0.0.1:${CLIENT_PORT},http://localhost:${CLIENT_PORT}" \
    RUST_LOG="${RUST_LOG:-info}" \
    "$gw_bin"
  wait_http_200 "http://127.0.0.1:${GATEWAY_PORT}/healthz" "gateway" 20

  log "starting match server (map=${MAP_ID}, mode=${MODE}, ws=${WS_PORT}, wt=${WT_PORT})..."
  spawn_bg server env \
    GATEWAY_URL="http://127.0.0.1:${GATEWAY_PORT}" \
    "$srv_bin" \
    --map "$MAP_ID" \
    --mode "$MODE" \
    --bots "$BOTS" \
    --ws-port "$WS_PORT" \
    --wt-port "$WT_PORT" \
    --metrics-port "$METRICS_PORT" \
    --cert "$CERT_DIR/dev-cert.pem" \
    --key "$CERT_DIR/dev-key.pem" \
    --data-dir "$ROOT/data" \
    --public-addr "127.0.0.1" \
    --server-name "Local Dev" \
    --spool-dir "$STATE_DIR/spool"
  wait_server_registered 20

  log "starting client Vite dev server on 127.0.0.1:${CLIENT_PORT}..."
  spawn_bg client env \
    FRONTLINE_GATEWAY_TARGET="http://127.0.0.1:${GATEWAY_PORT}" \
    FRONTLINE_WS_TARGET="ws://127.0.0.1:${WS_PORT}" \
    pnpm -C "$ROOT/client" exec vite --host 127.0.0.1 --port "$CLIENT_PORT" --strictPort
  wait_http_200 "http://127.0.0.1:${CLIENT_PORT}/" "client dev server" 30

  local certhash
  certhash="$(awk -F= '/^hex=/{print $2}' "$CERT_DIR/dev-cert.sha256" 2>/dev/null || true)"

  printf '\n'
  log "stack is UP:"
  printf '  client (lobby) : http://127.0.0.1:%s/\n' "$CLIENT_PORT"
  printf '  client (direct): http://127.0.0.1:%s/?server=same&scene=vegetation&debug=1\n' "$CLIENT_PORT"
  printf '  gateway        : http://127.0.0.1:%s/healthz\n' "$GATEWAY_PORT"
  printf '  match server   : ws://127.0.0.1:%s  |  https://127.0.0.1:%s/wt\n' "$WS_PORT" "$WT_PORT"
  printf '  metrics        : http://127.0.0.1:%s/metrics\n' "$METRICS_PORT"
  printf '  wt certhash    : %s\n' "$certhash"
  printf '  logs           : %s/{gateway,server,client}.log\n' "$STATE_DIR"
  printf '\n'

  if [ "$FOREGROUND" -eq 1 ]; then
    trap 'cmd_down; exit 0' INT TERM
    log "running in foreground; press Ctrl-C to stop"
    while true; do
      for svc in gateway server client; do
        local p
        p="$(read_pid "$svc")"
        if ! pid_alive "$p"; then
          die "$svc exited unexpectedly (check $STATE_DIR/$svc.log)"
        fi
      done
      sleep 2
    done
  fi
}

cmd_status() {
  local svc p state
  for svc in gateway server client; do
    p="$(read_pid "$svc")"
    if pid_alive "$p"; then
      state="running (pid $p)"
    else
      state="stopped"
    fi
    printf '%-8s : %s\n' "$svc" "$state"
  done
  if curl -fsS --max-time 2 "http://127.0.0.1:${GATEWAY_PORT}/healthz" >/dev/null 2>&1; then
    printf 'healthz  : ok\n'
    printf 'servers  : %s\n' "$(curl -fsS --max-time 2 "http://127.0.0.1:${GATEWAY_PORT}/v1/servers" 2>/dev/null || echo '[]')"
  fi
}

cmd_logs() {
  local svc="${1:-all}"
  if [ "$svc" = "all" ]; then
    local f
    for f in gateway server client; do
      if [ -f "$STATE_DIR/$f.log" ]; then
        printf '=== %s ===\n' "$f"
        tail -n 40 "$STATE_DIR/$f.log"
      fi
    done
  elif [ -f "$STATE_DIR/$svc.log" ]; then
    tail -n 80 "$STATE_DIR/$svc.log"
  else
    die "no log file for '$svc' in $STATE_DIR"
  fi
}

cmd_reset() {
  cmd_down
  rm -rf "$STATE_DIR" "$CERT_DIR"
  log "reset complete (removed $STATE_DIR and $CERT_DIR)"
}

cmd_check() {
  exec "$ROOT/tools/dev/check-dev-stack.sh" "$@"
}

case "${1:-}" in
  up) shift; cmd_up "$@" ;;
  down) cmd_down ;;
  status) cmd_status ;;
  logs) shift; cmd_logs "$@" ;;
  reset) cmd_reset ;;
  check) shift; cmd_check "$@" ;;
  -h|--help|"") usage ;;
  *) die "unknown command: $1" ;;
esac
