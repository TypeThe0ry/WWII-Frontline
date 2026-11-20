//! Integration coverage for guest settings, OAuth placeholders, and settings CORS.

use axum::body::Body;
use axum::http::{header, Method, Request, StatusCode};
use axum::response::Response;
use axum::Router;
use ed25519_dalek::SigningKey;
use frontline_gateway::app::{router, AppState};
use frontline_gateway::auth::token::TOKEN_TTL_S;
use frontline_gateway::clock::Clock;
use frontline_gateway::config::Config;
use http_body_util::BodyExt;
use serde_json::{json, Value};
use tower::ServiceExt;

const T0: u64 = 1_800_000_000_000;
const SETTINGS: &str = "/v1/me/settings";
const SETTINGS_BODY_LIMIT: usize = 16 * 1024;

async fn test_app(config: Config) -> (Router, Clock) {
    let clock = Clock::manual(T0);
    let state = AppState::new(config, clock.clone(), SigningKey::from_bytes(&[5; 32]))
        .await
        .unwrap();
    (router(state), clock)
}

async fn send(app: &Router, request: Request<Body>) -> Response {
    app.clone().oneshot(request).await.unwrap()
}

async fn ok_json(response: Response) -> Value {
    assert_eq!(response.status(), StatusCode::OK);
    let body = response.into_body().collect().await.unwrap().to_bytes();
    serde_json::from_slice(&body).expect("expected a JSON response")
}

async fn guest_bearer(app: &Router) -> String {
    let request = Request::post("/v1/guest")
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from("{}"))
        .unwrap();
    let guest = ok_json(send(app, request).await).await;
    format!("Bearer {}", guest["token"].as_str().unwrap())
}

async fn settings(
    app: &Router,
    method: Method,
    authorization: Option<&str>,
    body: impl Into<Body>,
) -> Response {
    let mut request = Request::builder()
        .method(method)
        .uri(SETTINGS)
        .header(header::CONTENT_TYPE, "application/json");
    if let Some(authorization) = authorization {
        request = request.header(header::AUTHORIZATION, authorization);
    }
    send(app, request.body(body.into()).unwrap()).await
}

async fn get_settings(app: &Router, bearer: &str) -> Value {
    ok_json(settings(app, Method::GET, Some(bearer), Body::empty()).await).await
}

async fn put_settings(app: &Router, bearer: &str, value: &Value) -> Value {
    ok_json(settings(app, Method::PUT, Some(bearer), value.to_string()).await).await
}

#[tokio::test]
async fn settings_default_to_empty_and_replace_the_object_for_only_one_guest() {
    let (app, _) = test_app(Config::test()).await;
    let alice = guest_bearer(&app).await;
    let bob = guest_bearer(&app).await;
    assert_ne!(alice, bob);
    assert_eq!(get_settings(&app, &alice).await, json!({}));

    let original = json!({"audio": {"music": true, "volume": 30}, "bindings": ["W", "S"]});
    assert_eq!(put_settings(&app, &alice, &original).await, original);
    assert_eq!(get_settings(&app, &alice).await, original);
    assert_eq!(get_settings(&app, &bob).await, json!({}));

    let bobs_settings = json!({"locale": "en"});
    assert_eq!(
        put_settings(&app, &bob, &bobs_settings).await,
        bobs_settings
    );

    let replacement = json!({"audio": {"music": false}});
    assert_eq!(put_settings(&app, &alice, &replacement).await, replacement);
    assert_eq!(get_settings(&app, &alice).await, replacement);
    assert_eq!(get_settings(&app, &bob).await, bobs_settings);
    assert_eq!(put_settings(&app, &bob, &json!({})).await, json!({}));
    assert_eq!(get_settings(&app, &bob).await, json!({}));
}

#[tokio::test]
async fn settings_survive_reopening_the_database() {
    let dir = tempfile::tempdir().unwrap();
    let config = Config {
        database_url: format!("sqlite://{}", dir.path().join("gateway.db").display()),
        ..Config::test()
    };
    let saved = json!({"audio": {"volume": 42}, "bindings": ["W", "S"]});
    let bearer = {
        let (app, _) = test_app(config.clone()).await;
        let bearer = guest_bearer(&app).await;
        assert_eq!(put_settings(&app, &bearer, &saved).await, saved);
        bearer
    };

    let (reopened, _) = test_app(config).await;
    assert_eq!(get_settings(&reopened, &bearer).await, saved);
}

#[tokio::test]
async fn settings_require_a_valid_guest_bearer_for_both_methods() {
    let (app, clock) = test_app(Config::test()).await;
    let bearer = guest_bearer(&app).await;
    assert_eq!(get_settings(&app, &bearer).await, json!({}));
    clock.advance_ms(TOKEN_TTL_S * 1_000);

    for (case, authorization) in [
        ("missing", None),
        ("wrong scheme", Some("Basic Z3Vlc3Q6")),
        ("invalid token", Some("Bearer invalid")),
        ("expired", Some(bearer.as_str())),
    ] {
        for (method, body) in [(Method::GET, ""), (Method::PUT, "{}")] {
            let response = settings(&app, method.clone(), authorization, body).await;
            assert_eq!(
                response.status(),
                StatusCode::UNAUTHORIZED,
                "{case} {method}"
            );
        }
    }
}

#[tokio::test]
async fn settings_reject_non_objects_and_malformed_json_without_overwriting() {
    let (app, _) = test_app(Config::test()).await;
    let bearer = guest_bearer(&app).await;
    let saved = json!({"volume": 42});
    assert_eq!(put_settings(&app, &bearer, &saved).await, saved);

    for body in ["[]", "null", "42", "true", r#""text""#, r#"{"volume":}"#] {
        let response = settings(&app, Method::PUT, Some(&bearer), body).await;
        assert_eq!(
            response.status(),
            StatusCode::UNPROCESSABLE_ENTITY,
            "body {body}"
        );
    }
    assert_eq!(get_settings(&app, &bearer).await, saved);
}

#[tokio::test]
async fn settings_enforce_the_16_kib_request_body_limit() {
    let (app, _) = test_app(Config::test()).await;
    let bearer = guest_bearer(&app).await;
    let overhead = r#"{"data":""}"#.len();
    let exact = json!({"data": "a".repeat(SETTINGS_BODY_LIMIT - overhead)});
    let mut body = exact.to_string();
    assert_eq!(body.len(), SETTINGS_BODY_LIMIT);
    assert_eq!(
        ok_json(settings(&app, Method::PUT, Some(&bearer), body.clone()).await).await,
        exact
    );

    // One byte of legal JSON whitespace exceeds the wire limit without changing the value.
    body.push(' ');
    let response = settings(&app, Method::PUT, Some(&bearer), body).await;
    assert_eq!(response.status(), StatusCode::PAYLOAD_TOO_LARGE);
    assert_eq!(get_settings(&app, &bearer).await, exact);
}

#[tokio::test]
async fn oauth_routes_are_disabled_by_default_and_enabled_providers_are_placeholders() {
    let (disabled, _) = test_app(Config::test()).await;
    let (enabled, _) = test_app(Config {
        oauth_enabled: true,
        ..Config::test()
    })
    .await;

    for (provider, enabled_status) in [
        ("discord", StatusCode::NOT_IMPLEMENTED),
        ("google", StatusCode::NOT_IMPLEMENTED),
        ("unknown", StatusCode::NOT_FOUND),
    ] {
        for endpoint in ["start", "callback"] {
            let path = format!("/v1/oauth/{provider}/{endpoint}");
            for (app, expected) in [
                (&disabled, StatusCode::NOT_FOUND),
                (&enabled, enabled_status),
            ] {
                let request = Request::get(path.as_str()).body(Body::empty()).unwrap();
                let response = send(app, request).await;
                assert_eq!(response.status(), expected, "GET {path}");
                for name in [header::LOCATION, header::SET_COOKIE, header::AUTHORIZATION] {
                    assert!(
                        !response.headers().contains_key(&name),
                        "GET {path} sent {name}"
                    );
                }
                let body = response.into_body().collect().await.unwrap().to_bytes();
                if let Ok(json) = serde_json::from_slice::<Value>(&body) {
                    for key in ["token", "access_token", "refresh_token", "id_token"] {
                        assert!(json.get(key).is_none(), "GET {path} returned {key}");
                    }
                }
            }
        }
    }
}

#[tokio::test]
async fn settings_put_preflight_allows_the_configured_origin() {
    const ORIGIN: &str = "https://play.example";
    let (app, _) = test_app(Config {
        allowed_origins: vec![ORIGIN.into()],
        ..Config::test()
    })
    .await;
    let request = Request::builder()
        .method(Method::OPTIONS)
        .uri(SETTINGS)
        .header(header::ORIGIN, ORIGIN)
        .header(header::ACCESS_CONTROL_REQUEST_METHOD, "PUT")
        .header(
            header::ACCESS_CONTROL_REQUEST_HEADERS,
            "authorization, content-type",
        )
        .body(Body::empty())
        .unwrap();
    let response = send(&app, request).await;
    assert!(response.status().is_success());
    assert_eq!(
        response.headers()[header::ACCESS_CONTROL_ALLOW_ORIGIN],
        ORIGIN
    );
    let methods = response.headers()[header::ACCESS_CONTROL_ALLOW_METHODS]
        .to_str()
        .unwrap();
    assert!(methods.split(',').any(|method| method.trim() == "PUT"));
}
