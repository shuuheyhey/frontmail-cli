use clap::{Args, Parser, Subcommand};
use clap_complete::Shell;

use crate::{
    commands::{ContinuationQueryParam, OutputOptions, PaginationContext, ReadRequest},
    envelope::ActionContext,
    resources::{Resource, ResourceError, parse_query_pairs, related_segments, validate_api_path},
};

#[derive(Parser)]
#[command(name = "front", disable_help_subcommand = true)]
pub struct Cli {
    #[arg(long, global = true)]
    pub version: bool,
    /// Select a named config profile
    #[arg(long, global = true, value_name = "NAME")]
    pub profile: Option<String>,
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
    #[command(flatten)]
    pub output: OutputArgs,
}

#[derive(Args)]
pub struct ListArgs {
    /// Resource name, singular or plural
    pub resource: String,
    #[command(flatten)]
    pub query: QueryArgs,
    #[command(flatten)]
    pub output: OutputArgs,
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
    #[command(flatten)]
    pub output: OutputArgs,
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
    #[command(flatten)]
    pub output: OutputArgs,
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

#[derive(Args, Default)]
pub struct OutputArgs {
    /// Return collection counts without response data
    #[arg(long, conflicts_with_all = ["keys_only", "fields", "max_items"])]
    pub count_only: bool,
    /// Return sorted object keys without object values
    #[arg(long, conflicts_with = "fields")]
    pub keys_only: bool,
    /// Keep only these literal top-level fields, separated by commas
    #[arg(long, value_delimiter = ',')]
    pub fields: Vec<String>,
    /// Maximum number of decoded collection items to return locally
    #[arg(long, value_parser = parse_positive_usize)]
    pub max_items: Option<usize>,
}

fn parse_positive_usize(value: &str) -> Result<usize, String> {
    match value.parse::<usize>() {
        Ok(value) if value > 0 => Ok(value),
        _ => Err("must be a positive integer".into()),
    }
}

impl From<&OutputArgs> for OutputOptions {
    fn from(args: &OutputArgs) -> Self {
        Self {
            count_only: args.count_only,
            keys_only: args.keys_only,
            fields: args.fields.clone(),
            max_items: args.max_items,
        }
    }
}

#[derive(Args)]
pub struct CompletionArgs {
    /// Shell whose completion script should be generated
    pub shell: Shell,
}

pub fn prepare_read_request(command: &Commands) -> Result<Option<ReadRequest>, ResourceError> {
    prepare_read_request_with_action_context(command, &ActionContext::default())
}

pub fn prepare_read_request_with_profile(
    command: &Commands,
    profile: Option<&str>,
) -> Result<Option<ReadRequest>, ResourceError> {
    prepare_read_request_with_action_context(
        command,
        &ActionContext::from_explicit_profile(profile),
    )
}

pub fn prepare_read_request_with_action_context(
    command: &Commands,
    action_context: &ActionContext,
) -> Result<Option<ReadRequest>, ResourceError> {
    let request = match command {
        Commands::Whoami => ReadRequest {
            command: "front whoami".into(),
            segments: vec!["me".into()],
            query: vec![],
            pagination: None,
            action_context: action_context.clone(),
            output: OutputOptions::default(),
        },
        Commands::Api(ApiArgs {
            action: ApiAction::Get(args),
        }) => {
            let command = format!("front api get {}", args.path);
            let query = build_query(&args.query)?;
            ReadRequest {
                command: command.clone(),
                segments: validate_api_path(&args.path)?,
                query: query.pairs,
                pagination: Some(PaginationContext::passthrough(command, query.continuation)),
                action_context: action_context.clone(),
                output: (&args.output).into(),
            }
        }
        Commands::List(args) => {
            let resource = Resource::parse(&args.resource)?;
            let segments = resource.collection_segments()?;
            validate_collection_query(resource, &args.query)?;
            let capabilities = resource.collection_query_capabilities();
            let command = format!("front list {}", resource.name());
            let query = build_query(&args.query)?;
            let pagination = if capabilities.page_token {
                PaginationContext::structured(command.clone(), query.continuation)
            } else {
                PaginationContext::passthrough(command.clone(), query.continuation)
            };
            ReadRequest {
                command,
                segments,
                query: query.pairs,
                pagination: Some(pagination),
                action_context: action_context.clone(),
                output: (&args.output).into(),
            }
        }
        Commands::Get(args) => {
            let resource = Resource::parse(&args.resource)?;
            ReadRequest {
                command: format!("front get {} {}", resource.name(), args.id),
                segments: resource.item_segments(&args.id)?,
                query: parse_query_pairs(&args.params)?,
                pagination: None,
                action_context: action_context.clone(),
                output: (&args.output).into(),
            }
        }
        Commands::Related(args) => {
            let resource = Resource::parse(&args.resource)?;
            let command = format!(
                "front related {} {} {}",
                resource.name(),
                args.id,
                args.relation
            );
            let query = build_query(&args.query)?;
            ReadRequest {
                command: command.clone(),
                segments: related_segments(resource, &args.id, &args.relation)?,
                query: query.pairs,
                pagination: Some(PaginationContext::structured(command, query.continuation)),
                action_context: action_context.clone(),
                output: (&args.output).into(),
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

struct PreparedQuery {
    pairs: Vec<(String, String)>,
    continuation: Vec<ContinuationQueryParam>,
}

fn build_query(args: &QueryArgs) -> Result<PreparedQuery, ResourceError> {
    let mut pairs = parse_query_pairs(&args.params)?;
    let mut continuation: Vec<ContinuationQueryParam> = pairs
        .iter()
        .cloned()
        .map(|(name, value)| ContinuationQueryParam::Passthrough(name, value))
        .collect();
    if let Some(limit) = args.limit {
        pairs.push(("limit".into(), limit.to_string()));
        continuation.push(ContinuationQueryParam::StructuredLimit(limit));
    }
    if let Some(token) = &args.page_token {
        pairs.push(("page_token".into(), token.clone()));
        continuation.push(ContinuationQueryParam::StructuredPageToken(token.clone()));
    }
    Ok(PreparedQuery {
        pairs,
        continuation,
    })
}
