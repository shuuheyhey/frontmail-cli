use std::{collections::BTreeMap, fs};

use frontmail_cli::config::{
    Config, ConfigSource, load_from, resolve_token, resolve_user, token_source, user_source,
};
use secrecy::ExposeSecret;
use tempfile::tempdir;

#[test]
fn missing_config_is_the_empty_configuration() {
    let dir = tempdir().unwrap();
    let config = load_from(&dir.path().join("missing.yaml")).unwrap();
    assert_eq!(config, Config::default());
}

#[test]
fn yaml_config_is_loaded_without_embedding_a_token() {
    let dir = tempdir().unwrap();
    let path = dir.path().join("config.yaml");
    fs::write(
        &path,
        "token_command:\n  - printf\n  - command-token\\n\nuser: configured@example.com\n",
    )
    .unwrap();

    let config = load_from(&path).unwrap();
    assert_eq!(config.token_command, ["printf", "command-token\\n"]);
    assert_eq!(config.user, "configured@example.com");
}

#[test]
fn malformed_config_error_does_not_expose_the_invalid_value() {
    const SENSITIVE: &str = "synthetic-sensitive-value";
    let dir = tempdir().unwrap();
    let path = dir.path().join("config.yaml");
    fs::write(&path, format!("token_command: {SENSITIVE}\n")).unwrap();

    let error = load_from(&path).unwrap_err();

    assert!(!error.to_string().contains(SENSITIVE));
    assert!(!format!("{error:?}").contains(SENSITIVE));
    let mut source = std::error::Error::source(&error);
    while let Some(cause) = source {
        assert!(!cause.to_string().contains(SENSITIVE));
        assert!(!format!("{cause:?}").contains(SENSITIVE));
        source = cause.source();
    }
}

#[test]
fn environment_token_has_priority_over_token_command() {
    let config = Config {
        token_command: vec!["printf".into(), "command-token".into()],
        user: String::new(),
    };
    let env = BTreeMap::from([("FRONT_API_TOKEN".into(), "environment-token".into())]);

    let token = resolve_token(&config, &env).unwrap();
    assert_eq!(token.expose_secret(), "environment-token");
}

#[test]
fn token_source_prefers_a_non_empty_environment_value() {
    let config = Config {
        token_command: vec!["printf".into(), "command-token".into()],
        user: String::new(),
    };
    let env = BTreeMap::from([("FRONT_API_TOKEN".into(), "environment-token".into())]);

    assert_eq!(token_source(&config, &env), ConfigSource::Environment);
}

#[test]
fn token_source_uses_a_configured_command_when_environment_is_empty() {
    let config = Config {
        token_command: vec!["printf".into(), "command-token".into()],
        user: String::new(),
    };
    let env = BTreeMap::from([("FRONT_API_TOKEN".into(), String::new())]);

    assert_eq!(token_source(&config, &env), ConfigSource::TokenCommand);
}

#[test]
fn token_source_is_none_without_an_environment_value_or_command() {
    assert_eq!(
        token_source(&Config::default(), &BTreeMap::new()),
        ConfigSource::None
    );
}

#[test]
fn token_command_is_executed_as_argv_and_trimmed() {
    #[cfg(target_os = "windows")]
    let token_command = vec![
        "cmd".into(),
        "/C".into(),
        "echo".into(),
        "command-token".into(),
    ];
    #[cfg(not(target_os = "windows"))]
    let token_command = vec!["printf".into(), "command-token\\n".into()];
    let config = Config {
        token_command,
        user: String::new(),
    };

    let token = resolve_token(&config, &BTreeMap::new()).unwrap();
    assert_eq!(token.expose_secret(), "command-token");
}

#[test]
fn environment_user_has_priority_over_config() {
    let config = Config {
        token_command: vec![],
        user: "configured@example.com".into(),
    };
    let env = BTreeMap::from([("FRONT_USER".into(), "environment@example.com".into())]);
    assert_eq!(resolve_user(&config, &env), "environment@example.com");
}

#[test]
fn user_source_prefers_a_non_empty_environment_value() {
    let config = Config {
        token_command: vec![],
        user: "configured@example.com".into(),
    };
    let env = BTreeMap::from([("FRONT_USER".into(), "environment@example.com".into())]);

    assert_eq!(user_source(&config, &env), ConfigSource::Environment);
}

#[test]
fn user_source_uses_config_when_environment_is_empty() {
    let config = Config {
        token_command: vec![],
        user: "configured@example.com".into(),
    };
    let env = BTreeMap::from([("FRONT_USER".into(), String::new())]);

    assert_eq!(user_source(&config, &env), ConfigSource::Config);
    assert_eq!(resolve_user(&config, &env), "configured@example.com");
}

#[test]
fn user_source_is_none_without_an_environment_value_or_configured_user() {
    assert_eq!(
        user_source(&Config::default(), &BTreeMap::new()),
        ConfigSource::None
    );
}
