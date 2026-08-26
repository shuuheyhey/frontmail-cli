use frontmail_cli::resources::{Resource, parse_query_pairs, related_segments, validate_api_path};

#[test]
fn resource_aliases_map_to_official_collection_paths() {
    let cases = [
        ("accounts", "accounts"),
        ("channel", "channels"),
        ("contacts", "contacts"),
        ("conversation", "conversations"),
        ("knowledge-base", "knowledge_bases"),
        ("message-template", "message_templates"),
        ("message-template-folder", "message_template_folders"),
        ("tag", "tags"),
        ("teammate-group", "teammate_groups"),
        ("view", "views"),
    ];

    for (input, expected) in cases {
        let resource = Resource::parse(input).unwrap();
        assert_eq!(resource.collection_segments().unwrap(), [expected]);
    }
}

#[test]
fn listable_resources_match_the_pinned_official_pagination_matrix() {
    let cases = [
        (Resource::Account, true, true),
        (Resource::Channel, false, false),
        (Resource::Contact, true, true),
        (Resource::Conversation, true, true),
        (Resource::Event, true, true),
        (Resource::Inbox, false, false),
        (Resource::KnowledgeBase, false, false),
        (Resource::Link, true, true),
        (Resource::MessageTemplate, false, false),
        (Resource::MessageTemplateFolder, false, false),
        (Resource::Rule, false, false),
        (Resource::Shift, false, false),
        (Resource::Tag, true, true),
        (Resource::Teammate, false, false),
        (Resource::TeammateGroup, false, false),
        (Resource::Team, false, false),
        (Resource::View, true, true),
    ];

    for (resource, expected_limit, expected_page_token) in cases {
        let actual = resource.collection_query_capabilities();
        assert_eq!(
            (actual.limit, actual.page_token),
            (expected_limit, expected_page_token),
            "{}",
            resource.name()
        );
    }
}

#[test]
fn item_paths_use_the_official_plural_segment() {
    assert_eq!(
        Resource::parse("message-template")
            .unwrap()
            .item_segments("rsp_123")
            .unwrap(),
        ["message_templates", "rsp_123"]
    );
    assert_eq!(
        Resource::parse("comment")
            .unwrap()
            .item_segments("cmt_123")
            .unwrap(),
        ["comments", "cmt_123"]
    );
}

#[test]
fn unsupported_collection_is_rejected_before_http() {
    let error = Resource::parse("message")
        .unwrap()
        .collection_segments()
        .unwrap_err();
    assert!(error.to_string().contains("cannot be listed"));
}

#[test]
fn related_paths_are_allowlisted_per_parent_resource() {
    assert_eq!(
        related_segments(Resource::Conversation, "cnv_1", "comments",).unwrap(),
        ["conversations", "cnv_1", "comments"]
    );
    assert_eq!(
        related_segments(Resource::Teammate, "alt:email:user@example.com", "inboxes").unwrap(),
        ["teammates", "alt:email:user@example.com", "inboxes"]
    );
    assert!(related_segments(Resource::Tag, "tag_1", "messages").is_err());
}

#[test]
fn generic_path_accepts_relative_front_segments_only() {
    assert_eq!(
        validate_api_path("/company/statuses").unwrap(),
        ["company", "statuses"]
    );
    assert_eq!(
        validate_api_path("/teammates/alt:email:user@example.com/inboxes").unwrap(),
        ["teammates", "alt:email:user@example.com", "inboxes"]
    );

    for invalid in [
        "me",
        "//evil.example/me",
        "https://evil.example/me",
        "/conversations/../me",
        "/conversations/./me",
        "/contacts?limit=25",
        "/contacts#fragment",
        "/messages/msg_1/download/att_1",
        "/contacts/",
    ] {
        assert!(validate_api_path(invalid).is_err(), "accepted {invalid:?}");
    }
}

#[test]
fn query_pairs_split_once_and_reject_ambiguous_input() {
    assert_eq!(
        parse_query_pairs(&["limit=25".into(), "q=alice=bob".into()]).unwrap(),
        [
            ("limit".into(), "25".into()),
            ("q".into(), "alice=bob".into()),
        ]
    );

    for invalid in ["limit", "=25", "bad\nheader=value"] {
        assert!(parse_query_pairs(&[invalid.into()]).is_err());
    }
}
