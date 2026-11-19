# Local Development Stack (M1-066)

## Recommended: Native One-Command Dev Stack (`./dev.sh`)

From the repository root:

```sh
./dev.sh up       # starts gateway (8080), match server (7000/7001/9100), and Vite dev server (5173)
./dev.sh status   # shows PIDs, /healthz, and registered match servers
./dev.sh logs     # prints recent logs for gateway, server, and client
./dev.sh down     # stops all stack processes
./dev.sh reset    # stops stack and removes .dev-stack/ and dev-cert/
./dev.sh check    # runs tools/dev/check-dev-stack.sh end-to-end verification
```

### Ports & Endpoints (Native)

| Service | Port / Protocol | Purpose |
| --- | --- | --- |
| `client` (Vite) | `127.0.0.1:5173` (TCP HTTP) | Lobby & game client; proxies `/v1` → gateway and `/game-ws` → server |
| `gateway` | `127.0.0.1:8080` (TCP HTTP) | `/healthz`, `/v1/guest`, `/v1/servers`, `/v1/servers/{id}/ticket` |
| `server` (WS) | `127.0.0.1:7000` (TCP WS) | WebSocket fallback & `/game-ws` proxy target |
| `server` (WT) | `127.0.0.1:7001` (UDP QUIC) | WebTransport endpoint (`/wt`) using 13-day `dev-cert` |
| `server` (Metrics) | `127.0.0.1:9100` (TCP HTTP) | Prometheus `/metrics` |

### WebTransport Certificate (`serverCertificateHashes`)

`./dev.sh up` automatically generates a 13-day ECDSA P-256 self-signed certificate in `dev-cert/` (and regenerates it if it expires within 24 hours). The SHA-256 fingerprint is written to `dev-cert/dev-cert.sha256` and printed at startup. For direct WT testing, pass `?certhash=<hex>` to the client URL.

## Optional: Docker Compose Template (`deploy/dev/docker-compose.yml`)

Containerized templates for `gateway`, `server`, and `static` (`nginx:1.27-alpine`) are provided in `deploy/dev/`:

```sh
tools/dev/gen-dev-cert.sh --out dev-cert --days 13
pnpm -C client build
cp deploy/dev/.env.example deploy/dev/.env   # edit tokens
docker compose -f deploy/dev/docker-compose.yml --env-file deploy/dev/.env up --build
```

> **Note:** Docker Desktop on macOS/Windows can have unstable UDP/QUIC port forwarding for WebTransport (`7001/udp`). Prefer `./dev.sh up` on macOS/Windows or when iterating on client/server code.
