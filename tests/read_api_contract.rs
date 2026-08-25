use frontmail_cli::{
    client::FrontClient,
    commands::{ReadRequest, execute_read, whoami_json},
};
use secrecy::SecretString;
use serde_json::Value;
use wiremock::{
    Mock, MockServer, ResponseTemplate,
    matchers::{method, path, query_param},
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
async fn collection_envelope_preserves_data_and_builds_pagination_action() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/tags"))
        .and(query_param("limit", "2"))
        .and(query_param("page_token", "current"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "_results": [
                {"id": "tag_1", "name": "Urgent"},
                {"id": "tag_2", "name": "Billing"}
            ],
            "_pagination": {
                "next": "https://api2.frontapp.com/tags?limit=2&page_token=next%20token"
            }
        })))
        .mount(&server)
        .await;

    let output = execute_read(
        &client(&server),
        ReadRequest {
            command: "front list tags".into(),
            segments: vec!["tags".into()],
            query: vec![
                ("q".into(), "alice smith".into()),
                ("sort_by".into(), "created_at".into()),
                ("limit".into(), "2".into()),
                ("page_token".into(), "current".into()),
            ],
            pagination_command: Some("front list tags".into()),
        },
    )
    .await
    .unwrap();
    let actual: Value = serde_json::from_str(&output).unwrap();

    assert_eq!(actual["command"], "front list tags");
    assert_eq!(actual["result"]["count"], 2);
    assert_eq!(actual["result"]["next_page_token"], "next token");
    assert_eq!(actual["result"]["data"]["_results"][0]["name"], "Urgent");
    assert_eq!(actual["next_actions"][0]["command"], "front list tags");
    assert_eq!(
        actual["next_actions"][0]["params"]["--page-token"]["value"],
        "next token"
    );
    assert_eq!(actual["next_actions"][0]["params"]["--limit"]["value"], "2");
    assert_eq!(
        actual["next_actions"][0]["params"]["--param"]["values"],
        serde_json::json!(["q=alice smith", "sort_by=created_at"])
    );
}

#[tokio::test]
async fn item_envelope_omits_collection_metadata() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/tags/tag_1"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "id": "tag_1", "name": "Urgent"
        })))
        .mount(&server)
        .await;

    let output = execute_read(
        &client(&server),
        ReadRequest {
            command: "front get tag tag_1".into(),
            segments: vec!["tags".into(), "tag_1".into()],
            query: vec![],
            pagination_command: None,
        },
    )
    .await
    .unwrap();
    let actual: Value = serde_json::from_str(&output).unwrap();

    assert_eq!(actual["result"]["data"]["id"], "tag_1");
    assert!(actual["result"].get("count").is_none());
    assert!(actual["result"].get("next_page_token").is_none());
}

#[tokio::test]
async fn whoami_uses_the_token_details_endpoint() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/me"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "id": "tea_1",
            "email": "user@example.com",
            "first_name": "User"
        })))
        .mount(&server)
        .await;

    let output = whoami_json(&client(&server)).await.unwrap();
    let actual: Value = serde_json::from_str(&output).unwrap();
    assert_eq!(actual["command"], "front whoami");
    assert_eq!(actual["result"]["data"]["email"], "user@example.com");
}
