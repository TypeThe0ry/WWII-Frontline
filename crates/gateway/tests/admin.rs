//! Admin and report contracts exercised through the Axum router with a manual clock.

use std::time::{Duration, Instant};

use axum::body::{Body, Bytes};
use axum::extract::ConnectInfo;
use axum::http::{header, Method, Request, StatusCode};
use axum::Router;
use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine};
use ed25519_dalek::SigningKey;
use frontline_gateway::admin::AdminConfig;
use frontline_gateway::app::{router, AppState};
use frontline_gateway::clock::Clock;
use frontline_gateway::config::Config;
use frontline_gateway::registry::to_hex;
use frontline_protocol::{dto::TicketClaims, PROTOCOL_VERSION};
use http_body_util::BodyExt;
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use sqlx::{Connection, SqliteConnection};
use tower::ServiceExt;

const START_MS: u64 = 1_800_000_000_000;
const REPORT_INTERVAL_MS: u64 = 60_000;
const DAY_MS: u64 = 86_400_000;
const ADMIN: &str = "test-admin-token";
const WRONG_ADMIN: &str = "test-admin-tokeN";
const INTERNAL: &str = "internal-test-token";
const ALLOWED: &str = "192.0.2.10:8080";
const OUTSIDE: &str = "198.51.100.10:8080";
const PROXY: &str = "203.0.113.10:8080";

struct Reply {
    status: StatusCode,
    body: Bytes,
}

impl Reply {
    #[track_caller]
    fn expect(self, status: StatusCode) -> Self {
        assert_eq!(
            self.status,
            status,
            "{}",
            String::from_utf8_lossy(&self.body)
        );
        self
    }

    #[track_caller]
    fn success(self) -> Self {
        assert!(
            self.status.is_success(),
            "{}: {}",
            self.status,
            String::from_utf8_lossy(&self.body)
        );
        self
    }

    #[track_caller]
    fn json(&self) -> Value {
        serde_json::from_slice(&self.body)
            .unwrap_or_else(|err| panic!("{err}: {}", String::from_utf8_lossy(&self.body)))
    }
}

struct Rig {
    app: Router,
    clock: Clock,
    // Separate from the router's SQLite connection. Keep the temp dir alive until both drop.
    db: SqliteConnection,
    dir: tempfile::TempDir,
}

impl Rig {
    async fn new() -> Self {
        Self::open(tempfile::tempdir().unwrap(), Clock::manual(START_MS)).await
    }

    async fn open(dir: tempfile::TempDir, clock: Clock) -> Self {
        let database_url = format!("sqlite://{}", dir.path().join("gateway.db").display());
        let cfg = Config {
            admin: AdminConfig::parse(
                Some(&sha256_hex(ADMIN)),
                Some("192.0.2.0/24,2001:db8:1::/48"),
                Some("203.0.113.0/24"),
            )
            .unwrap(),
            database_url: database_url.clone(),
            // Admin IP checks must use the trusted proxy CIDRs even with legacy XFF enabled.
            trust_forwarded: true,
            ..Config::test()
        };
        let state = AppState::new(cfg, clock.clone(), SigningKey::from_bytes(&[5; 32]))
            .await
            .unwrap();
        Self {
            app: router(state),
            clock,
            db: SqliteConnection::connect(&database_url).await.unwrap(),
            dir,
        }
    }

    async fn reopen_after(self, elapsed_ms: u64) -> Self {
        let Self {
            app,
            clock,
            db,
            dir,
        } = self;
        drop(app);
        db.close().await.unwrap();
        clock.advance_ms(elapsed_ms);
        Self::open(dir, clock).await
    }

    async fn request(
        &self,
        method: Method,
        path: &str,
        peer: &str,
        bearer: Option<&str>,
        xff: Option<&str>,
        body: Value,
    ) -> Reply {
        let mut req = Request::builder()
            .method(method)
            .uri(path)
            .extension(ConnectInfo(peer.parse::<std::net::SocketAddr>().unwrap()));
        if let Some(token) = bearer {
            req = req.header(header::AUTHORIZATION, format!("Bearer {token}"));
        }
        if let Some(ip) = xff {
            req = req.header("x-forwarded-for", ip);
        }
        let req = if body.is_null() {
            req.body(Body::empty())
        } else {
            req.header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(body.to_string()))
        }
        .unwrap();
        let response = self.app.clone().oneshot(req).await.unwrap();
        let (parts, body) = response.into_parts();
        Reply {
            status: parts.status,
            body: body.collect().await.unwrap().to_bytes(),
        }
    }

    async fn call(&self, method: Method, path: &str, bearer: Option<&str>, body: Value) -> Reply {
        self.request(method, path, ALLOWED, bearer, None, body)
            .await
    }

    async fn admin(&mut self, method: Method, path: &str, body: Value) -> Reply {
        let before = self.audit_count().await;
        self.call(method.clone(), path, None, body.clone())
            .await
            .expect(StatusCode::FORBIDDEN);
        self.assert_audit_since(before, StatusCode::FORBIDDEN).await;
        let reply = self.call(method, path, Some(ADMIN), body).await;
        self.assert_audit_since(before + 1, reply.status).await;
        reply
    }

    async fn audit_count(&mut self) -> i64 {
        sqlx::query_scalar("SELECT COUNT(*) FROM audit_log")
            .fetch_one(&mut self.db)
            .await
            .unwrap()
    }

    async fn assert_audit_since(&mut self, before: i64, status: StatusCode) {
        let rows: Vec<(String, String)> = sqlx::query_as(
            "SELECT result, typeof(result) FROM audit_log ORDER BY id LIMIT -1 OFFSET ?",
        )
        .bind(before)
        .fetch_all(&mut self.db)
        .await
        .unwrap();
        assert_eq!(rows, [(status.as_u16().to_string(), "text".into())]);
    }

    async fn register(&self) -> String {
        self.call(
            Method::POST,
            "/internal/matches/register",
            Some(INTERNAL),
            json!({"name":"Admin test", "map":"carentan", "mode":"conquest", "region":"eu",
                   "tags":[], "version":"0.1", "protocol_version":PROTOCOL_VERSION,
                   "data_hash":u64::MAX, "cap":128, "addr":"play.example",
                   "ws_port":8443, "wt_port":4433}),
        )
        .await
        .expect(StatusCode::OK)
        .json()["match_id"]
            .as_str()
            .unwrap()
            .to_owned()
    }

    async fn heartbeat(&self, match_id: &str) {
        self.call(
            Method::POST,
            &format!("/internal/matches/{match_id}/heartbeat"),
            Some(INTERNAL),
            json!({"humans":0, "bots":0, "phase":"live", "time_left_s":600}),
        )
        .await
        .expect(StatusCode::NO_CONTENT);
    }

    async fn guest(&self, peer: &str) -> Value {
        self.request(Method::POST, "/v1/guest", peer, None, None, json!({}))
            .await
            .expect(StatusCode::OK)
            .json()
    }

    async fn ticket(&self, match_id: &str, token: &str, peer: &str) -> Reply {
        self.request(
            Method::POST,
            &format!("/v1/servers/{match_id}/ticket"),
            peer,
            Some(token),
            None,
            json!({"nick":"Gaius"}),
        )
        .await
    }

    async fn claims(&self, match_id: &str, token: &str, peer: &str) -> TicketClaims {
        let body = self
            .ticket(match_id, token, peer)
            .await
            .expect(StatusCode::OK)
            .json();
        let (claims, _) = body["ticket"].as_str().unwrap().split_once('.').unwrap();
        TicketClaims::decode(&URL_SAFE_NO_PAD.decode(claims).unwrap()).unwrap()
    }

    async fn player(&self, match_id: &str, peer: &str) -> TicketClaims {
        let guest = self.guest(peer).await;
        self.claims(match_id, guest["token"].as_str().unwrap(), peer)
            .await
    }

    async fn report(&self, body: Value) -> Reply {
        self.call(Method::POST, "/internal/report", Some(INTERNAL), body)
            .await
    }

    async fn open_reports(&self) -> Reply {
        self.call(
            Method::GET,
            "/admin/reports?status=open",
            Some(ADMIN),
            Value::Null,
        )
        .await
    }

    async fn actions(&self, match_id: &str, bearer: Option<&str>) -> Reply {
        self.call(
            Method::GET,
            &format!("/internal/matches/{match_id}/actions"),
            bearer,
            Value::Null,
        )
        .await
    }
}

fn sha256_hex(input: &str) -> String {
    format!("{:x}", Sha256::digest(input.as_bytes()))
}

fn report_body(match_id: &str, reporter: &TicketClaims, target: &TicketClaims) -> Value {
    json!({"match_id":match_id, "reporter_jti":to_hex(&reporter.jti),
           "target_jti":to_hex(&target.jti), "reason":2, "note":"Test report"})
}

#[track_caller]
fn id(value: &Value) -> &Value {
    let id = &value["id"];
    assert!(
        id.as_str().is_some_and(|s| !s.is_empty()) || id.is_u64(),
        "missing id in {value}"
    );
    id
}

fn id_path(value: &Value) -> String {
    let id = id(value);
    id.as_str().map_or_else(|| id.to_string(), str::to_owned)
}

// Some endpoints specify the fields of interest without specifying an array or envelope.
fn contains_field(value: &Value, field: &str, expected: &Value) -> bool {
    match value {
        Value::Object(fields) => {
            fields.get(field) == Some(expected)
                || fields
                    .values()
                    .any(|value| contains_field(value, field, expected))
        }
        Value::Array(rows) => rows.iter().any(|row| contains_field(row, field, expected)),
        _ => false,
    }
}

#[tokio::test]
async fn admin_audit_auth_and_proxy_allowlist() {
    let mut r = Rig::new().await;
    const OK: StatusCode = StatusCode::OK;
    const FORBIDDEN: StatusCode = StatusCode::FORBIDDEN;
    let cases = [
        ("direct", ALLOWED, Some(ADMIN), None, OK),
        ("direct IPv6", "[2001:db8:1::a]:8080", Some(ADMIN), None, OK),
        ("missing bearer", ALLOWED, None, None, FORBIDDEN),
        ("wrong bearer", ALLOWED, Some(WRONG_ADMIN), None, FORBIDDEN),
        ("internal bearer", ALLOWED, Some(INTERNAL), None, FORBIDDEN),
        ("outside CIDR", OUTSIDE, Some(ADMIN), None, FORBIDDEN),
        (
            "spoofed XFF",
            OUTSIDE,
            Some(ADMIN),
            Some("192.0.2.10"),
            FORBIDDEN,
        ),
        (
            "ignore XFF from direct client",
            ALLOWED,
            Some(ADMIN),
            Some("198.51.100.10"),
            OK,
        ),
        (
            "trusted proxy without XFF",
            PROXY,
            Some(ADMIN),
            None,
            FORBIDDEN,
        ),
        (
            "trusted proxy with allowed client",
            PROXY,
            Some(ADMIN),
            Some("192.0.2.10"),
            OK,
        ),
        (
            "trusted proxy with denied client",
            PROXY,
            Some(ADMIN),
            Some("198.51.100.10"),
            FORBIDDEN,
        ),
        (
            "two trusted proxy hops",
            PROXY,
            Some(ADMIN),
            Some("192.0.2.10, 203.0.113.11"),
            OK,
        ),
        (
            "spoofed XFF prefix behind a trusted proxy",
            PROXY,
            Some(ADMIN),
            Some("192.0.2.10, 198.51.100.10"),
            FORBIDDEN,
        ),
    ];
    for (case, peer, bearer, xff, expected) in cases {
        let before = r.audit_count().await;
        let reply = r
            .request(
                Method::GET,
                "/admin/reports?status=open",
                peer,
                bearer,
                xff,
                Value::Null,
            )
            .await;
        assert_eq!(
            reply.status,
            expected,
            "{case}: {}",
            String::from_utf8_lossy(&reply.body)
        );
        r.assert_audit_since(before, expected).await;
    }
    for (method, path) in [
        (Method::GET, "/admin/unknown"),
        (Method::OPTIONS, "/admin/reports"),
        (Method::PATCH, "/admin/reports"),
    ] {
        let before = r.audit_count().await;
        r.call(method, path, None, Value::Null)
            .await
            .expect(FORBIDDEN);
        r.assert_audit_since(before, FORBIDDEN).await;
    }
}

#[tokio::test]
async fn admin_audit_unlist_and_announce() {
    let mut r = Rig::new().await;
    let match_id = r.register().await;
    let other = r.register().await;
    let servers = r
        .call(Method::GET, "/v1/servers", None, Value::Null)
        .await
        .expect(StatusCode::OK)
        .json();
    assert!(contains_field(&servers, "match_id", &json!(match_id)));

    r.admin(
        Method::PUT,
        &format!("/admin/servers/{match_id}"),
        json!({"listed":false}),
    )
    .await
    .success();
    let servers = r
        .call(Method::GET, "/v1/servers", None, Value::Null)
        .await
        .expect(StatusCode::OK)
        .json();
    assert!(!contains_field(&servers, "match_id", &json!(match_id)));
    assert!(contains_field(&servers, "match_id", &json!(other)));

    let timed = json!("Gateway maintenance in five minutes");
    let default = json!("Welcome to the test gateway");
    for announcement in [
        json!({"text":timed, "duration_s":60}),
        json!({"text":default}),
    ] {
        r.admin(Method::POST, "/admin/announcements", announcement)
            .await
            .expect(StatusCode::CREATED);
    }
    let live = r
        .call(Method::GET, "/v1/announcements", None, Value::Null)
        .await
        .expect(StatusCode::OK)
        .json();
    assert!(contains_field(&live, "text", &timed));
    assert!(contains_field(&live, "text", &default));
    r.clock.advance_ms(61_000);
    let live = r
        .call(Method::GET, "/v1/announcements", None, Value::Null)
        .await
        .expect(StatusCode::OK)
        .json();
    assert!(!contains_field(&live, "text", &timed));
}

#[tokio::test]
async fn admin_audit_failure_rolls_back_ban() {
    let mut r = Rig::new().await;
    let ban = json!({"subject_kind":"ip", "ip_cidr":"198.51.100.0/24", "reason":"Test ban", "duration":"1h"});
    // Fail initial audit insertion, then fail final-result writes whether done by INSERT or UPDATE.
    for (triggers, cleanup) in [
        (
            "CREATE TRIGGER fail_insert BEFORE INSERT ON audit_log
             BEGIN SELECT RAISE(ABORT, 'audit unavailable'); END;",
            "DROP TRIGGER fail_insert;",
        ),
        (
            "CREATE TRIGGER fail_final_insert BEFORE INSERT ON audit_log WHEN NEW.result = '201'
             BEGIN SELECT RAISE(ABORT, 'audit result unavailable'); END;
             CREATE TRIGGER fail_final_update BEFORE UPDATE ON audit_log WHEN NEW.result = '201'
             BEGIN SELECT RAISE(ABORT, 'audit result unavailable'); END;",
            "DROP TRIGGER fail_final_insert; DROP TRIGGER fail_final_update;",
        ),
    ] {
        sqlx::raw_sql(triggers).execute(&mut r.db).await.unwrap();
        let reply = r
            .call(Method::POST, "/admin/ban", Some(ADMIN), ban.clone())
            .await;
        assert!(
            reply.status.is_server_error(),
            "{}: {}",
            reply.status,
            String::from_utf8_lossy(&reply.body)
        );
        let bans: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM bans")
            .fetch_one(&mut r.db)
            .await
            .unwrap();
        assert_eq!(bans, 0, "ban survived an audit failure");
        sqlx::raw_sql(cleanup).execute(&mut r.db).await.unwrap();
    }
    r.admin(Method::POST, "/admin/ban", ban)
        .await
        .expect(StatusCode::CREATED);
    let bans: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM bans")
        .fetch_one(&mut r.db)
        .await
        .unwrap();
    assert_eq!(bans, 1);
}

#[tokio::test]
async fn admin_audit_ticket_bans_by_subject_and_cidr() {
    let mut r = Rig::new().await;
    let m = r.register().await;
    let guest = r.guest(OUTSIDE).await;
    let token = guest["token"].as_str().unwrap();
    let control = r.guest(ALLOWED).await;
    let control_token = control["token"].as_str().unwrap();
    r.ticket(&m, token, OUTSIDE).await.expect(StatusCode::OK);

    let banned = r.admin(Method::POST, "/admin/ban", json!({
        "subject_kind":"account", "subject_hash":guest["guest_id"], "reason":"Account ban", "duration":"perm",
    })).await.expect(StatusCode::CREATED).json();
    let expiry: Option<i64> =
        sqlx::query_scalar("SELECT expires_at FROM bans WHERE subject_kind = 'account'")
            .fetch_one(&mut r.db)
            .await
            .unwrap();
    assert_eq!(expiry, None);
    r.ticket(&m, token, ALLOWED)
        .await
        .expect(StatusCode::FORBIDDEN);
    r.ticket(&m, control_token, OUTSIDE)
        .await
        .expect(StatusCode::OK);
    let removed = r
        .admin(
            Method::DELETE,
            &format!("/admin/ban/{}", id_path(&banned)),
            Value::Null,
        )
        .await
        .expect(StatusCode::NO_CONTENT);
    assert!(removed.body.is_empty());
    r.ticket(&m, token, OUTSIDE).await.expect(StatusCode::OK);

    id(&r.admin(Method::POST, "/admin/ban", json!({
        "subject_kind":"guest", "subject_hash":sha256_hex(token), "reason":"Guest ban", "duration":"1h",
    })).await.expect(StatusCode::CREATED).json());
    r.ticket(&m, token, ALLOWED)
        .await
        .expect(StatusCode::FORBIDDEN);
    r.ticket(&m, control_token, OUTSIDE)
        .await
        .expect(StatusCode::OK);
    r.clock.advance_ms(3_599_000);
    r.heartbeat(&m).await;
    r.ticket(&m, token, OUTSIDE)
        .await
        .expect(StatusCode::FORBIDDEN);
    r.clock.advance_ms(2_000);
    r.ticket(&m, token, OUTSIDE).await.expect(StatusCode::OK);

    for (cidr, duration, inside, outside) in [
        (
            "198.51.100.0/25",
            "24h",
            "198.51.100.99:8080",
            "198.51.100.128:8080",
        ),
        (
            "2001:db8:2::/48",
            "7d",
            "[2001:db8:2::5]:8080",
            "[2001:db8:3::5]:8080",
        ),
    ] {
        id(&r.admin(Method::POST, "/admin/ban", json!({
            "subject_kind":"ip", "ip_cidr":cidr, "reason":"Network ban", "duration":duration,
        })).await.expect(StatusCode::CREATED).json());
        r.ticket(&m, control_token, inside)
            .await
            .expect(StatusCode::FORBIDDEN);
        r.ticket(&m, control_token, outside)
            .await
            .expect(StatusCode::OK);
    }
}

#[tokio::test]
async fn admin_audit_report_resolution_and_kick_delivery() {
    let mut r = Rig::new().await;
    let m = r.register().await;
    let other = r.register().await;
    let reporter = r.player(&m, ALLOWED).await;
    let target = r.player(&m, OUTSIDE).await;
    let chat = json!([{"player_id":target.player_id, "text":"Message from the reported player"}]);
    let mut report = report_body(&m, &reporter, &target);
    report["chat_excerpt"] = chat.clone();
    for bearer in [None, Some(ADMIN)] {
        r.call(Method::POST, "/internal/report", bearer, report.clone())
            .await
            .expect(StatusCode::UNAUTHORIZED);
    }
    let mut invalid = report.clone();
    invalid["reason"] = json!(4);
    assert!(r.report(invalid).await.status.is_client_error());
    let report = r.report(report).await.expect(StatusCode::CREATED).json();
    let stored: Vec<String> = sqlx::query_scalar("SELECT chat_excerpt FROM reports")
        .fetch_all(&mut r.db)
        .await
        .unwrap();
    assert_eq!(stored.len(), 1);
    assert_eq!(serde_json::from_str::<Value>(&stored[0]).unwrap(), chat);

    let open = r
        .admin(Method::GET, "/admin/reports?status=open", Value::Null)
        .await
        .expect(StatusCode::OK)
        .json();
    let open = open.as_array().unwrap();
    assert_eq!(open.len(), 1);
    assert_eq!(id(&open[0]), id(&report));
    assert!(open[0]["replay_ref"].is_null());

    let report_id = id_path(&report);
    let replay_path = format!("/internal/matches/{m}/reports/{report_id}/replay");
    let replay_ref = format!("replays/{m}_{report_id}.bin");
    let attach = json!({"replay_ref":replay_ref});
    r.call(Method::PUT, &replay_path, Some(ADMIN), attach.clone())
        .await
        .expect(StatusCode::UNAUTHORIZED);
    r.call(Method::PUT, &replay_path, Some(INTERNAL), attach)
        .await
        .expect(StatusCode::NO_CONTENT);
    let persisted: String = sqlx::query_scalar("SELECT replay_ref FROM reports")
        .fetch_one(&mut r.db)
        .await
        .unwrap();
    assert_eq!(persisted, replay_ref);
    let open = r
        .admin(Method::GET, "/admin/reports?status=open", Value::Null)
        .await
        .expect(StatusCode::OK)
        .json();
    assert_eq!(id(&open[0]), id(&report));
    assert_eq!(open[0]["replay_ref"], replay_ref);
    let resolved = r
        .admin(
            Method::POST,
            &format!("/admin/reports/{report_id}/resolve"),
            Value::Null,
        )
        .await
        .expect(StatusCode::NO_CONTENT);
    assert!(resolved.body.is_empty());
    assert_eq!(
        r.admin(Method::GET, "/admin/reports?status=open", Value::Null)
            .await
            .expect(StatusCode::OK)
            .json(),
        json!([])
    );

    let kick = r
        .admin(
            Method::POST,
            "/admin/kick",
            json!({"match_id":m, "player_id":target.player_id}),
        )
        .await
        .expect(StatusCode::ACCEPTED)
        .json();
    r.actions(&m, Some(ADMIN))
        .await
        .expect(StatusCode::UNAUTHORIZED);
    let pending = r
        .actions(&m, Some(INTERNAL))
        .await
        .expect(StatusCode::OK)
        .json();
    assert!(contains_field(&pending, "id", id(&kick)));
    assert!(contains_field(
        &pending,
        "player_id",
        &json!(target.player_id)
    ));
    let other_pending = r
        .actions(&other, Some(INTERNAL))
        .await
        .expect(StatusCode::OK)
        .json();
    assert!(!contains_field(&other_pending, "id", id(&kick)));
    let ack_path = format!("/internal/matches/{m}/actions/{}/ack", id_path(&kick));
    r.call(Method::POST, &ack_path, None, json!({"result":"applied"}))
        .await
        .expect(StatusCode::UNAUTHORIZED);
    r.call(
        Method::POST,
        &ack_path,
        Some(INTERNAL),
        json!({"result":"applied"}),
    )
    .await
    .success();
    let pending = r
        .actions(&m, Some(INTERNAL))
        .await
        .expect(StatusCode::OK)
        .json();
    assert!(!contains_field(&pending, "id", id(&kick)));
}

#[tokio::test]
async fn internal_report_dedup_global_interval_and_per_match_cap() {
    let mut r = Rig::new().await;
    let m = r.register().await;
    let other = r.register().await;
    let session = r.guest(ALLOWED).await;
    let token = session["token"].as_str().unwrap();
    let reporter = r.claims(&m, token, ALLOWED).await;
    let other_target = r.player(&other, OUTSIDE).await;
    let other_reporter = r.claims(&other, token, ALLOWED).await;
    let across_matches = report_body(&other, &other_reporter, &other_target);
    let mut targets = Vec::new();
    for i in 1..=11 {
        targets.push(r.player(&m, &format!("198.51.100.{i}:8080")).await);
    }
    let first = report_body(&m, &reporter, &targets[0]);
    let created = r
        .report(first.clone())
        .await
        .expect(StatusCode::CREATED)
        .json();
    let repeat = r.report(first.clone()).await.expect(StatusCode::OK).json();
    assert_eq!(id(&repeat), id(&created));
    r.report(across_matches.clone())
        .await
        .expect(StatusCode::TOO_MANY_REQUESTS);
    // The interval is per reporter, even when both requests use the same server connection IP.
    r.report(report_body(&m, &targets[0], &reporter))
        .await
        .expect(StatusCode::CREATED);

    r.clock.advance_ms(REPORT_INTERVAL_MS - 1);
    r.heartbeat(&m).await;
    r.report(report_body(&m, &reporter, &targets[1]))
        .await
        .expect(StatusCode::TOO_MANY_REQUESTS);
    // Keep the same in-match JTIs after the ticket's admission TTL has passed.
    for (i, target) in targets[1..10].iter().enumerate() {
        r.clock
            .advance_ms(if i == 0 { 1 } else { REPORT_INTERVAL_MS });
        r.heartbeat(&m).await;
        r.report(report_body(&m, &reporter, target))
            .await
            .expect(StatusCode::CREATED);
    }
    r.clock.advance_ms(REPORT_INTERVAL_MS);
    r.heartbeat(&m).await;
    r.heartbeat(&other).await;
    r.report(report_body(&m, &reporter, &targets[10]))
        .await
        .expect(StatusCode::TOO_MANY_REQUESTS);
    let repeat = r.report(first).await.expect(StatusCode::OK).json();
    assert_eq!(id(&repeat), id(&created));
    r.report(across_matches).await.expect(StatusCode::CREATED);

    let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM reports")
        .fetch_one(&mut r.db)
        .await
        .unwrap();
    assert_eq!(
        count, 12,
        "ten in the first match plus the two independent reports"
    );
}

#[tokio::test]
async fn report_chat_update_is_scoped_and_cannot_revive_expired_evidence() {
    let mut r = Rig::new().await;
    let m = r.register().await;
    let other = r.register().await;
    let reporter = r.player(&m, ALLOWED).await;
    let target = r.player(&m, OUTSIDE).await;
    let report = r
        .report(report_body(&m, &reporter, &target))
        .await
        .expect(StatusCode::CREATED)
        .json();
    let report_id = id_path(&report);
    let path = format!("/internal/matches/{m}/reports/{report_id}/chat");
    let other_path = format!("/internal/matches/{other}/reports/{report_id}/chat");
    let messages = json!([{"player_id":target.player_id,"text":"𐀀".repeat(200)}]);
    let payload = json!({"messages":messages});
    r.call(Method::PUT, &path, Some(ADMIN), payload.clone())
        .await
        .expect(StatusCode::UNAUTHORIZED);
    r.call(Method::PUT, &other_path, Some(INTERNAL), payload.clone())
        .await
        .expect(StatusCode::NOT_FOUND);
    r.call(Method::PUT, &path, Some(INTERNAL), payload.clone())
        .await
        .expect(StatusCode::NO_CONTENT);
    let stored: String = sqlx::query_scalar("SELECT chat_excerpt FROM reports")
        .fetch_one(&mut r.db)
        .await
        .unwrap();
    assert_eq!(serde_json::from_str::<Value>(&stored).unwrap(), messages);
    r.clock.advance_ms(14 * DAY_MS);
    r.call(Method::PUT, &path, Some(INTERNAL), payload)
        .await
        .expect(StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn startup_sweep_applies_report_and_audit_retention() {
    let mut r = Rig::new().await;
    let mut previous_day = 0;
    let stages = [
        (0, &[true][..], &[(0, true)][..]),
        (13, &[true, true], &[(0, true), (13, true)]),
        (15, &[false, true], &[(0, true), (13, true)]),
        (31, &[false, false], &[(0, false), (13, true)]),
        (366, &[false, false], &[(13, false)]),
    ];
    for (day, expected_excerpts, expected_audit) in stages {
        if day != 0 {
            r = r.reopen_after((day - previous_day) * DAY_MS).await;
        }
        previous_day = day;
        // Create a newer row at day 13 so each sweep has a retention control.
        if matches!(day, 0 | 13) {
            let m = r.register().await;
            let reporter = r.player(&m, ALLOWED).await;
            let target = r.player(&m, OUTSIDE).await;
            let mut report = report_body(&m, &reporter, &target);
            report["chat_excerpt"] =
                json!([{"player_id":target.player_id, "text":"Retain this excerpt"}]);
            r.report(report).await.expect(StatusCode::CREATED);
            r.open_reports().await.expect(StatusCode::OK);
        }

        let excerpts: Vec<bool> =
            sqlx::query_scalar("SELECT chat_excerpt IS NOT NULL FROM reports ORDER BY id")
                .fetch_all(&mut r.db)
                .await
                .unwrap();
        assert_eq!(excerpts, expected_excerpts, "report excerpts at day {day}");
        let audit: Vec<(i64, String, Option<String>)> =
            sqlx::query_as("SELECT ts, result, ip FROM audit_log ORDER BY ts")
                .fetch_all(&mut r.db)
                .await
                .unwrap();
        let expected: Vec<_> = expected_audit
            .iter()
            .map(|&(created_day, has_ip)| {
                (
                    (START_MS + created_day * DAY_MS) as i64,
                    "200".to_owned(),
                    has_ip.then(|| "192.0.2.10".to_owned()),
                )
            })
            .collect();
        assert_eq!(audit, expected, "audit rows at day {day}");
    }
}

#[tokio::test]
async fn admin_score_rollup_skips_unbound_simulation_slots() {
    let mut rig = Rig::new().await;
    let guest = rig.guest(ALLOWED).await;
    let token = guest["token"].as_str().unwrap();
    let guest_id = guest["guest_id"].as_str().unwrap();
    for scored in [false, true] {
        let id = rig.register().await;
        rig.heartbeat(&id).await;
        let claims = rig.claims(&id, token, ALLOWED).await;
        let mut payload =
            json!({"rows":[{"player":claims.player_id,"kills":2,"deaths":1,"score":20}]});
        if scored {
            payload["score_summary"] = json!({"players": []});
        }
        rig.call(
            Method::POST,
            &format!("/internal/matches/{id}/result"),
            Some(INTERNAL),
            payload,
        )
        .await
        .expect(StatusCode::NO_CONTENT);
    }
    let player = rig
        .admin(
            Method::GET,
            &format!("/admin/players/{guest_id}"),
            Value::Null,
        )
        .await
        .expect(StatusCode::OK)
        .json();
    assert_eq!(
        player["stats"],
        json!({"matches":1,"kills":2,"deaths":1,"score":20})
    );
}

#[tokio::test]
async fn admin_live_chat_snapshots_replace_and_end_with_match() {
    let mut r = Rig::new().await;
    let m = r.register().await;
    let other = r.register().await;
    let upload = format!("/internal/matches/{m}/chat");
    let chat = format!("/admin/chat/{m}");
    let initial = json!([{"player_id":1, "text":"First snapshot"}]);
    let latest = json!([{"player_id":2, "text":"Replacement snapshot"}]);
    for snapshot in [&initial, &latest] {
        r.call(
            Method::PUT,
            &upload,
            Some(INTERNAL),
            json!({"messages":snapshot}),
        )
        .await
        .expect(StatusCode::NO_CONTENT);
        let view = r
            .admin(Method::GET, &chat, Value::Null)
            .await
            .expect(StatusCode::OK)
            .json();
        assert_eq!(&view, snapshot);
    }
    let unrelated = r
        .admin(Method::GET, &format!("/admin/chat/{other}"), Value::Null)
        .await
        .expect(StatusCode::OK)
        .json();
    assert_eq!(unrelated, json!([]));

    r.call(
        Method::POST,
        &format!("/internal/matches/{m}/result"),
        Some(INTERNAL),
        json!({"rows":[]}),
    )
    .await
    .expect(StatusCode::NO_CONTENT);
    r.admin(Method::GET, &chat, Value::Null)
        .await
        .expect(StatusCode::NOT_FOUND);
    r.call(
        Method::PUT,
        &upload,
        Some(INTERNAL),
        json!({"messages":latest}),
    )
    .await
    .expect(StatusCode::GONE);
}

fn percentile(samples: &[Duration], pct: usize) -> Duration {
    samples[(samples.len() * pct).div_ceil(100) - 1]
}

#[tokio::test]
#[ignore = "timing probe: run in release mode with --ignored --nocapture --test-threads=1"]
async fn admin_auth_denial_timings() {
    let mut r = Rig::new().await;
    let mut wrong_token = Vec::with_capacity(1000);
    let mut denied_ip = Vec::with_capacity(1000);
    let path = "/admin/reports?status=open";
    for _ in 0..1000 {
        for (peer, token, samples) in [
            (ALLOWED, WRONG_ADMIN, &mut wrong_token),
            (OUTSIDE, ADMIN, &mut denied_ip),
        ] {
            let start = Instant::now();
            let reply = r
                .request(Method::GET, path, peer, Some(token), None, Value::Null)
                .await;
            samples.push(start.elapsed());
            reply.expect(StatusCode::FORBIDDEN);
        }
    }
    assert_eq!(r.audit_count().await, 2000);
    wrong_token.sort_unstable();
    denied_ip.sort_unstable();
    let p95_wrong = percentile(&wrong_token, 95);
    let p95_ip = percentile(&denied_ip, 95);
    eprintln!("1000 requests per case: wrong bearer p95 {p95_wrong:?}, disallowed IP p95 {p95_ip:?}, difference {:?}", p95_wrong.abs_diff(p95_ip));
}

#[tokio::test]
#[ignore = "timing probe: run in release mode with --ignored --nocapture --test-threads=1"]
async fn admin_open_reports_p95_with_1000_rows() {
    let mut r = Rig::new().await;
    let m = r.register().await;
    let mut players = Vec::with_capacity(110);
    for i in 1..=110 {
        players.push(r.player(&m, &format!("198.51.100.{i}:8080")).await);
    }
    let (targets, reporters) = players.split_at(10);
    for target in targets {
        r.heartbeat(&m).await;
        for reporter in reporters {
            r.report(report_body(&m, reporter, target))
                .await
                .expect(StatusCode::CREATED);
        }
        r.clock.advance_ms(REPORT_INTERVAL_MS);
    }
    let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM reports")
        .fetch_one(&mut r.db)
        .await
        .unwrap();
    assert_eq!(count, 1000);
    let mut samples = Vec::with_capacity(100);
    for _ in 0..100 {
        let start = Instant::now();
        let reply = r.open_reports().await;
        samples.push(start.elapsed());
        assert!(!reply
            .expect(StatusCode::OK)
            .json()
            .as_array()
            .unwrap()
            .is_empty());
    }
    samples.sort_unstable();
    eprintln!(
        "GET /admin/reports?status=open with 1000 open rows: p95 {:?} over 100 calls",
        percentile(&samples, 95)
    );
}
