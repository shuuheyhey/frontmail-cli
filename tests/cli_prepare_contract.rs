use clap::Parser;
use frontmail_cli::cli::{Cli, prepare_read_request};

#[test]
fn list_request_uses_canonical_command_and_structured_query() {
    let cli = Cli::try_parse_from([
        "front",
        "list",
        "tags",
        "--limit",
        "25",
        "--page-token",
        "next token",
        "--param",
        "q=hello world",
    ])
    .unwrap();
    let request = prepare_read_request(cli.command.as_ref().unwrap())
        .unwrap()
        .unwrap();

    assert_eq!(request.command, "front list tag");
    assert_eq!(request.segments, ["tags"]);
    assert_eq!(
        request.query,
        [
            ("q".into(), "hello world".into()),
            ("limit".into(), "25".into()),
            ("page_token".into(), "next token".into()),
        ]
    );
    assert_eq!(
        request.pagination_command.as_deref(),
        Some("front list tag")
    );
}

#[test]
fn list_request_rejects_unsupported_structured_pagination_flags() {
    let cases = [
        (["front", "list", "inboxes", "--limit", "2"], "--limit"),
        (
            ["front", "list", "teammates", "--page-token", "next"],
            "--page-token",
        ),
    ];

    for (args, flag) in cases {
        let cli = Cli::try_parse_from(args).unwrap();
        let error = prepare_read_request(cli.command.as_ref().unwrap()).unwrap_err();
        assert!(error.to_string().contains(flag), "{}", args.join(" "));
    }
}

#[test]
fn list_request_keeps_repeatable_params_for_non_paginated_collections() {
    let cli = Cli::try_parse_from([
        "front",
        "list",
        "inboxes",
        "--param",
        "limit=2",
        "--param",
        "page_token=next",
    ])
    .unwrap();
    let request = prepare_read_request(cli.command.as_ref().unwrap())
        .unwrap()
        .unwrap();

    assert_eq!(
        request.query,
        [
            ("limit".into(), "2".into()),
            ("page_token".into(), "next".into()),
        ]
    );
}

#[test]
fn item_related_and_api_requests_preserve_their_operands() {
    let cases = [
        (
            ["front", "get", "tag", "tag_1", "", ""],
            "front get tag tag_1",
        ),
        (
            ["front", "related", "conversation", "cnv_1", "comments", ""],
            "front related conversation cnv_1 comments",
        ),
        (
            ["front", "api", "get", "/company/statuses", "", ""],
            "front api get /company/statuses",
        ),
    ];

    for (args, expected) in cases {
        let args: Vec<_> = args.into_iter().filter(|arg| !arg.is_empty()).collect();
        let cli = Cli::try_parse_from(args).unwrap();
        let request = prepare_read_request(cli.command.as_ref().unwrap())
            .unwrap()
            .unwrap();
        assert_eq!(request.command, expected);
    }
}

#[test]
fn collection_limit_range_accepts_boundaries_and_rejects_values_outside_the_range() {
    let cases: [(&[&str], bool); 8] = [
        (&["front", "inbox", "--limit", "1"], true),
        (&["front", "inbox", "--limit", "100"], true),
        (&["front", "inbox", "--limit", "0"], false),
        (&["front", "inbox", "--limit", "101"], false),
        (&["front", "list", "tags", "--limit", "1"], true),
        (&["front", "list", "tags", "--limit", "100"], true),
        (&["front", "list", "tags", "--limit", "0"], false),
        (&["front", "list", "tags", "--limit", "101"], false),
    ];

    for (args, should_parse) in cases {
        assert_eq!(
            Cli::try_parse_from(args).is_ok(),
            should_parse,
            "{}",
            args.join(" ")
        );
    }
}
