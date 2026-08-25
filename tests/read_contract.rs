use frontmail_cli::{client::FrontClient, commands::read_json};
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
async fn read_fetches_conversation_and_messages_and_truncates_utf8_safely() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/conversations/cnv_1"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "id": "cnv_1",
            "subject": "Hello",
            "status": "open",
            "recipient": {"handle": "customer@example.com"}
        })))
        .mount(&server)
        .await;
    Mock::given(method("GET"))
        .and(path("/conversations/cnv_1/messages"))
        .and(query_param("limit", "25"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "_results": [{
                "id": "msg_1",
                "text": "あ".repeat(200),
                "body": "<p>ignored</p>",
                "created_at": 1700000000,
                "is_inbound": true,
                "author": {"first_name": "Bob", "last_name": "Smith", "email": "bob@example.com"}
            }, {
                "id": "msg_2",
                "text": "",
                "body": "Body fallback"
            }],
            "_pagination": {"next": "https://api2.frontapp.com/messages?page_token=next"}
        })))
        .mount(&server)
        .await;

    let output = read_json(&client(&server), "cnv_1").await.unwrap();
    let actual: Value = serde_json::from_str(&output).unwrap();
    assert_eq!(actual["result"]["conversation"]["id"], "cnv_1");
    assert_eq!(actual["result"]["truncated"], true);
    let text = actual["result"]["messages"][0]["text"].as_str().unwrap();
    assert!(text.is_char_boundary(text.len()));
    assert!(text.ends_with("... [truncated]"));
    assert!(text.len() <= 500 + "... [truncated]".len());
    assert_eq!(actual["result"]["messages"][1]["text"], "Body fallback");
    assert_eq!(
        actual["next_actions"][0]["params"]["conversation-id"]["value"],
        "cnv_1"
    );
}
