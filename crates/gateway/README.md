# Frontline gateway

Set `INTERNAL_API_TOKEN` and `TOKEN_SECRET` before starting `frontline-gateway`.
`DATABASE_URL` defaults to `sqlite://gateway.sqlite3` relative to the working directory.
The process creates the database and applies embedded migrations before accepting HTTP
requests. Use a writable persistent directory for SQLite and `TICKET_SIGNING_KEY_FILE`
when retaining live registrations and accepting tickets across restarts. A registration
uses the public key returned when it was created. If that key is replaced, the match
must register again to receive tickets signed under the new key.

The SQLite database stores guest IDs and nicknames, match registrations and heartbeat
metadata, and terminal result bodies. SQLx uses WAL for disk databases and keeps writes
serialized. A completed match leaves the server list in the same transaction that
stores its result. Duplicate result deliveries keep the first payload.

`GET /metrics` serves Prometheus text format. The scrape contains these metric families.

- `gateway_http_request_duration_seconds` is the handler latency histogram.
- `gateway_http_requests_total` counts responses by route template and status class.
  Compute the 5xx ratio by dividing the aggregate 5xx rate by the aggregate request rate.
- `gateway_auth_attempts_total` reports guest login, guest renewal and ticket authentication
  outcomes. An invalid renewal counts as an authentication failure even when the gateway
  successfully creates a fresh guest.
- `gateway_ticket_storage_duration_seconds` covers queue/connection wait (`pool_wait`,
  retained for dashboard compatibility) and the transaction phase (`transaction`). For a
  batched reservation, the latter measures latency to commit, not CPU time per ticket.
- `gateway_ticket_stage_duration_seconds` covers ticket `auth`, `request_parse` (JSON and
  validation, not HTTP framing), `eddsa_sign`, `serialization`, and successful request `log`.

Local checks:

```sh
cargo test -p frontline-gateway --locked
cargo bench -p frontline-gateway --bench ticket_issue --locked
cargo build --release -p frontline-gateway --locked
python3 scripts/gateway-ticket-load.py target/release/frontline-gateway --protocol-version 4
python3 scripts/gateway-ticket-load.py target/release/frontline-gateway --protocol-version 4 \
  --concurrency 500 --runs 7 --report-only --metrics
```

## Match results and player settings

Use the guest session returned by `POST /v1/guest` as `Authorization: Bearer <token>`
for these endpoints.

| Method and path | Response |
| --- | --- |
| `GET /v1/matches/{match_id}/result` | `{ "match_id", "map", "mode", "received_at_ms", "result" }`. Use the match ID from the server list. Unknown matches and matches without a result return `404`. |
| `GET /v1/me/settings` | The authenticated guest's settings object, or `{}` before the first write. |
| `PUT /v1/me/settings` | Replaces the authenticated guest's settings and returns the saved object. Send a JSON object of up to 16 KiB, or `{}` to clear it. Invalid JSON and non-objects return `422`. Oversized bodies return `413`. |

`result` is the existing authenticated server callback payload. The current server
sends `players`, `bots`, `objectives`, `duration_s`, `ticks`, and `rows` at shutdown.
The current server sends empty scoreboard rows. This API queries a single match by ID.
`received_at_ms` is the gateway's first receipt time in Unix milliseconds, or `null`
for rows saved before migration 0002. Retries and replay of spooled results preserve
both the first payload and its receipt time.

## Reserved OAuth endpoints

`GET /v1/oauth/{provider}/start` and `GET /v1/oauth/{provider}/callback`
return `404` by default. `OAUTH_ENABLED=1` exposes `501` placeholders for
`discord` and `google`. The placeholders make no provider requests and issue no tokens.
Guest sign-in remains available. Provider registration is tracked under H-063.
