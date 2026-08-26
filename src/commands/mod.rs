use std::collections::BTreeMap;

mod doctor;
mod read_api;
pub use doctor::doctor_json;
pub use read_api::{ReadRequest, execute_read, whoami_json};

use chrono::{DateTime, NaiveDate, SecondsFormat, Utc};
use serde::Serialize;
use url::Url;

use crate::{
    client::{ClientError, FrontClient},
    envelope::{self, Action, ParamSpec},
    inbox_params,
    models::{
        ContactSummary, ConversationResponse, ConversationSummary, MessageResponse, MessageSummary,
    },
};

pub const DEFAULT_LIMIT: u32 = 25;
pub const MAX_TEXT_LENGTH: usize = 500;

#[derive(Debug, thiserror::Error)]
pub enum CommandError {
    #[error(transparent)]
    Client(#[from] ClientError),
    #[error("invalid date {value:?}, expected YYYY-MM-DD format")]
    InvalidDate { value: String },
    #[error("serialize output: {0}")]
    Serialize(#[from] serde_json::Error),
}

#[derive(Clone, Debug)]
pub struct InboxOptions {
    pub inbox_id: Option<String>,
    pub query: String,
    pub query_was_set: bool,
    pub from: Option<String>,
    pub assignee: Option<String>,
    pub before: Option<String>,
    pub after: Option<String>,
    pub limit: u32,
    pub limit_was_set: bool,
    pub page_token: Option<String>,
}

impl Default for InboxOptions {
    fn default() -> Self {
        Self {
            inbox_id: None,
            query: "is:open is:unassigned".into(),
            query_was_set: false,
            from: None,
            assignee: None,
            before: None,
            after: None,
            limit: DEFAULT_LIMIT,
            limit_was_set: false,
            page_token: None,
        }
    }
}

#[derive(Serialize)]
struct InboxSummary {
    id: String,
    name: String,
}

#[derive(Serialize)]
struct InboxesResult {
    #[serde(skip_serializing_if = "String::is_empty")]
    user: String,
    count: usize,
    inboxes: Vec<InboxSummary>,
}

#[derive(Serialize)]
struct InboxResult {
    total: usize,
    showing: usize,
    query: String,
    conversations: Vec<ConversationSummary>,
    #[serde(skip_serializing_if = "String::is_empty")]
    next_page_token: String,
}

#[derive(Serialize)]
struct ReadResult {
    conversation: ConversationSummary,
    messages: Vec<MessageSummary>,
    #[serde(skip_serializing_if = "is_false")]
    truncated: bool,
}

fn is_false(value: &bool) -> bool {
    !*value
}

pub async fn inboxes_json(client: &FrontClient, user: &str) -> Result<String, CommandError> {
    let response = if user.is_empty() {
        client.list_inboxes().await?
    } else {
        client
            .list_teammate_inboxes(&format!("alt:email:{user}"))
            .await?
    };

    let inboxes: Vec<_> = response
        .results
        .into_iter()
        .map(|inbox| InboxSummary {
            id: inbox.id.unwrap_or_default(),
            name: inbox.name.unwrap_or_default(),
        })
        .collect();
    let mut actions = vec![];
    if let Some(first) = inboxes.first().filter(|inbox| !inbox.id.is_empty()) {
        actions.push(Action {
            command: "front inbox <inbox-id>".into(),
            description: "Search conversations in this inbox".into(),
            params: BTreeMap::from([(
                "inbox-id".into(),
                ParamSpec {
                    value: Some(first.id.clone()),
                    ..ParamSpec::new("Inbox ID")
                },
            )]),
        });
    }
    actions.push(inbox_action());

    Ok(envelope::success(
        "front inboxes",
        InboxesResult {
            user: user.into(),
            count: inboxes.len(),
            inboxes,
        },
        actions,
    )?)
}

pub async fn inbox_json(
    client: &FrontClient,
    options: &InboxOptions,
) -> Result<String, CommandError> {
    let query = build_search_query(options)?;
    let response = client
        .search_conversations(&query, options.limit, options.page_token.as_deref())
        .await?;
    let next_page_token = response
        .pagination
        .as_ref()
        .and_then(|page| page.next.as_deref())
        .and_then(next_page_token)
        .unwrap_or_default();
    let conversations: Vec<_> = response.results.into_iter().map(map_conversation).collect();

    let mut actions = vec![];
    if !next_page_token.is_empty() {
        let mut params = BTreeMap::from([(
            "--page-token".into(),
            ParamSpec {
                value: Some(next_page_token.clone()),
                ..ParamSpec::new("Next page token")
            },
        )]);
        add_current_flags(options, &mut params);
        actions.push(Action {
            command: options
                .inbox_id
                .as_ref()
                .map(|id| format!("front inbox {id}"))
                .unwrap_or_else(|| "front inbox".into()),
            description: "Next page of results".into(),
            params,
        });
    }
    if let Some(first) = conversations.first() {
        actions.push(Action {
            command: "front read <conversation-id>".into(),
            description: "Read conversation and messages".into(),
            params: BTreeMap::from([(
                "conversation-id".into(),
                ParamSpec {
                    value: Some(first.id.clone()),
                    ..ParamSpec::new("Conversation ID")
                },
            )]),
        });
    }
    actions.push(Action {
        command: "front inboxes".into(),
        description: "List all inboxes".into(),
        params: BTreeMap::new(),
    });

    Ok(envelope::success(
        "front inbox",
        InboxResult {
            total: response.total,
            showing: conversations.len(),
            query,
            conversations,
            next_page_token,
        },
        actions,
    )?)
}

pub async fn read_json(
    client: &FrontClient,
    conversation_id: &str,
) -> Result<String, CommandError> {
    let (conversation, messages) = tokio::try_join!(
        client.get_conversation(conversation_id),
        client.list_conversation_messages(conversation_id, DEFAULT_LIMIT),
    )?;
    let truncated = messages
        .pagination
        .as_ref()
        .is_some_and(|pagination| pagination.next.is_some());
    let messages = messages.results.into_iter().map(map_message).collect();
    let actions = vec![
        Action {
            command: "front read <conversation-id>".into(),
            description: "Refresh this conversation".into(),
            params: BTreeMap::from([(
                "conversation-id".into(),
                ParamSpec {
                    value: Some(conversation_id.into()),
                    ..ParamSpec::new("Conversation ID")
                },
            )]),
        },
        inbox_action(),
        Action {
            command: "front inboxes".into(),
            description: "List all inboxes".into(),
            params: BTreeMap::new(),
        },
    ];

    Ok(envelope::success(
        "front read",
        ReadResult {
            conversation: map_conversation(conversation),
            messages,
            truncated,
        },
        actions,
    )?)
}

pub fn build_search_query(options: &InboxOptions) -> Result<String, CommandError> {
    let mut query = options.query.clone();
    if options.assignee.is_some() && !options.query_was_set {
        query = query.replacen("is:unassigned", "is:assigned", 1);
    }
    let mut parts = vec![query];
    push_filter(&mut parts, "inbox", options.inbox_id.as_deref());
    push_filter(&mut parts, "from", options.from.as_deref());
    if let Some(assignee) = &options.assignee {
        parts.push(format!("assignee:alt:email:{assignee}"));
    }
    if let Some(before) = &options.before {
        parts.push(format!("before:{}", date_to_unix(before)?));
    }
    if let Some(after) = &options.after {
        parts.push(format!("after:{}", date_to_unix(after)?));
    }
    Ok(parts.join(" ").trim().to_owned())
}

fn push_filter(parts: &mut Vec<String>, name: &str, value: Option<&str>) {
    if let Some(value) = value.filter(|value| !value.is_empty()) {
        parts.push(format!("{name}:{value}"));
    }
}

fn date_to_unix(value: &str) -> Result<i64, CommandError> {
    NaiveDate::parse_from_str(value, "%Y-%m-%d")
        .map(|date| {
            date.and_hms_opt(0, 0, 0)
                .expect("midnight is valid")
                .and_utc()
                .timestamp()
        })
        .map_err(|_| CommandError::InvalidDate {
            value: value.into(),
        })
}

fn next_page_token(next: &str) -> Option<String> {
    Url::parse(next)
        .ok()?
        .query_pairs()
        .find(|(name, _)| name == "page_token")
        .map(|(_, value)| value.into_owned())
}

fn map_conversation(conversation: ConversationResponse) -> ConversationSummary {
    let assignee = conversation
        .assignee
        .filter(|assignee| !assignee.email.is_empty())
        .map(|assignee| ContactSummary {
            name: full_name(&assignee.first_name, &assignee.last_name),
            email: assignee.email,
            ..ContactSummary::default()
        });
    let recipient = conversation.recipient.unwrap_or_default();
    ConversationSummary {
        id: conversation.id,
        subject: conversation.subject,
        status: conversation.status,
        from: ContactSummary {
            handle: recipient.handle,
            name: recipient.name.unwrap_or_default(),
            ..ContactSummary::default()
        },
        date: timestamp(conversation.updated_at.or(conversation.created_at)),
        assignee,
        waiting_since: timestamp(conversation.waiting_since),
        tags: conversation
            .tags
            .unwrap_or_default()
            .into_iter()
            .map(|tag| tag.name)
            .collect(),
    }
}

fn map_message(message: MessageResponse) -> MessageSummary {
    let from = message
        .author
        .map(|author| ContactSummary {
            name: full_name(&author.first_name, &author.last_name),
            email: author.email,
            ..ContactSummary::default()
        })
        .or_else(|| {
            message
                .recipients
                .into_iter()
                .find(|recipient| recipient.role == "from")
                .map(|recipient| ContactSummary {
                    handle: recipient.handle,
                    name: recipient.name.unwrap_or_default(),
                    ..ContactSummary::default()
                })
        });
    let body = message
        .text
        .filter(|text| !text.is_empty())
        .or_else(|| message.body.filter(|body| !body.is_empty()))
        .unwrap_or_default();
    MessageSummary {
        id: message.id.unwrap_or_default(),
        from,
        date: timestamp(message.created_at),
        is_inbound: message.is_inbound,
        text: truncate_utf8(&body, MAX_TEXT_LENGTH),
    }
}

fn truncate_utf8(value: &str, max_bytes: usize) -> String {
    if value.len() <= max_bytes {
        return value.into();
    }
    let mut end = max_bytes;
    while !value.is_char_boundary(end) {
        end -= 1;
    }
    format!("{}... [truncated]", &value[..end])
}

fn timestamp(value: Option<f64>) -> String {
    value
        .and_then(|value| DateTime::<Utc>::from_timestamp(value as i64, 0))
        .map(|value| value.to_rfc3339_opts(SecondsFormat::Secs, true))
        .unwrap_or_default()
}

fn full_name(first_name: &str, last_name: &str) -> String {
    match (first_name.is_empty(), last_name.is_empty()) {
        (true, _) => last_name.into(),
        (_, true) => first_name.into(),
        (false, false) => format!("{first_name} {last_name}"),
    }
}

fn inbox_action() -> Action {
    Action {
        command: "front inbox [inbox-id]".into(),
        description: "Search conversations".into(),
        params: inbox_params(),
    }
}

fn add_current_flags(options: &InboxOptions, params: &mut BTreeMap<String, ParamSpec>) {
    let limit = options.limit_was_set.then(|| options.limit.to_string());
    let values = [
        (
            "--query",
            options.query_was_set.then_some(options.query.as_str()),
        ),
        ("--from", options.from.as_deref()),
        ("--assignee", options.assignee.as_deref()),
        ("--before", options.before.as_deref()),
        ("--after", options.after.as_deref()),
        ("--limit", limit.as_deref()),
    ];
    for (name, value) in values {
        if let Some(value) = value {
            params.entry(name.into()).or_insert_with(|| ParamSpec {
                value: Some(value.into()),
                ..ParamSpec::new(match name {
                    "--query" => "Search query",
                    "--from" => "Sender filter",
                    "--assignee" => "Assignee filter",
                    "--before" => "Before date",
                    "--after" => "After date",
                    _ => "Maximum results",
                })
            });
        }
    }
}
