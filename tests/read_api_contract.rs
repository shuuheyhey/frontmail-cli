use frontmail_cli::{
    client::FrontClient,
    commands::{OutputOptions, ReadRequest, execute_read, whoami_json},
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
            output: Default::default(),
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
    assert!(actual["result"].get("returned").is_none());
    assert!(actual["result"].get("projection").is_none());
    assert!(actual["result"].get("truncated").is_none());
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
            output: Default::default(),
        },
    )
    .await
    .unwrap();
    let actual: Value = serde_json::from_str(&output).unwrap();

    assert_eq!(actual["result"]["data"]["id"], "tag_1");
    assert!(actual["result"].get("count").is_none());
    assert!(actual["result"].get("next_page_token").is_none());
}

async fn execute_body(body: Value, output: OutputOptions) -> Value {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/example"))
        .respond_with(ResponseTemplate::new(200).set_body_json(body))
        .mount(&server)
        .await;

    let output = execute_read(
        &client(&server),
        ReadRequest {
            command: "front api get /example".into(),
            segments: vec!["example".into()],
            query: vec![],
            pagination_command: None,
            output,
        },
    )
    .await
    .unwrap();

    serde_json::from_str(&output).unwrap()
}

#[tokio::test]
async fn count_only_omits_data_and_reports_the_original_collection_count() {
    let actual = execute_body(
        serde_json::json!({
            "_results": [
                {"id": "tag_1", "name": "Customer value one"},
                {"id": "tag_2", "name": "Customer value two"}
            ]
        }),
        OutputOptions {
            count_only: true,
            ..OutputOptions::default()
        },
    )
    .await;

    assert_eq!(
        actual["result"],
        serde_json::json!({
            "count": 2,
            "returned": 0,
            "projection": {"mode": "count-only"}
        })
    );
    let result = actual["result"].to_string();
    assert!(!result.contains("Customer value"));
    assert!(!result.contains("tag_1"));
}

#[tokio::test]
async fn keys_only_returns_sorted_keys_without_customer_values() {
    let actual = execute_body(
        serde_json::json!({
            "_results": [
                {"name": "Customer value", "id": "tag_1"},
                {"zeta": 1, "alpha": 2}
            ],
            "_pagination": {"next": "https://customer.invalid/secret"}
        }),
        OutputOptions {
            keys_only: true,
            ..OutputOptions::default()
        },
    )
    .await;

    assert_eq!(
        actual["result"]["data"],
        serde_json::json!({
            "_results": [
                ["id", "name"],
                ["alpha", "zeta"]
            ]
        })
    );
    assert_eq!(actual["result"]["count"], 2);
    assert_eq!(actual["result"]["returned"], 2);
    assert_eq!(
        actual["result"]["projection"],
        serde_json::json!({"mode": "keys-only"})
    );
    let data = actual["result"]["data"].to_string();
    assert!(!data.contains("Customer value"));
    assert!(!data.contains("tag_1"));
    assert!(!data.contains("customer.invalid"));
}

#[tokio::test]
async fn fields_projects_literal_keys_on_collection_items_and_single_objects() {
    let options = OutputOptions {
        fields: vec!["id".into(), "literal.nested".into(), "missing".into()],
        ..OutputOptions::default()
    };
    let collection = execute_body(
        serde_json::json!({
            "_results": [{
                "id": "tag_1",
                "literal.nested": "kept",
                "nested": {"value": "not selected"},
                "name": "not selected"
            }]
        }),
        options.clone(),
    )
    .await;
    let item = execute_body(
        serde_json::json!({
            "id": "tag_1",
            "literal.nested": "kept",
            "name": "not selected"
        }),
        options,
    )
    .await;

    let expected = serde_json::json!({"id": "tag_1", "literal.nested": "kept"});
    assert_eq!(collection["result"]["data"]["_results"][0], expected);
    assert_eq!(item["result"]["data"], expected);
    assert_eq!(collection["result"]["returned"], 1);
    assert_eq!(item["result"]["returned"], 1);
    assert_eq!(
        item["result"]["projection"],
        serde_json::json!({
            "mode": "fields",
            "fields": ["id", "literal.nested", "missing"]
        })
    );
}

#[tokio::test]
async fn fields_treats_non_array_results_as_a_literal_single_object_key() {
    let actual = execute_body(
        serde_json::json!({"_results": "literal value", "id": "tag_1"}),
        OutputOptions {
            fields: vec!["_results".into()],
            ..OutputOptions::default()
        },
    )
    .await;

    assert_eq!(
        actual["result"]["data"],
        serde_json::json!({"_results": "literal value"})
    );
}

#[tokio::test]
async fn keys_treats_non_array_results_as_a_literal_single_object_key() {
    let actual = execute_body(
        serde_json::json!({"_results": "literal value", "id": "tag_1"}),
        OutputOptions {
            keys_only: true,
            ..OutputOptions::default()
        },
    )
    .await;

    assert_eq!(
        actual["result"]["data"],
        serde_json::json!(["_results", "id"])
    );
}

#[tokio::test]
async fn max_items_truncates_locally_but_keeps_the_original_count() {
    let actual = execute_body(
        serde_json::json!({
            "_results": [{"id": 1}, {"id": 2}, {"id": 3}],
            "other": "preserved"
        }),
        OutputOptions {
            max_items: Some(2),
            ..OutputOptions::default()
        },
    )
    .await;

    assert_eq!(actual["result"]["count"], 3);
    assert_eq!(actual["result"]["returned"], 2);
    assert_eq!(actual["result"]["truncated"], true);
    assert_eq!(
        actual["result"]["data"]["_results"],
        serde_json::json!([{"id": 1}, {"id": 2}])
    );
    assert_eq!(actual["result"]["data"]["other"], "preserved");
    assert!(actual["result"].get("projection").is_none());
}

#[tokio::test]
async fn max_items_preserves_the_top_level_array_shape_and_original_count() {
    let actual = execute_body(
        serde_json::json!([
            {"id": 1, "name": "one"},
            {"id": 2, "name": "two"},
            {"id": 3, "name": "three"}
        ]),
        OutputOptions {
            max_items: Some(2),
            ..OutputOptions::default()
        },
    )
    .await;

    assert_eq!(
        actual["result"],
        serde_json::json!({
            "data": [
                {"id": 1, "name": "one"},
                {"id": 2, "name": "two"}
            ],
            "count": 3,
            "returned": 2,
            "truncated": true
        })
    );
}

#[tokio::test]
async fn keys_only_preserves_top_level_array_shape_and_reports_counts() {
    let actual = execute_body(
        serde_json::json!([
            {"zeta": "Customer value", "alpha": 1},
            {"id": "tag_1"}
        ]),
        OutputOptions {
            keys_only: true,
            ..OutputOptions::default()
        },
    )
    .await;

    assert_eq!(
        actual["result"],
        serde_json::json!({
            "data": [
                ["alpha", "zeta"],
                ["id"]
            ],
            "count": 2,
            "returned": 2,
            "projection": {"mode": "keys-only"}
        })
    );
}

#[tokio::test]
async fn fields_preserves_top_level_array_shape_and_reports_counts() {
    let actual = execute_body(
        serde_json::json!([
            {"id": "tag_1", "name": "Urgent", "ignored": true},
            {"id": "tag_2", "ignored": true}
        ]),
        OutputOptions {
            fields: vec!["id".into(), "name".into()],
            ..OutputOptions::default()
        },
    )
    .await;

    assert_eq!(
        actual["result"],
        serde_json::json!({
            "data": [
                {"id": "tag_1", "name": "Urgent"},
                {"id": "tag_2"}
            ],
            "count": 2,
            "returned": 2,
            "projection": {
                "mode": "fields",
                "fields": ["id", "name"]
            }
        })
    );
}

#[tokio::test]
async fn count_only_omits_top_level_array_data_and_reports_counts() {
    let actual = execute_body(
        serde_json::json!([
            {"id": "tag_1", "name": "Customer value one"},
            {"id": "tag_2", "name": "Customer value two"}
        ]),
        OutputOptions {
            count_only: true,
            ..OutputOptions::default()
        },
    )
    .await;

    assert_eq!(
        actual["result"],
        serde_json::json!({
            "count": 2,
            "returned": 0,
            "projection": {"mode": "count-only"}
        })
    );
}

#[tokio::test]
async fn projections_handle_empty_collections_missing_fields_and_non_objects() {
    let empty = execute_body(
        serde_json::json!({"_results": []}),
        OutputOptions {
            max_items: Some(2),
            ..OutputOptions::default()
        },
    )
    .await;
    let missing = execute_body(
        serde_json::json!({"id": "tag_1"}),
        OutputOptions {
            fields: vec!["missing".into()],
            ..OutputOptions::default()
        },
    )
    .await;
    let non_object = execute_body(
        serde_json::json!("Customer value"),
        OutputOptions {
            keys_only: true,
            ..OutputOptions::default()
        },
    )
    .await;

    assert_eq!(empty["result"]["count"], 0);
    assert_eq!(empty["result"]["returned"], 0);
    assert!(empty["result"].get("truncated").is_none());
    assert_eq!(missing["result"]["data"], serde_json::json!({}));
    assert_eq!(non_object["result"]["data"], serde_json::json!([]));
    assert!(
        !non_object["result"]["data"]
            .to_string()
            .contains("Customer value")
    );
}

#[tokio::test]
async fn pagination_actions_preserve_active_output_flags() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/tags"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "_results": [{"id": "tag_1", "name": "Urgent"}],
            "_pagination": {
                "next": "https://api2.frontapp.com/tags?page_token=next"
            }
        })))
        .mount(&server)
        .await;

    let output = execute_read(
        &client(&server),
        ReadRequest {
            command: "front list tag".into(),
            segments: vec!["tags".into()],
            query: vec![],
            pagination_command: Some("front list tag".into()),
            output: OutputOptions {
                fields: vec!["id".into(), "name".into()],
                max_items: Some(1),
                ..OutputOptions::default()
            },
        },
    )
    .await
    .unwrap();
    let actual: Value = serde_json::from_str(&output).unwrap();

    assert_eq!(
        actual["next_actions"][0]["params"]["--fields"]["value"],
        "id,name"
    );
    assert_eq!(
        actual["next_actions"][0]["params"]["--max-items"]["value"],
        "1"
    );
}

#[tokio::test]
async fn pagination_action_preserves_count_only_as_a_bare_switch() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/tags"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "_results": [{"id": "tag_1", "name": "Customer value"}],
            "_pagination": {
                "next": "https://api2.frontapp.com/tags?page_token=next"
            }
        })))
        .mount(&server)
        .await;

    let output = execute_read(
        &client(&server),
        ReadRequest {
            command: "front list tag".into(),
            segments: vec!["tags".into()],
            query: vec![],
            pagination_command: Some("front list tag".into()),
            output: OutputOptions {
                count_only: true,
                ..OutputOptions::default()
            },
        },
    )
    .await
    .unwrap();
    let actual: Value = serde_json::from_str(&output).unwrap();
    let params = actual["next_actions"][0]["params"].as_object().unwrap();
    let count_only = params["--count-only"].as_object().unwrap();

    assert_eq!(
        Value::Object(count_only.clone()),
        serde_json::json!({
            "description": "Return collection counts without response data"
        })
    );
    assert!(!count_only.contains_key("value"));
    assert!(!count_only.contains_key("values"));
    assert_eq!(params["--page-token"]["value"], "next");
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
