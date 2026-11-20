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
python3 scripts/gateway-ticket-load.py target/release/frontline-gateway --protocol-version 5
python3 scripts/gateway-ticket-load.py target/release/frontline-gateway --protocol-version 5 \
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

## Moderation API (M2-023)

Set `ADMIN_TOKEN_HASH` to the SHA-256 hex digest of a high-entropy, dedicated administrator token. Set `ADMIN_IP_ALLOWLIST` to a comma-separated list of client CIDRs. Pass the raw token in `Authorization: Bearer …`. Both checks must pass. Omitted configuration disables administrator authorization. Configure `ADMIN_TRUSTED_PROXIES` with the immediate proxy CIDRs when proxies append the preceding hop to `X-Forwarded-For`. The gateway walks the chain from right to left and uses the first untrusted address. Login and ticket ban checks use the same source address policy. The legacy `TRUST_FORWARDED` switch still controls the M1 public rate limiter.

| Method and path | Behavior |
| --- | --- |
| `GET /admin/reports?status=open&limit=50&before=123` | Review queue with descending ID pagination. `status` also accepts `resolved`. |
| `POST /admin/reports/{id}/resolve` | Resolve a report. |
| `POST /admin/ban` | Accepts `subject_kind`, `subject_hash`, optional `ip_cidr`, `reason`, and `duration`. `account` uses the current stable guest/profile ID. `guest` uses the SHA-256 hex digest of a session bearer. `ip` uses the CIDR alone. Accepted durations are `1h`, `24h`, `7d`, and `perm` for identity bans. |
| `DELETE /admin/ban/{id}` | Remove the ban. Use the ID returned at creation. Guest login and ticket exchange return `403` with `expires_at` in Unix milliseconds, or `null` for a permanent ban. Use kick to terminate a live connection. |
| `POST /admin/{kick, mute, force_team, broadcast, map_change}` | Queue a `match_id` and its action parameters. Player actions accept `player_id` from the signed ticket and optional `jti` to pin the target connection. Broadcast accepts `text`, force-team accepts `team` (`0` or `1`), and map-change accepts `map`. |
| `GET /admin/actions/{id}` | Delivery result: `pending`, `applied`, `target_missing`, `unsupported`, or `expired`. Pending commands expire after 60 seconds. The current server executes kick, text mute, and broadcast. Live forced team and map transitions report `unsupported`. |
| `GET /admin/chat/{match_id}` | Bounded in-memory snapshot of recent text chat for an active match. |
| `GET /admin/players/{id}` | Profile, unambiguous match results, and report counts. |
| `GET /admin/servers`, `PUT /admin/servers/{match_id}` | List active registrations including hidden ones and change public discovery with `{ "listed": false }` or `{ "listed": true }`. A client with a known match ID can still request a ticket. |
| `POST /admin/announcements`, `DELETE /admin/announcements/{id}` | Publish plain `text` with optional `duration_s` (default 86,400, maximum 604,800) or retract an announcement. `GET /v1/announcements` returns up to 20 active announcements. |

The existing server bearer authenticates internal report submission and match-scoped replay, chat, and action routes. Registering tickets stores the JTI-to-profile association without changing wire claims. `POST /internal/report` accepts the `match_id`, reporter and target JTIs, reason `0..=3`, optional note, and optional linked text chat. The gateway returns `201` for a new report and `200` for an existing report with the same match, reporter, target and reason. One new report per 60 seconds per profile and ten per match are enforced in SQLite. Replay confirmation uses the accepted report's ID and the exact relative reference `replays/{match_id}_{id}.bin`.

An audit row is written before dispatch. Final status and successful database mutations commit together. Aborted operations may leave a `pending` row for diagnosis. Audit retains structured targets and outcome, with credentials and free text excluded. A startup and periodic sweep removes linked chat at 14 days, administrator source IPs and ban CIDRs at 30 days, and audit rows at 365 days. Voice is outside the evidence pipeline.
