use clap::{Parser, error::ErrorKind};
use frontmail_cli::{
    cli::{Cli, prepare_read_request, prepare_read_request_with_profile},
    commands::OutputOptions,
};

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
    assert_eq!(request.output, OutputOptions::default());
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

#[test]
fn generic_reads_prepare_output_options_without_adding_query_pairs() {
    let cases = [
        vec![
            "front",
            "list",
            "tags",
            "--limit",
            "25",
            "--fields",
            "id,name",
            "--max-items",
            "2",
        ],
        vec![
            "front",
            "get",
            "tag",
            "tag_1",
            "--fields",
            "id,name",
            "--max-items",
            "2",
        ],
        vec![
            "front",
            "related",
            "conversation",
            "cnv_1",
            "comments",
            "--fields",
            "id,name",
            "--max-items",
            "2",
        ],
        vec![
            "front",
            "api",
            "get",
            "/contacts",
            "--fields",
            "id,name",
            "--max-items",
            "2",
        ],
    ];

    for args in cases {
        let cli = Cli::try_parse_from(args).unwrap();
        let request = prepare_read_request(cli.command.as_ref().unwrap())
            .unwrap()
            .unwrap();

        assert_eq!(
            request.output,
            OutputOptions {
                fields: vec!["id".into(), "name".into()],
                max_items: Some(2),
                ..OutputOptions::default()
            }
        );
        assert!(
            request
                .query
                .iter()
                .all(|(name, _)| name != "fields" && name != "max_items")
        );
    }
}

#[test]
fn count_and_key_modes_prepare_distinct_output_options() {
    let count = Cli::try_parse_from(["front", "list", "tags", "--count-only"]).unwrap();
    let count = prepare_read_request(count.command.as_ref().unwrap())
        .unwrap()
        .unwrap();
    assert_eq!(
        count.output,
        OutputOptions {
            count_only: true,
            ..OutputOptions::default()
        }
    );

    let keys = Cli::try_parse_from(["front", "get", "tag", "tag_1", "--keys-only"]).unwrap();
    let keys = prepare_read_request(keys.command.as_ref().unwrap())
        .unwrap()
        .unwrap();
    assert_eq!(
        keys.output,
        OutputOptions {
            keys_only: true,
            ..OutputOptions::default()
        }
    );
}

#[test]
fn incompatible_output_modes_are_rejected_by_cli_parsing() {
    for args in [
        vec!["front", "list", "tags", "--count-only", "--keys-only"],
        vec!["front", "list", "tags", "--count-only", "--fields", "id"],
        vec!["front", "list", "tags", "--keys-only", "--fields", "id"],
        vec!["front", "list", "tags", "--count-only", "--max-items", "1"],
    ] {
        let Err(error) = Cli::try_parse_from(args) else {
            panic!("incompatible output modes were accepted")
        };
        assert_eq!(error.kind(), ErrorKind::ArgumentConflict);
    }
}

#[test]
fn zero_max_items_is_rejected_by_cli_parsing() {
    let result = Cli::try_parse_from(["front", "api", "get", "/contacts", "--max-items", "0"]);

    assert!(result.is_err());
}

#[test]
fn compact_reads_do_not_accept_generic_output_flags() {
    for args in [
        ["front", "whoami", "--count-only"],
        ["front", "inboxes", "--keys-only"],
    ] {
        assert!(Cli::try_parse_from(args).is_err());
    }
}

#[test]
fn profile_and_output_flags_prepare_the_same_paginated_request_in_global_positions() {
    let cases = [
        vec![
            "front",
            "--profile",
            "work",
            "list",
            "tags",
            "--limit",
            "100",
            "--fields",
            "id,name",
            "--max-items",
            "2",
        ],
        vec![
            "front",
            "list",
            "tags",
            "--limit",
            "100",
            "--fields",
            "id,name",
            "--max-items",
            "2",
            "--profile",
            "work",
        ],
        vec![
            "front",
            "api",
            "get",
            "/tags",
            "--profile",
            "work",
            "--limit",
            "100",
            "--fields",
            "id,name",
            "--max-items",
            "2",
        ],
    ];

    for args in cases {
        let cli = Cli::try_parse_from(args).unwrap();
        let request = prepare_read_request_with_profile(
            cli.command.as_ref().unwrap(),
            cli.profile.as_deref(),
        )
        .unwrap()
        .unwrap();

        assert_eq!(request.profile.as_deref(), Some("work"));
        assert_eq!(request.query, [("limit".into(), "100".into())]);
        assert_eq!(
            request.output,
            OutputOptions {
                fields: vec!["id".into(), "name".into()],
                max_items: Some(2),
                ..OutputOptions::default()
            }
        );
    }
}
