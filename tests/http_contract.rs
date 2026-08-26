use frontmail_cli::client::{ClientError, FrontClient};
use secrecy::SecretString;
use wiremock::{
    Mock, MockServer, ResponseTemplate,
    matchers::{header, method, path, query_param, query_param_is_missing},
};

fn client(server: &MockServer) -> FrontClient {
    FrontClient::new(
        server.uri().parse().unwrap(),
        SecretString::from("test-token".to_owned()),
        "front/test",
    )
    .unwrap()
}

#[tokio::test]
async fn search_encodes_the_query_as_a_path_segment_and_sends_pagination() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/conversations/search/is:open%20inbox:inb_123"))
        .and(query_param("limit", "10"))
        .and(query_param("page_token", "next token"))
        .and(header("authorization", "Bearer test-token"))
        .and(header("user-agent", "front/test"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "_results": [],
            "_total": 0
        })))
        .mount(&server)
        .await;

    let response = client(&server)
        .search_conversations("is:open inbox:inb_123", 10, Some("next token"))
        .await
        .unwrap();
    assert_eq!(response.total, 0);
    assert!(response.results.is_empty());
}

#[tokio::test]
async fn teammate_alias_is_encoded_as_one_path_segment() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/teammates/alt:email:user@example.com/inboxes"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "_results": [{"id": "inb_1", "name": "Support"}]
        })))
        .mount(&server)
        .await;

    let response = client(&server)
        .list_teammate_inboxes("alt:email:user@example.com")
        .await
        .unwrap();
    assert_eq!(response.results[0].id.as_deref(), Some("inb_1"));
}

#[tokio::test]
async fn front_error_body_and_status_are_preserved_without_the_token() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/inboxes"))
        .respond_with(ResponseTemplate::new(401).set_body_json(serde_json::json!({
            "_error": {"status": 401, "message": "Invalid authentication"}
        })))
        .mount(&server)
        .await;

    let error = client(&server).list_inboxes().await.unwrap_err();
    match &error {
        ClientError::Http { status, message } => {
            assert_eq!(*status, 401);
            assert_eq!(message, "Invalid authentication");
        }
        other => panic!("unexpected error: {other}"),
    }
    assert!(!error.to_string().contains("test-token"));
}

#[tokio::test]
async fn get_value_encodes_segments_and_repeated_query_pairs() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path(
            "/teammates/alt:email:user@example.com/message_templates",
        ))
        .and(query_param("limit", "25"))
        .and(query_param("q", "a b"))
        .and(header("authorization", "Bearer test-token"))
        .and(header("user-agent", "front/test"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "_results": [{"id": "rsp_1", "name": "Welcome"}]
        })))
        .mount(&server)
        .await;

    let value = client(&server)
        .get_value(
            &[
                "teammates".into(),
                "alt:email:user@example.com".into(),
                "message_templates".into(),
            ],
            &[("limit".into(), "25".into()), ("q".into(), "a b".into())],
        )
        .await
        .unwrap();

    assert_eq!(value["_results"][0]["id"], "rsp_1");
}

#[tokio::test]
async fn get_value_preserves_non_object_json() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/custom_fields"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!(["one", "two"])))
        .mount(&server)
        .await;

    let value = client(&server)
        .get_value(&["custom_fields".into()], &[])
        .await
        .unwrap();

    assert_eq!(value, serde_json::json!(["one", "two"]));
}

#[tokio::test]
async fn get_value_does_not_follow_a_redirect_to_a_download_path() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/safe"))
        .respond_with(ResponseTemplate::new(302).insert_header("Location", "/download/file"))
        .expect(1)
        .mount(&server)
        .await;
    Mock::given(method("GET"))
        .and(path("/download/file"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({})))
        .expect(0)
        .mount(&server)
        .await;

    let error = client(&server)
        .get_value(&["safe".into()], &[])
        .await
        .unwrap_err();

    assert!(matches!(error, ClientError::Http { status: 302, .. }));
}

#[tokio::test]
async fn get_value_follows_a_safe_301_on_the_configured_origin() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/old"))
        .respond_with(ResponseTemplate::new(301).insert_header("Location", "/new"))
        .expect(1)
        .mount(&server)
        .await;
    Mock::given(method("GET"))
        .and(path("/new"))
        .and(header("authorization", "Bearer test-token"))
        .and(header("user-agent", "front/test"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "id": "redirected"
        })))
        .expect(1)
        .mount(&server)
        .await;

    let value = client(&server)
        .get_value(&["old".into()], &[])
        .await
        .unwrap();

    assert_eq!(value["id"], "redirected");
}

#[tokio::test]
async fn get_value_follows_a_safe_relative_301_location() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/old"))
        .respond_with(ResponseTemplate::new(301).insert_header("Location", "new"))
        .expect(1)
        .mount(&server)
        .await;
    Mock::given(method("GET"))
        .and(path("/new"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "id": "relative-redirected"
        })))
        .expect(1)
        .mount(&server)
        .await;

    let value = client(&server)
        .get_value(&["old".into()], &[])
        .await
        .unwrap();

    assert_eq!(value["id"], "relative-redirected");
}

#[tokio::test]
async fn get_value_follows_a_query_only_301_location() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/old"))
        .and(query_param("old", "1"))
        .respond_with(ResponseTemplate::new(301).insert_header("Location", "?cursor=next"))
        .expect(1)
        .mount(&server)
        .await;
    Mock::given(method("GET"))
        .and(path("/old"))
        .and(query_param("cursor", "next"))
        .and(query_param_is_missing("old"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "id": "query-only-redirected"
        })))
        .expect(1)
        .mount(&server)
        .await;

    let value = client(&server)
        .get_value(&["old".into()], &[("old".into(), "1".into())])
        .await
        .unwrap();

    assert_eq!(value["id"], "query-only-redirected");
}

#[tokio::test]
async fn get_value_follows_a_query_only_301_location_with_a_url_value() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/old"))
        .and(query_param_is_missing("next"))
        .respond_with(
            ResponseTemplate::new(301).insert_header("Location", "?next=https://example.test"),
        )
        .expect(1)
        .mount(&server)
        .await;
    Mock::given(method("GET"))
        .and(path("/old"))
        .and(query_param("next", "https://example.test"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "id": "url-query-redirected"
        })))
        .expect(1)
        .mount(&server)
        .await;

    let value = client(&server)
        .get_value(&["old".into()], &[])
        .await
        .unwrap();

    assert_eq!(value["id"], "url-query-redirected");
}

#[tokio::test]
async fn get_value_does_not_follow_a_301_to_a_download_path() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/safe"))
        .respond_with(ResponseTemplate::new(301).insert_header("Location", "/download/file"))
        .expect(1)
        .mount(&server)
        .await;
    Mock::given(method("GET"))
        .and(path("/download/file"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({})))
        .expect(0)
        .mount(&server)
        .await;

    let error = client(&server)
        .get_value(&["safe".into()], &[])
        .await
        .unwrap_err();

    assert!(matches!(error, ClientError::Http { status: 301, .. }));
}

#[tokio::test]
async fn get_value_does_not_follow_a_301_with_a_traversal_path() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/safe"))
        .respond_with(ResponseTemplate::new(301).insert_header("Location", "/../new"))
        .expect(1)
        .mount(&server)
        .await;
    Mock::given(method("GET"))
        .and(path("/new"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({})))
        .expect(0)
        .mount(&server)
        .await;

    let error = client(&server)
        .get_value(&["safe".into()], &[])
        .await
        .unwrap_err();

    assert!(matches!(error, ClientError::Http { status: 301, .. }));
}

#[tokio::test]
async fn get_value_preserves_safe_percent_encoding_in_a_301_location() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/old"))
        .respond_with(
            ResponseTemplate::new(301).insert_header("Location", "/conversations/cnv%5F1?view=all"),
        )
        .expect(1)
        .mount(&server)
        .await;
    Mock::given(method("GET"))
        .and(path("/conversations/cnv%5F1"))
        .and(query_param("view", "all"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "id": "single-encoded"
        })))
        .expect(1)
        .mount(&server)
        .await;
    Mock::given(method("GET"))
        .and(path("/conversations/cnv%255F1"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "id": "double-encoded"
        })))
        .expect(0)
        .mount(&server)
        .await;

    let value = client(&server)
        .get_value(&["old".into()], &[])
        .await
        .unwrap();

    assert_eq!(value["id"], "single-encoded");
}

#[tokio::test]
async fn get_value_does_not_follow_unsafe_encoded_301_paths() {
    for location in [
        "/safe%2Fnew",
        "/safe%5Cnew",
        "/safe%0Anew",
        "/%64ownload/file",
    ] {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/safe"))
            .respond_with(ResponseTemplate::new(301).insert_header("Location", location))
            .expect(1)
            .mount(&server)
            .await;

        let error = client(&server)
            .get_value(&["safe".into()], &[])
            .await
            .unwrap_err();

        assert!(
            matches!(error, ClientError::Http { status: 301, .. }),
            "followed unsafe Location {location:?}: {error}"
        );
    }
}

#[tokio::test]
async fn get_value_follows_three_safe_301_hops() {
    let server = MockServer::start().await;
    for (from, to) in [
        ("/hop-0", "/hop-1"),
        ("/hop-1", "/hop-2"),
        ("/hop-2", "/hop-3"),
    ] {
        Mock::given(method("GET"))
            .and(path(from))
            .respond_with(ResponseTemplate::new(301).insert_header("Location", to))
            .expect(1)
            .mount(&server)
            .await;
    }
    Mock::given(method("GET"))
        .and(path("/hop-3"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "id": "three-hops"
        })))
        .expect(1)
        .mount(&server)
        .await;

    let value = client(&server)
        .get_value(&["hop-0".into()], &[])
        .await
        .unwrap();

    assert_eq!(value["id"], "three-hops");
}

#[tokio::test]
async fn get_value_does_not_follow_a_fourth_301_hop() {
    let server = MockServer::start().await;
    for (from, to) in [
        ("/hop-0", "/hop-1"),
        ("/hop-1", "/hop-2"),
        ("/hop-2", "/hop-3"),
        ("/hop-3", "/hop-4"),
    ] {
        Mock::given(method("GET"))
            .and(path(from))
            .respond_with(ResponseTemplate::new(301).insert_header("Location", to))
            .expect(1)
            .mount(&server)
            .await;
    }
    Mock::given(method("GET"))
        .and(path("/hop-4"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({})))
        .expect(0)
        .mount(&server)
        .await;

    let error = client(&server)
        .get_value(&["hop-0".into()], &[])
        .await
        .unwrap_err();

    assert!(matches!(error, ClientError::Http { status: 301, .. }));
}

#[tokio::test]
async fn get_value_does_not_follow_a_301_to_another_origin() {
    let server = MockServer::start().await;
    let other_origin = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/safe"))
        .respond_with(
            ResponseTemplate::new(301)
                .insert_header("Location", format!("{}/outside", other_origin.uri())),
        )
        .expect(1)
        .mount(&server)
        .await;
    Mock::given(method("GET"))
        .and(path("/outside"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({})))
        .expect(0)
        .mount(&other_origin)
        .await;

    let error = client(&server)
        .get_value(&["safe".into()], &[])
        .await
        .unwrap_err();

    assert!(matches!(error, ClientError::Http { status: 301, .. }));
}

#[tokio::test]
async fn get_value_does_not_follow_a_301_with_an_encoded_dot_segment() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/safe"))
        .respond_with(ResponseTemplate::new(301).insert_header("Location", "/%2e%2e/new"))
        .expect(1)
        .mount(&server)
        .await;
    Mock::given(method("GET"))
        .and(path("/new"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({})))
        .expect(0)
        .mount(&server)
        .await;

    let error = client(&server)
        .get_value(&["safe".into()], &[])
        .await
        .unwrap_err();

    assert!(matches!(error, ClientError::Http { status: 301, .. }));
}

#[tokio::test]
async fn get_value_does_not_follow_a_301_with_backslash_traversal() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/safe"))
        .respond_with(ResponseTemplate::new(301).insert_header("Location", "/safe\\..\\new"))
        .expect(1)
        .mount(&server)
        .await;
    Mock::given(method("GET"))
        .and(path("/new"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({})))
        .expect(0)
        .mount(&server)
        .await;

    let error = client(&server)
        .get_value(&["safe".into()], &[])
        .await
        .unwrap_err();

    assert!(matches!(error, ClientError::Http { status: 301, .. }));
}
