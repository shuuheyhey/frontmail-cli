pub mod cli;
pub mod client;
pub mod commands;
pub mod config;
pub mod envelope;
pub mod error;
pub mod models;
pub mod resources;

use std::collections::BTreeMap;

use envelope::{Action, ParamSpec};

pub const VERSION: &str = match option_env!("FRONT_VERSION") {
    Some(version) => version,
    None => env!("CARGO_PKG_VERSION"),
};

pub fn root_json() -> serde_json::Result<String> {
    envelope::success(
        "front",
        serde_json::json!({ "version": VERSION }),
        vec![
            Action {
                command: "front config".into(),
                description: "Show CLI configuration".into(),
                params: BTreeMap::new(),
            },
            Action {
                command: "front inbox [inbox-id]".into(),
                description: "Search conversations".into(),
                params: inbox_params(),
            },
            Action {
                command: "front inboxes".into(),
                description: "List all inboxes".into(),
                params: BTreeMap::new(),
            },
            Action {
                command: "front read <conversation-id>".into(),
                description: "Read a conversation and its messages".into(),
                params: BTreeMap::from([(
                    "conversation-id".into(),
                    ParamSpec {
                        required: true,
                        ..ParamSpec::new("conversation-id (required)")
                    },
                )]),
            },
            Action {
                command: "front whoami".into(),
                description: "Show the authenticated Front user".into(),
                params: BTreeMap::new(),
            },
            Action {
                command: "front list <resource>".into(),
                description: "List a supported Front resource".into(),
                params: generic_resource_params(false),
            },
            Action {
                command: "front get <resource> <id>".into(),
                description: "Get a supported Front resource by ID".into(),
                params: generic_resource_params(true),
            },
            Action {
                command: "front related <resource> <id> <relation>".into(),
                description: "List an allowlisted resource relation".into(),
                params: BTreeMap::from([
                    (
                        "id".into(),
                        ParamSpec {
                            required: true,
                            ..ParamSpec::new("Front resource ID or alternate ID")
                        },
                    ),
                    (
                        "relation".into(),
                        ParamSpec {
                            required: true,
                            ..ParamSpec::new("Allowlisted relation name")
                        },
                    ),
                    (
                        "resource".into(),
                        ParamSpec {
                            required: true,
                            ..ParamSpec::new("Supported Front resource")
                        },
                    ),
                ]),
            },
            Action {
                command: "front api get <path>".into(),
                description: "GET a relative Front Core API path".into(),
                params: BTreeMap::from([(
                    "path".into(),
                    ParamSpec {
                        required: true,
                        ..ParamSpec::new("Relative API path beginning with one slash")
                    },
                )]),
            },
            Action {
                command: "front completion <shell>".into(),
                description: "Generate shell completion code".into(),
                params: BTreeMap::from([(
                    "shell".into(),
                    ParamSpec {
                        required: true,
                        r#enum: vec![
                            "bash".into(),
                            "elvish".into(),
                            "fish".into(),
                            "powershell".into(),
                            "zsh".into(),
                        ],
                        ..ParamSpec::new("Target shell")
                    },
                )]),
            },
        ],
    )
}

fn generic_resource_params(include_id: bool) -> BTreeMap<String, ParamSpec> {
    let mut params = BTreeMap::from([(
        "resource".into(),
        ParamSpec {
            required: true,
            ..ParamSpec::new("Supported Front resource")
        },
    )]);
    if include_id {
        params.insert(
            "id".into(),
            ParamSpec {
                required: true,
                ..ParamSpec::new("Front resource ID or alternate ID")
            },
        );
    }
    params
}

pub fn inbox_params() -> BTreeMap<String, ParamSpec> {
    BTreeMap::from([
        (
            "--after".into(),
            ParamSpec::new("After date, YYYY-MM-DD (shortcut for after:<ts> in query)"),
        ),
        (
            "--assignee".into(),
            ParamSpec::new("Filter by assignee email address"),
        ),
        (
            "--before".into(),
            ParamSpec::new("Before date, YYYY-MM-DD (shortcut for before:<ts> in query)"),
        ),
        (
            "--from".into(),
            ParamSpec::new("Filter by sender handle (shortcut for from:<handle> in query)"),
        ),
        (
            "--limit".into(),
            ParamSpec {
                default: Some("25".into()),
                ..ParamSpec::new("Maximum number of results to return")
            },
        ),
        (
            "--query".into(),
            ParamSpec {
                default: Some("is:open is:unassigned".into()),
                ..ParamSpec::new("Search query (Front search syntax)")
            },
        ),
        ("inbox-id".into(), ParamSpec::new("inbox-id (optional)")),
    ])
}
