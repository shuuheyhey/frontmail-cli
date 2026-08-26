#[cfg(not(target_os = "windows"))]
use assert_cmd::Command;
use assert_cmd::cargo::cargo_bin_cmd;
use serde_json::{Value, json};
#[cfg(not(target_os = "windows"))]
use std::{
    fs,
    path::{Path, PathBuf},
};
use tempfile::tempdir;

#[cfg(not(target_os = "windows"))]
fn isolated_config_command(root: &Path) -> (Command, PathBuf) {
    let mut command = cargo_bin_cmd!("front");
    #[cfg(target_os = "macos")]
    let (config_env, front_dir) = (
        "HOME",
        root.join("Library")
            .join("Application Support")
            .join("front"),
    );
    #[cfg(not(target_os = "macos"))]
    let (config_env, front_dir) = ("XDG_CONFIG_HOME", root.join("front"));
    command.env(config_env, root);
    (command, front_dir.join("config.yaml"))
}

#[test]
fn root_prints_the_agent_friendly_command_catalog() {
    let output = cargo_bin_cmd!("front").output().expect("front should run");

    assert!(output.status.success());
    let actual: Value = serde_json::from_slice(&output.stdout).expect("valid JSON");
    assert_eq!(actual["ok"], true);
    assert_eq!(actual["command"], "front");
    assert_eq!(
        actual["result"],
        json!({ "version": env!("CARGO_PKG_VERSION") })
    );

    let commands: Vec<&str> = actual["next_actions"]
        .as_array()
        .expect("next_actions array")
        .iter()
        .map(|action| action["command"].as_str().expect("command string"))
        .collect();
    assert_eq!(
        commands,
        [
            "front config",
            "front doctor",
            "front inbox [inbox-id]",
            "front inboxes",
            "front read <conversation-id>",
            "front whoami",
            "front list <resource>",
            "front get <resource> <id>",
            "front related <resource> <id> <relation>",
            "front api get <path>",
            "front completion <shell>",
        ]
    );
}

#[test]
#[cfg(not(target_os = "windows"))]
fn doctor_without_a_token_reports_the_canonical_command() {
    let dir = tempdir().unwrap();
    let (mut command, _) = isolated_config_command(dir.path());

    let output = command
        .arg("doctor")
        .env_remove("FRONT_API_TOKEN")
        .env_remove("FRONT_USER")
        .output()
        .unwrap();

    assert_eq!(output.status.code(), Some(1));
    assert!(output.stderr.is_empty());
    let actual: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(actual["ok"], false);
    assert_eq!(actual["command"], "front doctor");
    assert_eq!(actual["error"]["code"], "UNAUTHORIZED");
}

#[test]
fn version_is_plain_text_for_cli_tooling_compatibility() {
    cargo_bin_cmd!("front")
        .arg("--version")
        .assert()
        .success()
        .stdout(format!("{}\n", env!("CARGO_PKG_VERSION")));
}

#[test]
#[cfg(not(target_os = "windows"))]
fn config_reports_status_without_exposing_a_token() {
    const ENV_TOKEN: &str = "synthetic-environment-token";
    const COMMAND_ARG: &str = "synthetic-token-command-argument";
    let dir = tempdir().unwrap();
    let (mut command, config_path) = isolated_config_command(dir.path());
    fs::create_dir_all(config_path.parent().unwrap()).unwrap();
    fs::write(
        config_path,
        format!(
            "token_command: [definitely-not-executed, {COMMAND_ARG}]\nuser: configured@example.com\n"
        ),
    )
    .unwrap();

    let output = command
        .arg("config")
        .env("FRONT_API_TOKEN", ENV_TOKEN)
        .env("FRONT_USER", "environment@example.com")
        .output()
        .unwrap();

    assert!(output.status.success());
    let actual: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(actual["command"], "front config");
    assert_eq!(actual["result"]["token_command"], "(configured)");
    assert_eq!(actual["result"]["token_source"], "environment");
    assert_eq!(actual["result"]["user"], "environment@example.com");
    assert_eq!(actual["result"]["user_source"], "environment");
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(!stdout.contains(ENV_TOKEN));
    assert!(!stderr.contains(ENV_TOKEN));
    assert!(!stdout.contains(COMMAND_ARG));
    assert!(!stderr.contains(COMMAND_ARG));
}

#[test]
#[cfg(not(target_os = "windows"))]
fn config_does_not_execute_the_configured_token_command() {
    let dir = tempdir().unwrap();
    let (mut command, config_path) = isolated_config_command(dir.path());
    fs::create_dir_all(config_path.parent().unwrap()).unwrap();
    fs::write(
        config_path,
        "token_command: [definitely-not-executed, synthetic-token-command-argument]\n",
    )
    .unwrap();

    let output = command
        .arg("config")
        .env_remove("FRONT_API_TOKEN")
        .output()
        .unwrap();

    assert!(output.status.success());
    let actual: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(actual["result"]["token_source"], "token_command");
}

#[test]
#[cfg(not(target_os = "windows"))]
fn config_omits_token_command_when_it_is_not_configured() {
    let dir = tempdir().unwrap();
    let (mut command, _) = isolated_config_command(dir.path());
    let output = command
        .arg("config")
        .env_remove("FRONT_API_TOKEN")
        .output()
        .unwrap();

    assert!(output.status.success());
    let actual: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert!(actual["result"].get("token_command").is_none());
}

#[test]
#[cfg(not(target_os = "windows"))]
fn malformed_config_does_not_expose_a_sensitive_value() {
    const SENSITIVE: &str = "synthetic-sensitive-value";
    let dir = tempdir().unwrap();
    let (mut command, config_path) = isolated_config_command(dir.path());
    fs::create_dir_all(config_path.parent().unwrap()).unwrap();
    fs::write(config_path, format!("token_command: {SENSITIVE}\n")).unwrap();

    let output = command
        .arg("config")
        .env_remove("FRONT_API_TOKEN")
        .output()
        .unwrap();

    assert_eq!(output.status.code(), Some(1));
    assert!(!String::from_utf8_lossy(&output.stdout).contains(SENSITIVE));
    assert!(!String::from_utf8_lossy(&output.stderr).contains(SENSITIVE));
}

#[test]
#[cfg(not(target_os = "windows"))]
fn malformed_config_labels_normalized_api_command() {
    let dir = tempdir().unwrap();
    let (mut command, config_path) = isolated_config_command(dir.path());
    fs::create_dir_all(config_path.parent().unwrap()).unwrap();
    fs::write(config_path, "token_command: [unterminated\n").unwrap();

    let output = command
        .args(["list", "tags"])
        .env_remove("FRONT_API_TOKEN")
        .output()
        .unwrap();

    assert_eq!(output.status.code(), Some(1));
    let actual: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(actual["command"], "front list tag");
    assert_eq!(actual["error"]["code"], "CONFIG_ERROR");
}

#[test]
#[cfg(not(target_os = "windows"))]
fn unsupported_collection_pagination_is_rejected_before_malformed_config_is_loaded() {
    let dir = tempdir().unwrap();
    let (mut command, config_path) = isolated_config_command(dir.path());
    fs::create_dir_all(config_path.parent().unwrap()).unwrap();
    fs::write(config_path, "token_command: [unterminated\n").unwrap();

    let output = command
        .args(["list", "inboxes", "--limit", "2"])
        .env_remove("FRONT_API_TOKEN")
        .output()
        .unwrap();

    assert_eq!(output.status.code(), Some(1));
    assert!(output.stderr.is_empty());
    let actual: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(actual["command"], "front list inbox");
    assert_eq!(actual["error"]["code"], "INVALID_INPUT");
}

#[test]
fn unknown_command_is_a_json_cli_error() {
    let output = cargo_bin_cmd!("front").arg("bogus").output().unwrap();
    assert_eq!(output.status.code(), Some(1));
    assert!(output.stderr.is_empty());

    let actual: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(actual["ok"], false);
    assert_eq!(actual["command"], "front");
    assert_eq!(actual["error"]["code"], "CLI_ERROR");
    assert_eq!(actual["fix"], "Run 'front' to see available commands");
}

#[test]
fn zero_collection_limits_are_json_cli_errors_before_authentication() {
    const UNRELATED_ENV_VALUE: &str = "synthetic-unrelated-environment-value";
    let cases: [(&[&str], &str); 4] = [
        (
            &["list", "tags", "--limit", "0"],
            "front list tags --limit 0",
        ),
        (
            &["api", "get", "/tags", "--limit", "0"],
            "front api get /tags --limit 0",
        ),
        (
            &["related", "tag", "tag_1", "conversations", "--limit", "0"],
            "front related tag tag_1 conversations --limit 0",
        ),
        (&["inbox", "--limit", "0"], "front inbox --limit 0"),
    ];

    for (args, command_name) in cases {
        let dir = tempdir().unwrap();
        let output = cargo_bin_cmd!("front")
            .args(args)
            .env("HOME", dir.path())
            .env("XDG_CONFIG_HOME", dir.path())
            .env("FRONT_USER", UNRELATED_ENV_VALUE)
            .env_remove("FRONT_API_TOKEN")
            .output()
            .unwrap();

        assert_eq!(output.status.code(), Some(1), "{command_name}");
        assert!(output.stderr.is_empty(), "{command_name}");
        let actual: Value = serde_json::from_slice(&output.stdout)
            .unwrap_or_else(|error| panic!("{command_name} must emit one JSON document: {error}"));
        assert_eq!(actual["ok"], false, "{command_name}");
        assert_eq!(actual["error"]["code"], "CLI_ERROR", "{command_name}");
        assert!(
            actual["error"]["message"]
                .as_str()
                .unwrap()
                .contains("1..=100"),
            "{command_name}"
        );
        assert!(
            !String::from_utf8_lossy(&output.stdout).contains(UNRELATED_ENV_VALUE),
            "{command_name}"
        );
    }
}

#[test]
fn invalid_resource_is_rejected_before_authentication() {
    let dir = tempdir().unwrap();
    let output = cargo_bin_cmd!("front")
        .args(["list", "planets"])
        .env("XDG_CONFIG_HOME", dir.path())
        .env_remove("FRONT_API_TOKEN")
        .output()
        .unwrap();

    assert_eq!(output.status.code(), Some(1));
    let actual: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(actual["command"], "front list");
    assert_eq!(actual["error"]["code"], "INVALID_INPUT");
    assert!(
        actual["error"]["message"]
            .as_str()
            .unwrap()
            .contains("planets")
    );
}

#[test]
fn unsafe_api_path_is_rejected_before_authentication() {
    let dir = tempdir().unwrap();
    let output = cargo_bin_cmd!("front")
        .args(["api", "get", "https://example.com/me"])
        .env("XDG_CONFIG_HOME", dir.path())
        .env_remove("FRONT_API_TOKEN")
        .output()
        .unwrap();

    assert_eq!(output.status.code(), Some(1));
    let actual: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(actual["command"], "front api get");
    assert_eq!(actual["error"]["code"], "INVALID_INPUT");
}

#[test]
fn malformed_query_parameter_is_rejected_before_authentication() {
    let dir = tempdir().unwrap();
    let output = cargo_bin_cmd!("front")
        .args(["list", "tags", "--param", "missing-value"])
        .env("XDG_CONFIG_HOME", dir.path())
        .env_remove("FRONT_API_TOKEN")
        .output()
        .unwrap();

    assert_eq!(output.status.code(), Some(1));
    let actual: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(actual["command"], "front list");
    assert_eq!(actual["error"]["code"], "INVALID_INPUT");
}

#[test]
fn unsupported_collection_pagination_flags_are_rejected_before_authentication() {
    let resources = [
        ("channels", "channel"),
        ("inboxes", "inbox"),
        ("knowledge-bases", "knowledge-base"),
        ("message-templates", "message-template"),
        ("message-template-folders", "message-template-folder"),
        ("rules", "rule"),
        ("shifts", "shift"),
        ("teammates", "teammate"),
        ("teammate-groups", "teammate-group"),
        ("teams", "team"),
    ];
    let flags = [("--limit", "2"), ("--page-token", "next")];

    for (resource, canonical_resource) in resources {
        for (flag, value) in flags {
            let dir = tempdir().unwrap();
            let output = cargo_bin_cmd!("front")
                .args(["list", resource, flag, value])
                .env("HOME", dir.path())
                .env("XDG_CONFIG_HOME", dir.path())
                .env_remove("FRONT_API_TOKEN")
                .output()
                .unwrap();

            let case = format!("front list {resource} {flag} {value}");
            assert_eq!(output.status.code(), Some(1), "{case}");
            assert!(output.stderr.is_empty(), "{case}");
            let actual: Value = serde_json::from_slice(&output.stdout)
                .unwrap_or_else(|error| panic!("{case} must emit one JSON document: {error}"));
            assert_eq!(
                actual["command"],
                format!("front list {canonical_resource}"),
                "{case}"
            );
            assert_eq!(actual["error"]["code"], "INVALID_INPUT", "{case}");
            assert!(
                actual["error"]["message"]
                    .as_str()
                    .is_some_and(|message| message.contains(flag)),
                "{case}"
            );
        }
    }
}

#[test]
#[cfg(not(target_os = "windows"))]
fn authentication_failures_report_the_requested_command() {
    let whoami_dir = tempdir().unwrap();
    let (mut whoami_command, _) = isolated_config_command(whoami_dir.path());
    let whoami_output = whoami_command
        .arg("whoami")
        .env_remove("FRONT_API_TOKEN")
        .output()
        .unwrap();

    let list_dir = tempdir().unwrap();
    let (mut list_command, _) = isolated_config_command(list_dir.path());
    let list_output = list_command
        .args(["list", "tags"])
        .env_remove("FRONT_API_TOKEN")
        .output()
        .unwrap();

    assert_eq!(whoami_output.status.code(), Some(1));
    assert_eq!(list_output.status.code(), Some(1));

    let whoami_error: Value = serde_json::from_slice(&whoami_output.stdout).unwrap();
    let list_error: Value = serde_json::from_slice(&list_output.stdout).unwrap();
    assert_eq!(whoami_error["command"], "front whoami");
    assert_eq!(list_error["command"], "front list tag");
    assert_eq!(whoami_error["error"]["code"], "UNAUTHORIZED");
}

#[test]
fn completion_generation_is_plain_text_and_does_not_require_authentication() {
    let dir = tempdir().unwrap();
    let output = cargo_bin_cmd!("front")
        .args(["completion", "bash"])
        .env("XDG_CONFIG_HOME", dir.path())
        .env_remove("FRONT_API_TOKEN")
        .output()
        .unwrap();

    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(stdout.contains("_front"));
    assert!(stdout.contains("complete"));
}
