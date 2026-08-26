use std::{collections::BTreeMap, fs};

use frontmail_cli::config::{
    Config, ConfigSource, Profile, ProfileSource, load_from, resolve_token, resolve_user,
    select_effective_config, token_source, user_source,
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
        ..Config::default()
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
        ..Config::default()
    };
    let env = BTreeMap::from([("FRONT_API_TOKEN".into(), "environment-token".into())]);

    assert_eq!(token_source(&config, &env), ConfigSource::Environment);
}

#[test]
fn token_source_uses_a_configured_command_when_environment_is_empty() {
    let config = Config {
        token_command: vec!["printf".into(), "command-token".into()],
        user: String::new(),
        ..Config::default()
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
        ..Config::default()
    };

    let token = resolve_token(&config, &BTreeMap::new()).unwrap();
    assert_eq!(token.expose_secret(), "command-token");
}

#[test]
fn environment_user_has_priority_over_config() {
    let config = Config {
        token_command: vec![],
        user: "configured@example.com".into(),
        ..Config::default()
    };
    let env = BTreeMap::from([("FRONT_USER".into(), "environment@example.com".into())]);
    assert_eq!(resolve_user(&config, &env), "environment@example.com");
}

#[test]
fn user_source_prefers_a_non_empty_environment_value() {
    let config = Config {
        token_command: vec![],
        user: "configured@example.com".into(),
        ..Config::default()
    };
    let env = BTreeMap::from([("FRONT_USER".into(), "environment@example.com".into())]);

    assert_eq!(user_source(&config, &env), ConfigSource::Environment);
}

#[test]
fn user_source_uses_config_when_environment_is_empty() {
    let config = Config {
        token_command: vec![],
        user: "configured@example.com".into(),
        ..Config::default()
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

fn profile(token_command: &[&str], user: &str) -> Profile {
    Profile {
        token_command: token_command.iter().map(|value| (*value).into()).collect(),
        user: user.into(),
    }
}

#[test]
fn yaml_config_loads_named_profiles_without_changing_legacy_fields() {
    let dir = tempdir().unwrap();
    let path = dir.path().join("config.yaml");
    fs::write(
        &path,
        "token_command: [legacy-command]\nuser: legacy-user@example.invalid\ndefault_profile: work\nprofiles:\n  work:\n    token_command: [profile-command, \"argument with spaces\"]\n    user: profile-user@example.invalid\n",
    )
    .unwrap();

    let config = load_from(&path).unwrap();

    assert_eq!(config.token_command, ["legacy-command"]);
    assert_eq!(config.user, "legacy-user@example.invalid");
    assert_eq!(config.default_profile.as_deref(), Some("work"));
    assert_eq!(
        config.profiles.get("work"),
        Some(&profile(
            &["profile-command", "argument with spaces"],
            "profile-user@example.invalid"
        ))
    );
}

#[test]
fn explicit_profile_ignores_ambient_and_legacy_credentials() {
    let config = Config {
        token_command: vec!["legacy-command".into()],
        user: "legacy-user@example.invalid".into(),
        profiles: BTreeMap::from([(
            "work".into(),
            profile(&["profile-command"], "profile-user@example.invalid"),
        )]),
        ..Config::default()
    };
    let env = BTreeMap::from([
        ("FRONT_API_TOKEN".into(), "ambient-token".into()),
        ("FRONT_USER".into(), "ambient-user@example.invalid".into()),
    ]);

    let selected = select_effective_config(&config, &env, Some("work")).unwrap();

    assert_eq!(selected.profile_name(), Some("work"));
    assert_eq!(selected.profile_source(), Some(ProfileSource::Explicit));
    assert_eq!(selected.token_source(), ConfigSource::TokenCommand);
    assert_eq!(selected.user_source(), ConfigSource::Config);
    assert_eq!(selected.user(), "profile-user@example.invalid");
}

#[test]
fn configured_default_profile_is_selected_without_legacy_sources() {
    let config = Config {
        default_profile: Some("second".into()),
        profiles: BTreeMap::from([
            ("first".into(), Profile::default()),
            ("second".into(), profile(&[], "second@example.invalid")),
        ]),
        ..Config::default()
    };

    let selected = select_effective_config(&config, &BTreeMap::new(), None).unwrap();

    assert_eq!(selected.profile_name(), Some("second"));
    assert_eq!(selected.profile_source(), Some(ProfileSource::Default));
    assert_eq!(selected.user(), "second@example.invalid");
}

#[test]
fn exactly_one_profile_is_selected_automatically() {
    let config = Config {
        profiles: BTreeMap::from([(
            "only".into(),
            profile(&["profile-command"], "only@example.invalid"),
        )]),
        ..Config::default()
    };

    let selected = select_effective_config(&config, &BTreeMap::new(), None).unwrap();

    assert_eq!(selected.profile_name(), Some("only"));
    assert_eq!(selected.profile_source(), Some(ProfileSource::Single));
}

#[test]
fn multiple_profiles_without_a_default_require_a_profile_name() {
    let config = Config {
        profiles: BTreeMap::from([
            ("first".into(), Profile::default()),
            ("second".into(), Profile::default()),
        ]),
        ..Config::default()
    };

    let error = select_effective_config(&config, &BTreeMap::new(), None).unwrap_err();
    let message = error.to_string();

    assert_eq!(error.code(), "CONFIG_ERROR");
    assert!(message.contains("first"));
    assert!(message.contains("second"));
}

#[test]
fn unknown_explicit_profile_lists_names_without_profile_values() {
    const COMMAND_VALUE: &str = "private-command-value";
    const USER_VALUE: &str = "private-user-value@example.invalid";
    let config = Config {
        profiles: BTreeMap::from([(
            "work-profile".into(),
            profile(&["program", COMMAND_VALUE], USER_VALUE),
        )]),
        ..Config::default()
    };

    let error = select_effective_config(&config, &BTreeMap::new(), Some("missing")).unwrap_err();
    let message = error.to_string();

    assert_eq!(error.code(), "CONFIG_ERROR");
    assert!(message.contains("missing"));
    assert!(message.contains("work-profile"));
    assert!(!message.contains(COMMAND_VALUE));
    assert!(!message.contains(USER_VALUE));
    assert!(!format!("{error:?}").contains(COMMAND_VALUE));
    assert!(!format!("{error:?}").contains(USER_VALUE));
}

#[test]
fn unknown_default_profile_is_a_config_error_before_single_profile_auto_selection() {
    let config = Config {
        default_profile: Some("missing".into()),
        profiles: BTreeMap::from([("work-profile".into(), Profile::default())]),
        ..Config::default()
    };

    let error = select_effective_config(&config, &BTreeMap::new(), None).unwrap_err();
    let message = error.to_string();

    assert_eq!(error.code(), "CONFIG_ERROR");
    assert!(message.contains("missing"));
    assert!(message.contains("work-profile"));
}

#[test]
fn either_legacy_source_prevents_profile_fallback() {
    let user_only = Config {
        user: "legacy-user@example.invalid".into(),
        profiles: BTreeMap::from([(
            "profile".into(),
            profile(&["profile-command"], "profile-user@example.invalid"),
        )]),
        ..Config::default()
    };
    let selected = select_effective_config(&user_only, &BTreeMap::new(), None).unwrap();
    assert_eq!(selected.profile_name(), None);
    assert_eq!(selected.token_source(), ConfigSource::None);
    assert_eq!(selected.user_source(), ConfigSource::Config);
    assert_eq!(selected.user(), "legacy-user@example.invalid");

    let env = BTreeMap::from([("FRONT_API_TOKEN".into(), "ambient-token".into())]);
    let env_token_only = Config {
        profiles: BTreeMap::from([(
            "profile".into(),
            profile(&[], "profile-user@example.invalid"),
        )]),
        ..Config::default()
    };
    let selected = select_effective_config(&env_token_only, &env, None).unwrap();
    assert_eq!(selected.profile_name(), None);
    assert_eq!(selected.token_source(), ConfigSource::Environment);
    assert_eq!(selected.user_source(), ConfigSource::None);
    assert_eq!(selected.user(), "");
}

#[test]
fn zero_profiles_preserves_the_legacy_no_token_result() {
    let selected = select_effective_config(&Config::default(), &BTreeMap::new(), None).unwrap();

    assert_eq!(selected.profile_name(), None);
    assert_eq!(selected.token_source(), ConfigSource::None);
    assert_eq!(selected.resolve_token().unwrap_err().code(), "UNAUTHORIZED");
}

#[test]
fn selected_profile_token_command_preserves_an_argument_with_spaces() {
    #[cfg(target_os = "windows")]
    let (_script_dir, token_command) = {
        let script_dir = tempfile::tempdir().unwrap();
        let script = script_dir.path().join("print-token.ps1");
        fs::write(
            &script,
            "param([string]$Value)\n[Console]::Out.Write($Value)\n",
        )
        .unwrap();
        let command = vec![
            "powershell.exe".into(),
            "-NoProfile".into(),
            "-NonInteractive".into(),
            "-File".into(),
            script.to_string_lossy().into_owned(),
            "profile token with spaces".into(),
        ];
        (script_dir, command)
    };
    #[cfg(not(target_os = "windows"))]
    let token_command = vec!["printf".into(), "profile token with spaces\\n".into()];
    let config = Config {
        profiles: BTreeMap::from([(
            "work".into(),
            Profile {
                token_command,
                user: "profile-user@example.invalid".into(),
            },
        )]),
        ..Config::default()
    };

    let selected = select_effective_config(&config, &BTreeMap::new(), Some("work")).unwrap();
    let token = selected.resolve_token().unwrap();

    assert_eq!(token.expose_secret(), "profile token with spaces");
}

#[test]
fn blank_profile_keys_are_rejected_before_auto_default_or_multiple_selection() {
    const PROFILE_VALUE: &str = "profile-value-must-not-appear";
    let cases = [
        Config {
            profiles: BTreeMap::from([(
                String::new(),
                profile(&["program", PROFILE_VALUE], PROFILE_VALUE),
            )]),
            ..Config::default()
        },
        Config {
            default_profile: Some(String::new()),
            profiles: BTreeMap::from([(
                String::new(),
                profile(&["program", PROFILE_VALUE], PROFILE_VALUE),
            )]),
            ..Config::default()
        },
        Config {
            profiles: BTreeMap::from([
                (
                    String::new(),
                    profile(&["program", PROFILE_VALUE], PROFILE_VALUE),
                ),
                ("work".into(), Profile::default()),
            ]),
            ..Config::default()
        },
        Config {
            profiles: BTreeMap::from([(
                " \t ".into(),
                profile(&["program", PROFILE_VALUE], PROFILE_VALUE),
            )]),
            ..Config::default()
        },
    ];

    for config in cases {
        let error = select_effective_config(&config, &BTreeMap::new(), None).unwrap_err();
        let message = error.to_string();
        assert_eq!(error.code(), "CONFIG_ERROR");
        assert!(message.contains("profile names"));
        assert!(message.contains("non-whitespace"));
        assert!(!message.contains(PROFILE_VALUE));
        assert!(!format!("{error:?}").contains(PROFILE_VALUE));
    }
}

#[test]
fn blank_default_profile_values_are_rejected() {
    for default_profile in ["", " \t "] {
        let config = Config {
            default_profile: Some(default_profile.into()),
            profiles: BTreeMap::from([("work".into(), Profile::default())]),
            ..Config::default()
        };

        let error = select_effective_config(&config, &BTreeMap::new(), None).unwrap_err();
        let message = error.to_string();
        assert_eq!(error.code(), "CONFIG_ERROR");
        assert!(message.contains("default_profile"));
        assert!(message.contains("non-whitespace"));
    }
}

#[test]
fn blank_explicit_profile_values_are_rejected() {
    let config = Config {
        profiles: BTreeMap::from([("work".into(), Profile::default())]),
        ..Config::default()
    };

    for explicit in ["", " \t "] {
        let error = select_effective_config(&config, &BTreeMap::new(), Some(explicit)).unwrap_err();
        let message = error.to_string();
        assert_eq!(error.code(), "CONFIG_ERROR");
        assert!(message.contains("--profile"));
        assert!(message.contains("non-whitespace"));
    }
}

#[test]
fn non_blank_profile_names_are_not_trimmed_or_aliased() {
    let config = Config {
        profiles: BTreeMap::from([(" work ".into(), Profile::default())]),
        ..Config::default()
    };

    let selected = select_effective_config(&config, &BTreeMap::new(), Some(" work ")).unwrap();
    assert_eq!(selected.profile_name(), Some(" work "));

    let error = select_effective_config(&config, &BTreeMap::new(), Some("work")).unwrap_err();
    assert!(error.to_string().contains(" work "));
}
