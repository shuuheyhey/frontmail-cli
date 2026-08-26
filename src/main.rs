use clap::{CommandFactory, Parser, error::ErrorKind};
use clap_complete::generate;
use frontmail_cli::{
    VERSION,
    cli::{Cli, Commands, prepare_read_request},
    client::{ClientError, FrontClient, classify_http},
    commands::{
        CommandError, DoctorAuthenticationError, InboxOptions, ReadRequest, doctor_json,
        execute_read, inbox_json, inboxes_json, read_json,
    },
    config, envelope,
    resources::ResourceError,
};
use reqwest::StatusCode;
use serde::Serialize;
use std::ffi::OsStr;

#[derive(Serialize)]
struct ConfigResult {
    path: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    profile: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    profile_source: Option<config::ProfileSource>,
    #[serde(skip_serializing_if = "Option::is_none")]
    token_command: Option<&'static str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    user: Option<String>,
    token_source: config::ConfigSource,
    user_source: config::ConfigSource,
}

#[tokio::main]
async fn main() {
    let raw_args: Vec<_> = std::env::args_os().collect();
    let query_was_set = flag_was_set(&raw_args, "--query");
    let limit_was_set = flag_was_set(&raw_args, "--limit");
    let cli = match Cli::try_parse_from(raw_args) {
        Ok(cli) => cli,
        Err(error)
            if matches!(
                error.kind(),
                ErrorKind::DisplayHelp | ErrorKind::DisplayVersion
            ) =>
        {
            print!("{error}");
            return;
        }
        Err(error) => {
            let message = error.to_string().trim().to_owned();
            let fix = if error.kind() == ErrorKind::InvalidSubcommand {
                "Run 'front' to see available commands"
            } else {
                "Run 'front' to see available commands and their required arguments"
            };
            print_failure("front", message, "CLI_ERROR", fix);
            std::process::exit(1);
        }
    };

    if cli.version {
        println!("{}", frontmail_cli::VERSION);
        return;
    }

    let profile = cli.profile;

    match cli.command {
        None => print_json(frontmail_cli::root_json()),
        Some(Commands::Config) => run_config(profile.as_deref()),
        Some(Commands::Completion(args)) => {
            generate(
                args.shell,
                &mut Cli::command(),
                "front",
                &mut std::io::stdout(),
            );
        }
        Some(command) => {
            let request = match prepare_read_request(&command) {
                Ok(request) => request,
                Err(error) => {
                    let command = match &error {
                        ResourceError::UnsupportedCollectionQueryParameter { resource, .. } => {
                            format!("front list {resource}")
                        }
                        _ => command_name(&command).into(),
                    };
                    print_failure(
                        command,
                        error.to_string(),
                        "INVALID_INPUT",
                        "Run 'front' to see supported resources and path rules",
                    );
                    std::process::exit(1);
                }
            };
            run_api_command(
                command,
                request,
                profile.as_deref(),
                query_was_set,
                limit_was_set,
            )
            .await;
        }
    }
}

async fn run_api_command(
    command: Commands,
    request: Option<ReadRequest>,
    profile: Option<&str>,
    query_was_set: bool,
    limit_was_set: bool,
) {
    let requested_command = request
        .as_ref()
        .map(|request| request.command.clone())
        .unwrap_or_else(|| command_name(&command).to_owned());
    let config_path = config::path();
    let loaded = match config::load_from(&config_path) {
        Ok(config) => config,
        Err(error) => {
            print_failure(
                &requested_command,
                error.to_string(),
                "CONFIG_ERROR",
                "Check config file syntax",
            );
            std::process::exit(1);
        }
    };
    let env = config::current_env();
    let selected = match config::select_effective_config(&loaded, &env, profile) {
        Ok(selected) => selected,
        Err(error) => {
            print_failure(
                &requested_command,
                error.to_string(),
                "CONFIG_ERROR",
                "Choose a configured profile or update default_profile",
            );
            std::process::exit(1);
        }
    };
    let auth_context = selected.auth_context();
    let token = match selected.resolve_token() {
        Ok(token) => token,
        Err(error) => {
            let fix = authentication_fix(auth_context, &config_path);
            print_failure(&requested_command, error.to_string(), "UNAUTHORIZED", fix);
            std::process::exit(1);
        }
    };
    let client = match FrontClient::production(token, format!("front-cli/{VERSION}")) {
        Ok(client) => client,
        Err(error) => {
            print_command_error(
                &requested_command,
                CommandError::Client(error),
                &config_path,
                auth_context,
            );
            return;
        }
    };

    let (command_name, result) = if let Some(request) = request {
        let command_name = request.command.clone();
        (command_name, execute_read(&client, request).await)
    } else {
        match command {
            Commands::Doctor => (
                "front doctor".into(),
                doctor_json(
                    &client,
                    selected.token_source(),
                    selected.user_source(),
                    selected.user(),
                )
                .await,
            ),
            Commands::Inboxes => (
                "front inboxes".into(),
                inboxes_json(&client, selected.user()).await,
            ),
            Commands::Inbox(args) => {
                let options = InboxOptions {
                    inbox_id: args.inbox_id,
                    query: args.query,
                    query_was_set,
                    from: args.from,
                    assignee: args.assignee,
                    before: args.before,
                    after: args.after,
                    limit: args.limit,
                    limit_was_set,
                    page_token: args.page_token,
                };
                ("front inbox".into(), inbox_json(&client, &options).await)
            }
            Commands::Read(args) => (
                "front read".into(),
                read_json(&client, &args.conversation_id).await,
            ),
            Commands::Config
            | Commands::Whoami
            | Commands::Api(_)
            | Commands::List(_)
            | Commands::Get(_)
            | Commands::Related(_)
            | Commands::Completion(_) => unreachable!("command handled before API client creation"),
        }
    };

    match result {
        Ok(json) => print!("{json}"),
        Err(error) => print_command_error(&command_name, error, &config_path, auth_context),
    }
}

fn command_name(command: &Commands) -> &'static str {
    match command {
        Commands::Config => "front config",
        Commands::Doctor => "front doctor",
        Commands::Inboxes => "front inboxes",
        Commands::Inbox(_) => "front inbox",
        Commands::Read(_) => "front read",
        Commands::Whoami => "front whoami",
        Commands::Api(_) => "front api get",
        Commands::List(_) => "front list",
        Commands::Get(_) => "front get",
        Commands::Related(_) => "front related",
        Commands::Completion(_) => "front completion",
    }
}

struct CommandErrorPresentation<'a> {
    command: &'a str,
    code: &'static str,
    fix: String,
}

fn authentication_fix(context: config::AuthContext<'_>, config_path: &std::path::Path) -> String {
    match context {
        config::AuthContext::Legacy => format!(
            "Set FRONT_API_TOKEN or configure token_command in {}",
            config_path.display()
        ),
        config::AuthContext::NamedProfile(name) => format!(
            "Configure token_command for profile {name:?} in {}",
            config_path.display()
        ),
    }
}

fn command_error_presentation<'a>(
    command: &'a str,
    error: &CommandError,
    config_path: &std::path::Path,
    auth_context: config::AuthContext<'_>,
) -> CommandErrorPresentation<'a> {
    let (code, fix) = match &error {
        CommandError::InvalidDate { .. } => (
            "INVALID_INPUT",
            "Use YYYY-MM-DD format for --before and --after flags".into(),
        ),
        CommandError::DoctorAuthentication(DoctorAuthenticationError::Http { status })
        | CommandError::Client(ClientError::Http { status, .. }) => {
            let status = StatusCode::from_u16(*status).unwrap_or(StatusCode::INTERNAL_SERVER_ERROR);
            let (code, mut fix) = classify_http(status);
            if status == StatusCode::UNAUTHORIZED {
                fix = authentication_fix(auth_context, config_path);
            }
            (code, fix)
        }
        CommandError::DoctorAuthentication(DoctorAuthenticationError::Transport)
        | CommandError::Client(ClientError::Transport(_)) => {
            ("TRANSPORT_ERROR", "Check network connectivity".into())
        }
        CommandError::DoctorAuthentication(DoctorAuthenticationError::Decode)
        | CommandError::Client(ClientError::Decode(_)) => {
            ("API_ERROR", "API returned an invalid response".into())
        }
        CommandError::DoctorAuthentication(DoctorAuthenticationError::ClientConfiguration)
        | CommandError::Client(ClientError::Build(_) | ClientError::InvalidBaseUrl) => {
            ("CONFIG_ERROR", "Check API client configuration".into())
        }
        CommandError::Serialize(_) => ("INTERNAL_ERROR", "Retry the command".into()),
    };
    CommandErrorPresentation { command, code, fix }
}

fn command_error_json(
    command: &str,
    error: &CommandError,
    config_path: &std::path::Path,
    auth_context: config::AuthContext<'_>,
) -> serde_json::Result<String> {
    let presentation = command_error_presentation(command, error, config_path, auth_context);
    envelope::failure(
        presentation.command,
        error.to_string(),
        presentation.code,
        presentation.fix,
        vec![],
    )
}

fn print_command_error(
    command: &str,
    error: CommandError,
    config_path: &std::path::Path,
    auth_context: config::AuthContext<'_>,
) {
    print_json(command_error_json(
        command,
        &error,
        config_path,
        auth_context,
    ));
    std::process::exit(1);
}

fn flag_was_set(args: &[std::ffi::OsString], flag: &str) -> bool {
    args.iter().any(|arg| {
        arg == OsStr::new(flag)
            || arg
                .to_str()
                .is_some_and(|arg| arg.starts_with(&format!("{flag}=")))
    })
}

fn run_config(profile: Option<&str>) {
    let path = config::path();
    match config::load_from(&path) {
        Ok(loaded) => {
            let env = config::current_env();
            let selected = match config::select_effective_config(&loaded, &env, profile) {
                Ok(selected) => selected,
                Err(error) => {
                    print_failure(
                        "front config",
                        error.to_string(),
                        "CONFIG_ERROR",
                        "Choose a configured profile or update default_profile",
                    );
                    std::process::exit(1);
                }
            };
            let token_command = selected
                .token_command_configured()
                .then_some("(configured)");
            let profile = selected.profile_name().map(str::to_owned);
            let profile_source = selected.profile_source();
            let user = profile.is_none().then(|| selected.user().to_owned());
            print_json(envelope::success(
                "front config",
                ConfigResult {
                    path: path.display().to_string(),
                    profile,
                    profile_source,
                    token_command,
                    user,
                    token_source: selected.token_source(),
                    user_source: selected.user_source(),
                },
                vec![],
            ));
        }
        Err(error) => {
            print_failure(
                "front config",
                error.to_string(),
                "CONFIG_ERROR",
                "Check config file syntax",
            );
            std::process::exit(1);
        }
    }
}

fn print_json(json: serde_json::Result<String>) {
    match json {
        Ok(json) => print!("{json}"),
        Err(error) => {
            eprintln!("{error}");
            std::process::exit(1);
        }
    }
}

fn print_failure(
    command: impl Into<String>,
    message: impl Into<String>,
    code: impl Into<String>,
    fix: impl Into<String>,
) {
    print_json(envelope::failure(command, message, code, fix, vec![]));
}

#[cfg(test)]
mod tests {
    use std::{collections::BTreeMap, path::Path};

    use super::*;
    use secrecy::SecretString;
    use serde_json::Value;
    use tempfile::tempdir;
    use wiremock::{
        Mock, MockServer, ResponseTemplate,
        matchers::{method, path},
    };

    #[tokio::test]
    async fn doctor_failure_json_is_sanitized_and_preserves_http_classification() {
        for (status, expected_code) in [
            (401, "UNAUTHORIZED"),
            (403, "FORBIDDEN"),
            (418, "API_ERROR"),
        ] {
            let server = MockServer::start().await;
            let private_marker = format!("synthetic-private-doctor-body-{status}");
            Mock::given(method("GET"))
                .and(path("/me"))
                .respond_with(
                    ResponseTemplate::new(status).set_body_json(serde_json::json!({
                        "_error": {
                            "message": private_marker.clone(),
                            "id": "synthetic-private-id",
                            "name": "synthetic-private-name"
                        }
                    })),
                )
                .mount(&server)
                .await;
            let client = FrontClient::new(
                server.uri().parse().unwrap(),
                SecretString::from("synthetic-doctor-token"),
                "front/test",
            )
            .unwrap();

            let error = doctor_json(
                &client,
                config::ConfigSource::Environment,
                config::ConfigSource::None,
                "",
            )
            .await
            .unwrap_err();
            let json = command_error_json(
                "front doctor",
                &error,
                &tempdir().unwrap().path().join("config.yaml"),
                config::AuthContext::Legacy,
            )
            .unwrap();
            let actual: Value = serde_json::from_str(&json).unwrap();

            assert_eq!(actual["command"], "front doctor");
            assert_eq!(actual["error"]["code"], expected_code);
            assert_eq!(
                actual["error"]["message"],
                format!("doctor authentication check failed (HTTP {status})")
            );
            for marker in [
                private_marker.as_str(),
                "synthetic-private-id",
                "synthetic-private-name",
            ] {
                assert!(!json.contains(marker), "leaked {marker:?}");
            }
        }
    }

    fn unauthorized_error() -> CommandError {
        CommandError::Client(ClientError::Http {
            status: StatusCode::UNAUTHORIZED.as_u16(),
            message: "authentication rejected".into(),
        })
    }

    #[test]
    fn named_profile_http_401_keeps_canonical_metadata_and_profile_fix() {
        const PROFILE_USER: &str = "profile-user-must-not-appear";
        const COMMAND_ARG: &str = "profile-command-argument-must-not-appear";
        const AMBIENT_TOKEN: &str = "ambient-token-must-not-appear";
        let loaded = config::Config {
            profiles: BTreeMap::from([(
                "work".into(),
                config::Profile {
                    token_command: vec!["program".into(), COMMAND_ARG.into()],
                    user: PROFILE_USER.into(),
                },
            )]),
            ..config::Config::default()
        };
        let env = BTreeMap::from([("FRONT_API_TOKEN".into(), AMBIENT_TOKEN.into())]);
        let selected = config::select_effective_config(&loaded, &env, Some("work")).unwrap();
        let error = unauthorized_error();
        let presentation = command_error_presentation(
            "front list tag",
            &error,
            Path::new("/config/front/config.yaml"),
            selected.auth_context(),
        );

        assert_eq!(presentation.command, "front list tag");
        assert_eq!(presentation.code, "UNAUTHORIZED");
        assert_eq!(
            presentation.fix,
            "Configure token_command for profile \"work\" in /config/front/config.yaml"
        );
        for value in [PROFILE_USER, COMMAND_ARG, AMBIENT_TOKEN] {
            assert!(!presentation.fix.contains(value));
        }
    }

    #[test]
    fn legacy_http_401_fix_remains_byte_for_byte_compatible() {
        let loaded = config::Config::default();
        let env = BTreeMap::from([("FRONT_API_TOKEN".into(), "legacy-token".into())]);
        let selected = config::select_effective_config(&loaded, &env, None).unwrap();
        let error = unauthorized_error();
        let presentation = command_error_presentation(
            "front inboxes",
            &error,
            Path::new("/config/front/config.yaml"),
            selected.auth_context(),
        );

        assert_eq!(presentation.command, "front inboxes");
        assert_eq!(presentation.code, "UNAUTHORIZED");
        assert_eq!(
            presentation.fix,
            "Set FRONT_API_TOKEN or configure token_command in /config/front/config.yaml"
        );
    }
}
