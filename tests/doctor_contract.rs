use std::collections::BTreeSet;

use frontmail_cli::{
    client::{ClientError, FrontClient},
    commands::{CommandError, doctor_json},
    config::ConfigSource,
};
use secrecy::SecretString;
use serde_json::{Value, json};
use wiremock::{
    Mock, MockServer, ResponseTemplate,
    matchers::{method, path, query_param},
};

const SYNTHETIC_TOKEN: &str = "synthetic-doctor-token";
const SYNTHETIC_USER: &str = "diagnostic user@example.test";
const AUTHENTICATED_ID: &str = "tea_authenticated_private_id";

fn client(server: &MockServer) -> FrontClient {
    FrontClient::new(
        server.uri().parse().unwrap(),
        SecretString::from(SYNTHETIC_TOKEN.to_owned()),
        "front/test",
    )
    .unwrap()
}

async fn mount_authentication(server: &MockServer, id: &str) {
    Mock::given(method("GET"))
        .and(path("/me"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "id": id,
            "name": "Private authenticated teammate name"
        })))
        .mount(server)
        .await;
}

async fn mount_user_lookup(server: &MockServer, id: &str) {
    Mock::given(method("GET"))
        .and(path("/teammates/alt:email:diagnostic%20user@example.test"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "id": id,
            "email": SYNTHETIC_USER,
            "first_name": "Private configured teammate name"
        })))
        .mount(server)
        .await;
}

async fn mount_successful_optional_checks(server: &MockServer) {
    Mock::given(method("GET"))
        .and(path("/tags"))
        .and(query_param("limit", "1"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "_results": [{"id": "tag_private_id", "name": "Private tag name"}]
        })))
        .mount(server)
        .await;
    Mock::given(method("GET"))
        .and(path("/inboxes"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "_results": [{"id": "inb_private_id", "name": "Private inbox name"}]
        })))
        .mount(server)
        .await;
    Mock::given(method("GET"))
        .and(path("/teammates"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "_results": [{"id": "tea_private_id", "name": "Private teammate name"}]
        })))
        .mount(server)
        .await;
}

#[tokio::test]
async fn doctor_uses_only_gets_and_returns_redacted_fixed_diagnostics() {
    let server = MockServer::start().await;
    mount_authentication(&server, AUTHENTICATED_ID).await;
    mount_user_lookup(&server, AUTHENTICATED_ID).await;
    mount_successful_optional_checks(&server).await;

    let output = doctor_json(
        &client(&server),
        ConfigSource::Environment,
        ConfigSource::Environment,
        SYNTHETIC_USER,
    )
    .await
    .unwrap();
    let actual: Value = serde_json::from_str(&output).unwrap();

    assert_eq!(
        actual,
        json!({
            "ok": true,
            "command": "front doctor",
            "result": {
                "token_source": "environment",
                "authentication": "ok",
                "configured_user_source": "environment",
                "configured_user_matches_token": true,
                "checks": {
                    "tags_read": "ok",
                    "inboxes_read": "ok",
                    "teammates_read": "ok"
                }
            }
        })
    );

    for sensitive in [
        SYNTHETIC_TOKEN,
        SYNTHETIC_USER,
        AUTHENTICATED_ID,
        "Private authenticated teammate name",
        "Private configured teammate name",
        "tag_private_id",
        "Private tag name",
        "inb_private_id",
        "Private inbox name",
        "tea_private_id",
        "Private teammate name",
    ] {
        assert!(!output.contains(sensitive), "leaked {sensitive:?}");
    }

    let requests = server.received_requests().await.unwrap();
    assert_eq!(requests.len(), 5);
    assert_eq!(requests[0].url.path(), "/me");
    assert!(
        requests
            .iter()
            .all(|request| request.method.as_str() == "GET")
    );
    let requested_paths: BTreeSet<_> = requests
        .iter()
        .map(
            |request| match request.url.query().filter(|query| !query.is_empty()) {
                Some(query) => format!("{}?{query}", request.url.path()),
                None => request.url.path().to_owned(),
            },
        )
        .collect();
    assert_eq!(
        requested_paths,
        BTreeSet::from([
            "/me".to_owned(),
            "/teammates/alt:email:diagnostic%20user@example.test".to_owned(),
            "/tags?limit=1".to_owned(),
            "/inboxes".to_owned(),
            "/teammates".to_owned(),
        ])
    );
}

#[tokio::test]
async fn doctor_continues_after_optional_failures_without_exposing_error_bodies() {
    let server = MockServer::start().await;
    mount_authentication(&server, AUTHENTICATED_ID).await;
    Mock::given(method("GET"))
        .and(path("/teammates/alt:email:diagnostic%20user@example.test"))
        .respond_with(ResponseTemplate::new(403).set_body_json(json!({
            "_error": {"message": "Private alias lookup error body"}
        })))
        .mount(&server)
        .await;
    Mock::given(method("GET"))
        .and(path("/tags"))
        .and(query_param("limit", "1"))
        .respond_with(ResponseTemplate::new(403).set_body_json(json!({
            "_error": {"message": "Private tags error body"}
        })))
        .mount(&server)
        .await;
    Mock::given(method("GET"))
        .and(path("/inboxes"))
        .respond_with(ResponseTemplate::new(500).set_body_json(json!({
            "_error": {"message": "Private inboxes error body"}
        })))
        .mount(&server)
        .await;
    Mock::given(method("GET"))
        .and(path("/teammates"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_raw("Private invalid teammate response", "text/plain"),
        )
        .mount(&server)
        .await;

    let output = doctor_json(
        &client(&server),
        ConfigSource::TokenCommand,
        ConfigSource::Config,
        SYNTHETIC_USER,
    )
    .await
    .unwrap();
    let actual: Value = serde_json::from_str(&output).unwrap();

    assert_eq!(actual["result"]["token_source"], "token_command");
    assert_eq!(actual["result"]["authentication"], "ok");
    assert_eq!(actual["result"]["configured_user_source"], "config");
    assert_eq!(
        actual["result"]["configured_user_matches_token"],
        "unavailable"
    );
    assert_eq!(actual["result"]["checks"]["tags_read"], "forbidden");
    assert_eq!(actual["result"]["checks"]["inboxes_read"], "error");
    assert_eq!(actual["result"]["checks"]["teammates_read"], "error");
    assert_eq!(server.received_requests().await.unwrap().len(), 5);

    for sensitive in [
        SYNTHETIC_TOKEN,
        SYNTHETIC_USER,
        AUTHENTICATED_ID,
        "Private alias lookup error body",
        "Private tags error body",
        "Private inboxes error body",
        "Private invalid teammate response",
    ] {
        assert!(!output.contains(sensitive), "leaked {sensitive:?}");
    }
}

#[tokio::test]
async fn doctor_reports_false_when_the_configured_user_has_a_different_id() {
    let server = MockServer::start().await;
    mount_authentication(&server, AUTHENTICATED_ID).await;
    mount_user_lookup(&server, "tea_different_private_id").await;
    mount_successful_optional_checks(&server).await;

    let output = doctor_json(
        &client(&server),
        ConfigSource::Environment,
        ConfigSource::Config,
        SYNTHETIC_USER,
    )
    .await
    .unwrap();
    let actual: Value = serde_json::from_str(&output).unwrap();

    assert_eq!(actual["result"]["configured_user_matches_token"], false);
    assert!(!output.contains(AUTHENTICATED_ID));
    assert!(!output.contains("tea_different_private_id"));
    assert!(!output.contains(SYNTHETIC_USER));
}

#[tokio::test]
async fn doctor_skips_user_lookup_when_no_effective_user_is_configured() {
    let server = MockServer::start().await;
    mount_authentication(&server, AUTHENTICATED_ID).await;
    mount_successful_optional_checks(&server).await;

    let output = doctor_json(
        &client(&server),
        ConfigSource::Environment,
        ConfigSource::None,
        "",
    )
    .await
    .unwrap();
    let actual: Value = serde_json::from_str(&output).unwrap();

    assert_eq!(actual["result"]["configured_user_source"], "none");
    assert_eq!(
        actual["result"]["configured_user_matches_token"],
        "not_configured"
    );
    let requests = server.received_requests().await.unwrap();
    assert_eq!(requests.len(), 4);
    assert!(
        requests
            .iter()
            .all(|request| !request.url.path().starts_with("/teammates/alt:email:"))
    );
}

#[tokio::test]
async fn doctor_authentication_failure_stops_before_optional_checks() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/me"))
        .respond_with(ResponseTemplate::new(401).set_body_json(json!({
            "_error": {"message": "Private authentication error body"}
        })))
        .mount(&server)
        .await;

    let error = doctor_json(
        &client(&server),
        ConfigSource::Environment,
        ConfigSource::Environment,
        SYNTHETIC_USER,
    )
    .await
    .unwrap_err();

    assert!(matches!(
        error,
        CommandError::Client(ClientError::Http { status: 401, .. })
    ));
    let requests = server.received_requests().await.unwrap();
    assert_eq!(requests.len(), 1);
    assert_eq!(requests[0].url.path(), "/me");
}
