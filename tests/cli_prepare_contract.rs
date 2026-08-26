use clap::Parser;
use frontmail_cli::cli::{Cli, prepare_read_request};

#[test]
fn list_request_uses_canonical_command_and_structured_query() {
    let cli = Cli::try_parse_from([
        "front",
        "list",
        "message-templates",
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

    assert_eq!(request.command, "front list message-template");
    assert_eq!(request.segments, ["message_templates"]);
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
        Some("front list message-template")
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
