use clap::Parser;
use frontmail_cli::{
    cli::{Cli, prepare_read_request_with_profile},
    client::FrontClient,
    commands::{
        ContinuationQueryParam, OutputOptions, PaginationContext, ReadRequest, execute_read,
        whoami_json,
    },
    envelope::ActionContext,
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

fn prepare_continuation(action: &Value) -> ReadRequest {
    let mut args: Vec<String> = action["command"]
        .as_str()
        .unwrap()
        .split_whitespace()
        .map(str::to_owned)
        .collect();
    for (name, spec) in action["params"].as_object().unwrap() {
        if let Some(value) = spec.get("value").and_then(Value::as_str) {
            args.push(name.clone());
            args.push(value.into());
        } else if let Some(values) = spec.get("values").and_then(Value::as_array) {
            for value in values {
                args.push(name.clone());
                args.push(value.as_str().unwrap().into());
            }
        } else {
            args.push(name.clone());
        }
    }
    let cli = Cli::try_parse_from(&args)
        .unwrap_or_else(|error| panic!("continuation did not parse: {args:?}: {error}"));
    prepare_read_request_with_profile(cli.command.as_ref().unwrap(), cli.profile.as_deref())
        .unwrap()
        .unwrap()
}

fn prepared_request(args: &[&str]) -> ReadRequest {
    let cli = Cli::try_parse_from(args).unwrap();
    prepare_read_request_with_profile(cli.command.as_ref().unwrap(), cli.profile.as_deref())
        .unwrap()
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
            pagination: Some(PaginationContext::structured(
                "front list tags",
                vec![
                    ContinuationQueryParam::Passthrough("q".into(), "alice smith".into()),
                    ContinuationQueryParam::Passthrough("sort_by".into(), "created_at".into()),
                    ContinuationQueryParam::StructuredLimit(2),
                    ContinuationQueryParam::StructuredPageToken("current".into()),
                ],
            )),
            action_context: ActionContext::default(),
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
async fn api_continuation_keeps_non_numeric_passthrough_limit_parseable() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/tags"))
        .and(query_param("limit", "unbounded"))
        .and(query_param("q", "hello world"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "_results": [],
            "_pagination": {
                "next": "https://api2.frontapp.com/tags?page_token=next%20token"
            }
        })))
        .mount(&server)
        .await;
    let request = prepared_request(&[
        "front",
        "api",
        "get",
        "/tags",
        "--param",
        "limit=unbounded",
        "--param",
        "q=hello world",
        "--profile",
        "work",
        "--count-only",
    ]);

    let output = execute_read(&client(&server), request).await.unwrap();
    let actual: Value = serde_json::from_str(&output).unwrap();
    let action = &actual["next_actions"][0];
    let continued = prepare_continuation(action);

    assert_eq!(
        action["params"]["--param"]["values"],
        serde_json::json!(["limit=unbounded", "q=hello world"])
    );
    assert!(action["params"].get("--limit").is_none());
    assert_eq!(action["params"]["--page-token"]["value"], "next token");
    assert_eq!(action["params"]["--profile"]["value"], "work");
    assert!(action["params"]["--count-only"].get("value").is_none());
    assert_eq!(
        continued.query,
        [
            ("limit".into(), "unbounded".into()),
            ("q".into(), "hello world".into()),
            ("page_token".into(), "next token".into()),
        ]
    );
}

#[tokio::test]
async fn api_continuation_keeps_passthrough_page_token_passthrough() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/tags"))
        .and(query_param("page_token", "stale"))
        .and(query_param("q", "first"))
        .and(query_param("q", "second"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "_results": [],
            "_pagination": {
                "next": "https://api2.frontapp.com/tags?page_token=fresh"
            }
        })))
        .mount(&server)
        .await;
    let request = prepared_request(&[
        "front",
        "api",
        "get",
        "/tags",
        "--param",
        "page_token=stale",
        "--param",
        "q=first",
        "--param",
        "q=second",
        "--keys-only",
    ]);

    let output = execute_read(&client(&server), request).await.unwrap();
    let actual: Value = serde_json::from_str(&output).unwrap();
    let action = &actual["next_actions"][0];
    let continued = prepare_continuation(action);

    assert_eq!(
        action["params"]["--param"]["values"],
        serde_json::json!(["page_token=fresh", "q=first", "q=second"])
    );
    assert!(action["params"].get("--page-token").is_none());
    assert!(action["params"]["--keys-only"].get("value").is_none());
    assert_eq!(
        continued.query,
        [
            ("page_token".into(), "fresh".into()),
            ("q".into(), "first".into()),
            ("q".into(), "second".into()),
        ]
    );
}

#[tokio::test]
async fn api_continuation_keeps_structured_page_token_structured() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/tags"))
        .and(query_param("limit", "2"))
        .and(query_param("page_token", "stale"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "_results": [],
            "_pagination": {
                "next": "https://api2.frontapp.com/tags?page_token=fresh"
            }
        })))
        .mount(&server)
        .await;
    let request = prepared_request(&[
        "front",
        "api",
        "get",
        "/tags",
        "--limit",
        "2",
        "--page-token",
        "stale",
        "--profile",
        "work",
        "--fields",
        "id,name",
        "--max-items",
        "1",
    ]);

    let output = execute_read(&client(&server), request).await.unwrap();
    let actual: Value = serde_json::from_str(&output).unwrap();
    let action = &actual["next_actions"][0];
    let continued = prepare_continuation(action);

    assert!(action["params"].get("--param").is_none());
    assert_eq!(action["params"]["--limit"]["value"], "2");
    assert_eq!(action["params"]["--page-token"]["value"], "fresh");
    assert_eq!(action["params"]["--profile"]["value"], "work");
    assert_eq!(action["params"]["--fields"]["value"], "id,name");
    assert_eq!(action["params"]["--max-items"]["value"], "1");
    assert_eq!(
        continued.query,
        [
            ("limit".into(), "2".into()),
            ("page_token".into(), "fresh".into()),
        ]
    );
}

#[tokio::test]
async fn non_pagination_resource_continuation_replaces_passthrough_page_token_in_place() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/inboxes"))
        .and(query_param("limit", "unbounded"))
        .and(query_param("page_token", "stale"))
        .and(query_param("page_token", "older"))
        .and(query_param("q", "first"))
        .and(query_param("q", "second"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "_results": [],
            "_pagination": {
                "next": "https://api2.frontapp.com/inboxes?page_token=fresh"
            }
        })))
        .mount(&server)
        .await;
    let request = prepared_request(&[
        "front",
        "list",
        "inboxes",
        "--param",
        "limit=unbounded",
        "--param",
        "page_token=stale",
        "--param",
        "page_token=older",
        "--param",
        "q=first",
        "--param",
        "q=second",
        "--keys-only",
    ]);

    let output = execute_read(&client(&server), request).await.unwrap();
    let actual: Value = serde_json::from_str(&output).unwrap();
    let action = &actual["next_actions"][0];
    let continued = prepare_continuation(action);

    assert_eq!(
        action["params"]["--param"]["values"],
        serde_json::json!(["limit=unbounded", "page_token=fresh", "q=first", "q=second"])
    );
    assert!(action["params"].get("--limit").is_none());
    assert!(action["params"].get("--page-token").is_none());
    assert!(action["params"]["--keys-only"].get("value").is_none());
    assert_eq!(
        continued.query,
        [
            ("limit".into(), "unbounded".into()),
            ("page_token".into(), "fresh".into()),
            ("q".into(), "first".into()),
            ("q".into(), "second".into()),
        ]
    );
}

#[tokio::test]
async fn api_continuation_preserves_structured_and_passthrough_limit_precedence() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/tags"))
        .and(query_param("limit", "passthrough"))
        .and(query_param("limit", "2"))
        .and(query_param("page_token", "passthrough-old"))
        .and(query_param("page_token", "structured-old"))
        .and(query_param("q", "first"))
        .and(query_param("q", "second"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "_results": [],
            "_pagination": {
                "next": "https://api2.frontapp.com/tags?page_token=fresh"
            }
        })))
        .mount(&server)
        .await;
    let request = prepared_request(&[
        "front",
        "api",
        "get",
        "/tags",
        "--param",
        "limit=passthrough",
        "--param",
        "page_token=passthrough-old",
        "--param",
        "q=first",
        "--param",
        "q=second",
        "--limit",
        "2",
        "--page-token",
        "structured-old",
    ]);

    let output = execute_read(&client(&server), request).await.unwrap();
    let actual: Value = serde_json::from_str(&output).unwrap();
    let action = &actual["next_actions"][0];
    let continued = prepare_continuation(action);

    assert_eq!(action["params"]["--limit"]["value"], "2");
    assert_eq!(
        action["params"]["--param"]["values"],
        serde_json::json!(["limit=passthrough", "q=first", "q=second"])
    );
    assert_eq!(action["params"]["--page-token"]["value"], "fresh");
    assert_eq!(
        continued.query,
        [
            ("limit".into(), "passthrough".into()),
            ("q".into(), "first".into()),
            ("q".into(), "second".into()),
            ("limit".into(), "2".into()),
            ("page_token".into(), "fresh".into()),
        ]
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
            pagination: None,
            action_context: ActionContext::default(),
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
            pagination: None,
            action_context: ActionContext::default(),
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
            pagination: Some(PaginationContext::structured("front list tag", vec![])),
            action_context: ActionContext::default(),
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
async fn pagination_action_preserves_profile_limit_and_local_output_controls() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/tags"))
        .and(query_param("limit", "100"))
        .and(query_param("page_token", "current"))
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
            query: vec![
                ("limit".into(), "100".into()),
                ("page_token".into(), "current".into()),
            ],
            pagination: Some(PaginationContext::structured(
                "front list tag",
                vec![
                    ContinuationQueryParam::StructuredLimit(100),
                    ContinuationQueryParam::StructuredPageToken("current".into()),
                ],
            )),
            action_context: ActionContext::from_explicit_profile(Some("work")),
            output: OutputOptions {
                fields: vec!["id".into(), "name".into()],
                max_items: Some(2),
                ..OutputOptions::default()
            },
        },
    )
    .await
    .unwrap();
    let actual: Value = serde_json::from_str(&output).unwrap();
    let action = &actual["next_actions"][0];

    assert_eq!(action["command"], "front list tag");
    assert_eq!(action["params"]["--profile"]["value"], "work");
    assert_eq!(action["params"]["--limit"]["value"], "100");
    assert_eq!(action["params"]["--fields"]["value"], "id,name");
    assert_eq!(action["params"]["--max-items"]["value"], "2");
    assert_eq!(action["params"]["--page-token"]["value"], "next");
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
            pagination: Some(PaginationContext::structured("front list tag", vec![])),
            action_context: ActionContext::default(),
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
