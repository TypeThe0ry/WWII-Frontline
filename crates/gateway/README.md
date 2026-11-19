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

Local checks:

```sh
cargo test -p frontline-gateway --locked
cargo bench -p frontline-gateway --bench ticket_issue --locked
cargo build --release -p frontline-gateway --locked
python3 scripts/gateway-ticket-load.py target/release/frontline-gateway --protocol-version 4
```
