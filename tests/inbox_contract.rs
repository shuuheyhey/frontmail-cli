use frontmail_cli::{
    client::FrontClient,
    commands::{InboxOptions, build_search_query, inbox_json, inboxes_json},
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

#[test]
fn search_query_preserves_go_compatibility_rules() {
    let options = InboxOptions {
        assignee: Some("alice@example.com".into()),
        before: Some("2026-03-01".into()),
        after: Some("2026-01-01".into()),
        ..InboxOptions::default()
    };
    assert_eq!(
        build_search_query(&options).unwrap(),
        "is:open is:assigned assignee:alt:email:alice@example.com before:1772323200 after:1767225600"
    );

    let custom = InboxOptions {
        query: "is:archived tag:tag_123".into(),
        query_was_set: true,
        assignee: Some("alice@example.com".into()),
        ..InboxOptions::default()
    };
    assert_eq!(
        build_search_query(&custom).unwrap(),
        "is:archived tag:tag_123 assignee:alt:email:alice@example.com"
    );
}

#[tokio::test]
async fn inboxes_uses_teammate_alias_when_user_is_configured() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/teammates/alt:email:user@example.com/inboxes"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "_results": [{"id": "inb_1", "name": "Support"}]
        })))
        .mount(&server)
        .await;

    let output = inboxes_json(&client(&server), "user@example.com")
        .await
        .unwrap();
    let actual: Value = serde_json::from_str(&output).unwrap();
    assert_eq!(actual["result"]["user"], "user@example.com");
    assert_eq!(actual["result"]["count"], 1);
    assert_eq!(actual["result"]["inboxes"][0]["id"], "inb_1");
    assert_eq!(
        actual["next_actions"][0]["params"]["inbox-id"]["value"],
        "inb_1"
    );
}

#[tokio::test]
async fn inbox_maps_conversations_and_exposes_the_next_page_token() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/conversations/search/is:open%20is:unassigned%20inbox:inb_1"))
        .and(query_param("limit", "25"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "_results": [{
                "id": "cnv_1",
                "subject": "Need help",
                "status": "assigned",
                "recipient": {"handle": "customer@example.com", "name": "Customer"},
                "assignee": {"first_name": "Alice", "last_name": "Agent", "email": "alice@example.com"},
                "updated_at": 1700000000,
                "waiting_since": 1699999000,
                "tags": [{"name": "urgent"}]
            }, {
                "id": "cnv_2",
                "subject": "Unassigned",
                "status": "unassigned",
                "recipient": {"handle": "second@example.com", "name": null},
                "assignee": null,
                "tags": null
            }],
            "_total": 7,
            "_pagination": {"next": "https://api2.frontapp.com/conversations?page_token=next%20token"}
        })))
        .mount(&server)
        .await;

    let options = InboxOptions {
        inbox_id: Some("inb_1".into()),
        ..InboxOptions::default()
    };
    let output = inbox_json(&client(&server), &options).await.unwrap();
    let actual: Value = serde_json::from_str(&output).unwrap();
    assert_eq!(actual["result"]["total"], 7);
    assert_eq!(actual["result"]["showing"], 2);
    assert_eq!(actual["result"]["next_page_token"], "next token");
    assert_eq!(
        actual["result"]["conversations"][0]["date"],
        "2023-11-14T22:13:20Z"
    );
    assert_eq!(actual["result"]["conversations"][0]["tags"][0], "urgent");
    assert!(
        actual["result"]["conversations"][1]
            .get("assignee")
            .is_none()
    );
    assert!(actual["result"]["conversations"][1].get("tags").is_none());
    assert_eq!(
        actual["next_actions"][0]["params"]["--page-token"]["value"],
        "next token"
    );
    assert_eq!(
        actual["next_actions"][1]["params"]["conversation-id"]["value"],
        "cnv_1"
    );
}
