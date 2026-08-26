use clap::{Args, Parser, Subcommand};
use clap_complete::Shell;

use crate::{
    commands::ReadRequest,
    resources::{Resource, ResourceError, parse_query_pairs, related_segments, validate_api_path},
};

#[derive(Parser)]
#[command(name = "front", disable_help_subcommand = true)]
pub struct Cli {
    #[arg(long, global = true)]
    pub version: bool,
    #[command(subcommand)]
    pub command: Option<Commands>,
}

#[derive(Subcommand)]
pub enum Commands {
    /// Show CLI configuration
    Config,
    /// Run redacted authentication and read-scope diagnostics
    Doctor,
    /// List all inboxes
    Inboxes,
    /// Search conversations
    Inbox(InboxArgs),
    /// Read a conversation and its messages
    Read(ReadArgs),
    /// Show the authenticated Front user
    Whoami,
    /// Call a safe Front API endpoint
    Api(ApiArgs),
    /// List a supported Front resource
    List(ListArgs),
    /// Get a supported Front resource by ID
    Get(GetArgs),
    /// List an allowlisted relation for a Front resource
    Related(RelatedArgs),
    /// Generate shell completion code
    Completion(CompletionArgs),
}

#[derive(Args)]
pub struct InboxArgs {
    /// inbox-id (optional)
    pub inbox_id: Option<String>,
    /// Search query (Front search syntax)
    #[arg(long, default_value = "is:open is:unassigned")]
    pub query: String,
    /// Filter by sender handle (shortcut for from:<handle> in query)
    #[arg(long)]
    pub from: Option<String>,
    /// Filter by assignee email address
    #[arg(long)]
    pub assignee: Option<String>,
    /// Before date, YYYY-MM-DD (shortcut for before:<ts> in query)
    #[arg(long)]
    pub before: Option<String>,
    /// After date, YYYY-MM-DD (shortcut for after:<ts> in query)
    #[arg(long)]
    pub after: Option<String>,
    /// Maximum number of results to return
    #[arg(
        long,
        default_value_t = 25,
        value_parser = clap::value_parser!(u32).range(1..=100)
    )]
    pub limit: u32,
    #[arg(long, hide = true)]
    pub page_token: Option<String>,
}

#[derive(Args)]
pub struct ReadArgs {
    /// conversation-id (required)
    pub conversation_id: String,
}

#[derive(Args)]
pub struct ApiArgs {
    #[command(subcommand)]
    pub action: ApiAction,
}

#[derive(Subcommand)]
pub enum ApiAction {
    /// GET a relative Front Core API path
    Get(ApiGetArgs),
}

#[derive(Args)]
pub struct ApiGetArgs {
    /// Relative API path beginning with one slash
    pub path: String,
    #[command(flatten)]
    pub query: QueryArgs,
}

#[derive(Args)]
pub struct ListArgs {
    /// Resource name, singular or plural
    pub resource: String,
    #[command(flatten)]
    pub query: QueryArgs,
}

#[derive(Args)]
pub struct GetArgs {
    /// Resource name, singular or plural
    pub resource: String,
    /// Front resource ID or alternate ID
    pub id: String,
    /// Additional query parameter as key=value
    #[arg(long = "param")]
    pub params: Vec<String>,
}

#[derive(Args)]
pub struct RelatedArgs {
    /// Parent resource name, singular or plural
    pub resource: String,
    /// Front parent resource ID or alternate ID
    pub id: String,
    /// Allowlisted relation name
    pub relation: String,
    #[command(flatten)]
    pub query: QueryArgs,
}

#[derive(Args)]
pub struct QueryArgs {
    /// Maximum results requested from Front
    #[arg(long, value_parser = clap::value_parser!(u32).range(1..=100))]
    pub limit: Option<u32>,
    /// Front pagination token
    #[arg(long)]
    pub page_token: Option<String>,
    /// Additional query parameter as key=value; repeat as needed
    #[arg(long = "param")]
    pub params: Vec<String>,
}

#[derive(Args)]
pub struct CompletionArgs {
    /// Shell whose completion script should be generated
    pub shell: Shell,
}

pub fn prepare_read_request(command: &Commands) -> Result<Option<ReadRequest>, ResourceError> {
    let request = match command {
        Commands::Whoami => ReadRequest {
            command: "front whoami".into(),
            segments: vec!["me".into()],
            query: vec![],
            pagination_command: None,
        },
        Commands::Api(ApiArgs {
            action: ApiAction::Get(args),
        }) => ReadRequest {
            command: format!("front api get {}", args.path),
            segments: validate_api_path(&args.path)?,
            query: build_query(&args.query)?,
            pagination_command: Some(format!("front api get {}", args.path)),
        },
        Commands::List(args) => {
            let resource = Resource::parse(&args.resource)?;
            let segments = resource.collection_segments()?;
            validate_collection_query(resource, &args.query)?;
            ReadRequest {
                command: format!("front list {}", resource.name()),
                segments,
                query: build_query(&args.query)?,
                pagination_command: Some(format!("front list {}", resource.name())),
            }
        }
        Commands::Get(args) => {
            let resource = Resource::parse(&args.resource)?;
            ReadRequest {
                command: format!("front get {} {}", resource.name(), args.id),
                segments: resource.item_segments(&args.id)?,
                query: parse_query_pairs(&args.params)?,
                pagination_command: None,
            }
        }
        Commands::Related(args) => {
            let resource = Resource::parse(&args.resource)?;
            ReadRequest {
                command: format!(
                    "front related {} {} {}",
                    resource.name(),
                    args.id,
                    args.relation
                ),
                segments: related_segments(resource, &args.id, &args.relation)?,
                query: build_query(&args.query)?,
                pagination_command: Some(format!(
                    "front related {} {} {}",
                    resource.name(),
                    args.id,
                    args.relation
                )),
            }
        }
        Commands::Config
        | Commands::Doctor
        | Commands::Inboxes
        | Commands::Inbox(_)
        | Commands::Read(_)
        | Commands::Completion(_) => return Ok(None),
    };
    Ok(Some(request))
}

fn validate_collection_query(resource: Resource, args: &QueryArgs) -> Result<(), ResourceError> {
    let capabilities = resource.collection_query_capabilities();
    if args.limit.is_some() && !capabilities.limit {
        return Err(ResourceError::UnsupportedCollectionQueryParameter {
            resource: resource.name(),
            parameter: "--limit",
        });
    }
    if args.page_token.is_some() && !capabilities.page_token {
        return Err(ResourceError::UnsupportedCollectionQueryParameter {
            resource: resource.name(),
            parameter: "--page-token",
        });
    }
    Ok(())
}

fn build_query(args: &QueryArgs) -> Result<Vec<(String, String)>, ResourceError> {
    let mut query = parse_query_pairs(&args.params)?;
    if let Some(limit) = args.limit {
        query.push(("limit".into(), limit.to_string()));
    }
    if let Some(token) = &args.page_token {
        query.push(("page_token".into(), token.clone()));
    }
    Ok(query)
}
