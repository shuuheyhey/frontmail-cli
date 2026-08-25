use frontmail_cli::client::{ClientError, FrontClient};
use secrecy::SecretString;
use wiremock::{
    Mock, MockServer, ResponseTemplate,
    matchers::{header, method, path, query_param},
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
