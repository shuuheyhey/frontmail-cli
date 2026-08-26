use clap::{CommandFactory, Parser, error::ErrorKind};
use clap_complete::generate;
use frontmail_cli::{
    VERSION,
    cli::{Cli, Commands, prepare_read_request},
    client::{ClientError, FrontClient, classify_http},
    commands::{
        CommandError, InboxOptions, ReadRequest, execute_read, inbox_json, inboxes_json, read_json,
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
    token_command: Option<&'static str>,
    user: String,
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

    match cli.command {
        None => print_json(frontmail_cli::root_json()),
        Some(Commands::Config) => run_config(),
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
            run_api_command(command, request, query_was_set, limit_was_set).await;
        }
    }
}

async fn run_api_command(
    command: Commands,
    request: Option<ReadRequest>,
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
    let token = match config::resolve_token(&loaded, &env) {
        Ok(token) => token,
        Err(error) => {
            print_failure(
                &requested_command,
                error.to_string(),
                "UNAUTHORIZED",
                format!(
                    "Set FRONT_API_TOKEN or configure token_command in {}",
                    config_path.display()
                ),
            );
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
            );
            return;
        }
    };

    let (command_name, result) = if let Some(request) = request {
        let command_name = request.command.clone();
        (command_name, execute_read(&client, request).await)
    } else {
        match command {
            Commands::Inboxes => {
                let user = config::resolve_user(&loaded, &env);
                ("front inboxes".into(), inboxes_json(&client, &user).await)
            }
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
        Err(error) => print_command_error(&command_name, error, &config_path),
    }
}

fn command_name(command: &Commands) -> &'static str {
    match command {
        Commands::Config => "front config",
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

fn print_command_error(command: &str, error: CommandError, config_path: &std::path::Path) {
    let (code, fix) = match &error {
        CommandError::InvalidDate { .. } => (
            "INVALID_INPUT",
            "Use YYYY-MM-DD format for --before and --after flags".into(),
        ),
        CommandError::Client(ClientError::Http { status, .. }) => {
            let status = StatusCode::from_u16(*status).unwrap_or(StatusCode::INTERNAL_SERVER_ERROR);
            let (code, mut fix) = classify_http(status);
            if status == StatusCode::UNAUTHORIZED {
                fix = format!(
                    "Set FRONT_API_TOKEN or configure token_command in {}",
                    config_path.display()
                );
            }
            (code, fix)
        }
        CommandError::Client(ClientError::Transport(_)) => {
            ("TRANSPORT_ERROR", "Check network connectivity".into())
        }
        CommandError::Client(ClientError::Decode(_)) => {
            ("API_ERROR", "API returned an invalid response".into())
        }
        CommandError::Client(ClientError::Build(_) | ClientError::InvalidBaseUrl) => {
            ("CONFIG_ERROR", "Check API client configuration".into())
        }
        CommandError::Serialize(_) => ("INTERNAL_ERROR", "Retry the command".into()),
    };
    print_failure(command, error.to_string(), code, fix);
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

fn run_config() {
    let path = config::path();
    match config::load_from(&path) {
        Ok(config) => {
            let env = config::current_env();
            let token_command = (!config.token_command.is_empty()).then_some("(configured)");
            print_json(envelope::success(
                "front config",
                ConfigResult {
                    path: path.display().to_string(),
                    token_command,
                    user: config::resolve_user(&config, &env),
                    token_source: config::token_source(&config, &env),
                    user_source: config::user_source(&config, &env),
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
