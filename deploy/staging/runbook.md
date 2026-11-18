# Staging runbook (M1-067a draft)

Status: **templates only**. Nothing here has been run against real hosts — domain (H-017), hosts (H-016/H-055),
Cloudflare R2 (H-015) and budget (H-020) are not provided, so deployment is `blocked` on a human.
Domains below are placeholders (`*.example.invalid`).

## Topology
- Match host (≥ 4 physical cores, 16 GB, 1 Gbps): `server` container, host networking.
- Gateway host (2 vCPU / 2 GB): `gateway` + Caddy (HTTPS, Let's Encrypt automatic).
- Static assets: R2 bucket behind CDN. `.fpak` content-hash names, `Cache-Control: public, max-age=31536000, immutable`;
  manifest and `index.html` `no-cache`. Plan with `node tools/deploy/upload-r2.mjs --dir <upload_dir> --budget-bytes N`.

## DNS (examples)
| Name | Type | Target |
|---|---|---|
| `gw.staging.example.invalid` | A | gateway host |
| `match1.staging.example.invalid` | A | match host (WebTransport needs a publicly trusted cert for exactly this name) |
| `cdn.staging.example.invalid` | CNAME | R2 custom domain |

## Firewall
443/tcp on the gateway host; the match UDP range (`SERVER_UDP_PORT_RANGE`) and 443/tcp for the WS fallback on the match host; SSH restricted by source.

## Deploy
1. `docker compose pull && docker compose up -d` on each host with the env vars from `docker-compose.yml`.
2. `deploy/staging/healthcheck.sh` from an external machine.
3. Upload assets (plan first, then the real uploader once credentials exist).

## Certificates
`cert/renew.sh` twice a day (timer). The deploy hook copies the cert and sends SIGHUP to `server`.
**Not implemented yet:** the server's SIGHUP certificate reload (needs `crates/server/src/tls.rs` work) — until then a renewal needs a restart between matches.
`cert/days-left.sh <fullchain.pem>` exits 1 at ≤ 14 days.

## Release / rollback
Bump `min_supported_protocol` in the gateway when `PROTOCOL_VERSION` or `data_hash` changes; old matches finish on the old image (blue/green).
Rollback: point the gateway at the previous image tag for new matches. **Rehearsal not done.**
