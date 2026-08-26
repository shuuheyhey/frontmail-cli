#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Resource {
    Account,
    Channel,
    Comment,
    Contact,
    Conversation,
    Event,
    Inbox,
    KnowledgeBase,
    KnowledgeBaseArticle,
    KnowledgeBaseCategory,
    Link,
    Message,
    MessageTemplate,
    MessageTemplateFolder,
    Rule,
    Shift,
    Signature,
    Tag,
    Teammate,
    TeammateGroup,
    Team,
    TimeOff,
    View,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct CollectionQueryCapabilities {
    pub limit: bool,
    pub page_token: bool,
}

#[derive(Debug, thiserror::Error)]
pub enum ResourceError {
    #[error("unknown resource {0:?}")]
    UnknownResource(String),
    #[error("{0} cannot be listed as a top-level collection")]
    UnsupportedCollection(&'static str),
    #[error("invalid {resource} ID {id:?}")]
    InvalidId { resource: &'static str, id: String },
    #[error("relation {relation:?} is not available for {resource}")]
    UnsupportedRelation {
        resource: &'static str,
        relation: String,
    },
    #[error("invalid API path {path:?}: {reason}")]
    InvalidPath { path: String, reason: &'static str },
    #[error("invalid query parameter {0:?}, expected key=value")]
    InvalidQuery(String),
    #[error("{resource} list does not support {parameter}")]
    UnsupportedCollectionQueryParameter {
        resource: &'static str,
        parameter: &'static str,
    },
}

impl Resource {
    pub fn parse(value: &str) -> Result<Self, ResourceError> {
        let normalized = value.trim().to_ascii_lowercase().replace('_', "-");
        match normalized.as_str() {
            "account" | "accounts" => Ok(Self::Account),
            "channel" | "channels" => Ok(Self::Channel),
            "comment" | "comments" => Ok(Self::Comment),
            "contact" | "contacts" => Ok(Self::Contact),
            "conversation" | "conversations" | "conv" => Ok(Self::Conversation),
            "event" | "events" => Ok(Self::Event),
            "inbox" | "inboxes" => Ok(Self::Inbox),
            "knowledge-base" | "knowledge-bases" => Ok(Self::KnowledgeBase),
            "knowledge-base-article" | "knowledge-base-articles" => Ok(Self::KnowledgeBaseArticle),
            "knowledge-base-category" | "knowledge-base-categories" => {
                Ok(Self::KnowledgeBaseCategory)
            }
            "link" | "links" => Ok(Self::Link),
            "message" | "messages" | "msg" => Ok(Self::Message),
            "message-template" | "message-templates" | "template" | "templates" => {
                Ok(Self::MessageTemplate)
            }
            "message-template-folder"
            | "message-template-folders"
            | "template-folder"
            | "template-folders" => Ok(Self::MessageTemplateFolder),
            "rule" | "rules" => Ok(Self::Rule),
            "shift" | "shifts" => Ok(Self::Shift),
            "signature" | "signatures" => Ok(Self::Signature),
            "tag" | "tags" => Ok(Self::Tag),
            "teammate" | "teammates" => Ok(Self::Teammate),
            "teammate-group" | "teammate-groups" => Ok(Self::TeammateGroup),
            "team" | "teams" => Ok(Self::Team),
            "time-off" | "time-offs" => Ok(Self::TimeOff),
            "view" | "views" => Ok(Self::View),
            _ => Err(ResourceError::UnknownResource(value.into())),
        }
    }

    pub fn collection_segments(self) -> Result<Vec<String>, ResourceError> {
        if matches!(
            self,
            Self::Comment
                | Self::KnowledgeBaseArticle
                | Self::KnowledgeBaseCategory
                | Self::Message
                | Self::Signature
                | Self::TimeOff
        ) {
            return Err(ResourceError::UnsupportedCollection(self.name()));
        }
        Ok(vec![self.path_segment().into()])
    }

    pub fn collection_query_capabilities(self) -> CollectionQueryCapabilities {
        let supports_pagination = matches!(
            self,
            Self::Account
                | Self::Contact
                | Self::Conversation
                | Self::Event
                | Self::Link
                | Self::Tag
                | Self::View
        );
        CollectionQueryCapabilities {
            limit: supports_pagination,
            page_token: supports_pagination,
        }
    }

    pub fn item_segments(self, id: &str) -> Result<Vec<String>, ResourceError> {
        validate_id(self, id)?;
        Ok(vec![self.path_segment().into(), id.into()])
    }

    pub fn name(self) -> &'static str {
        match self {
            Self::Account => "account",
            Self::Channel => "channel",
            Self::Comment => "comment",
            Self::Contact => "contact",
            Self::Conversation => "conversation",
            Self::Event => "event",
            Self::Inbox => "inbox",
            Self::KnowledgeBase => "knowledge-base",
            Self::KnowledgeBaseArticle => "knowledge-base-article",
            Self::KnowledgeBaseCategory => "knowledge-base-category",
            Self::Link => "link",
            Self::Message => "message",
            Self::MessageTemplate => "message-template",
            Self::MessageTemplateFolder => "message-template-folder",
            Self::Rule => "rule",
            Self::Shift => "shift",
            Self::Signature => "signature",
            Self::Tag => "tag",
            Self::Teammate => "teammate",
            Self::TeammateGroup => "teammate-group",
            Self::Team => "team",
            Self::TimeOff => "time-off",
            Self::View => "view",
        }
    }

    fn path_segment(self) -> &'static str {
        match self {
            Self::Account => "accounts",
            Self::Channel => "channels",
            Self::Comment => "comments",
            Self::Contact => "contacts",
            Self::Conversation => "conversations",
            Self::Event => "events",
            Self::Inbox => "inboxes",
            Self::KnowledgeBase => "knowledge_bases",
            Self::KnowledgeBaseArticle => "knowledge_base_articles",
            Self::KnowledgeBaseCategory => "knowledge_base_categories",
            Self::Link => "links",
            Self::Message => "messages",
            Self::MessageTemplate => "message_templates",
            Self::MessageTemplateFolder => "message_template_folders",
            Self::Rule => "rules",
            Self::Shift => "shifts",
            Self::Signature => "signatures",
            Self::Tag => "tags",
            Self::Teammate => "teammates",
            Self::TeammateGroup => "teammate_groups",
            Self::Team => "teams",
            Self::TimeOff => "time_offs",
            Self::View => "views",
        }
    }
}

pub fn related_segments(
    resource: Resource,
    id: &str,
    relation: &str,
) -> Result<Vec<String>, ResourceError> {
    validate_id(resource, id)?;
    let relation = normalize_relation(relation);
    let allowed = match resource {
        Resource::Account => &["contacts"][..],
        Resource::Comment => &["mentions"],
        Resource::Contact => &["conversations", "notes"],
        Resource::Conversation => &[
            "comments",
            "drafts",
            "events",
            "followers",
            "inboxes",
            "messages",
        ],
        Resource::Inbox => &["channels", "conversations", "teammates"],
        Resource::KnowledgeBase => &["articles", "categories", "content"],
        Resource::KnowledgeBaseArticle => &["content"],
        Resource::KnowledgeBaseCategory => &["articles", "content"],
        Resource::Link => &["conversations"],
        Resource::Message => &["seen"],
        Resource::MessageTemplateFolder => &["message_template_folders", "message_templates"],
        Resource::Shift => &["teammates"],
        Resource::Tag => &["children", "conversations"],
        Resource::Teammate => &[
            "channels",
            "contact_groups",
            "contact_lists",
            "contacts",
            "conversations",
            "inboxes",
            "message_template_folders",
            "message_templates",
            "private_inboxes",
            "rules",
            "shifts",
            "signatures",
            "tags",
            "time_offs",
        ],
        Resource::TeammateGroup => &["inboxes", "teammates", "teams"],
        Resource::Team => &[
            "channels",
            "contact_groups",
            "contact_lists",
            "contacts",
            "inboxes",
            "message_template_folders",
            "message_templates",
            "rules",
            "shifts",
            "signatures",
            "tags",
            "time_offs",
            "views",
        ],
        Resource::Channel
        | Resource::Event
        | Resource::MessageTemplate
        | Resource::Rule
        | Resource::Signature
        | Resource::TimeOff
        | Resource::View => &[],
    };
    if !allowed.contains(&relation.as_str()) {
        return Err(ResourceError::UnsupportedRelation {
            resource: resource.name(),
            relation,
        });
    }
    Ok(vec![resource.path_segment().into(), id.into(), relation])
}

pub fn validate_api_path(path: &str) -> Result<Vec<String>, ResourceError> {
    let invalid = |reason| ResourceError::InvalidPath {
        path: path.into(),
        reason,
    };
    if !path.starts_with('/') || path.starts_with("//") {
        return Err(invalid("path must start with exactly one slash"));
    }
    if path.contains('?') || path.contains('#') || path.contains("://") {
        return Err(invalid(
            "query strings, fragments, and absolute URLs are not allowed",
        ));
    }
    let segments: Vec<_> = path[1..].split('/').collect();
    if segments.is_empty() || segments.iter().any(|segment| segment.is_empty()) {
        return Err(invalid("empty path segments are not allowed"));
    }
    if segments.iter().any(|segment| {
        matches!(*segment, "." | "..")
            || segment.eq_ignore_ascii_case("download")
            || segment.chars().any(char::is_control)
    }) {
        return Err(invalid(
            "traversal, download, and control segments are not allowed",
        ));
    }
    Ok(segments.into_iter().map(str::to_owned).collect())
}

pub fn parse_query_pairs(values: &[String]) -> Result<Vec<(String, String)>, ResourceError> {
    values
        .iter()
        .map(|value| {
            let (key, value_part) = value
                .split_once('=')
                .ok_or_else(|| ResourceError::InvalidQuery(value.clone()))?;
            if key.is_empty()
                || key.chars().any(char::is_control)
                || value_part.chars().any(char::is_control)
            {
                return Err(ResourceError::InvalidQuery(value.clone()));
            }
            Ok((key.into(), value_part.into()))
        })
        .collect()
}

fn validate_id(resource: Resource, id: &str) -> Result<(), ResourceError> {
    if id.is_empty()
        || matches!(id, "." | "..")
        || id.contains('/')
        || id.chars().any(char::is_control)
    {
        return Err(ResourceError::InvalidId {
            resource: resource.name(),
            id: id.into(),
        });
    }
    Ok(())
}

fn normalize_relation(value: &str) -> String {
    match value.trim().to_ascii_lowercase().replace('-', "_").as_str() {
        "convos" => "conversations".into(),
        "folders" => "message_template_folders".into(),
        "templates" => "message_templates".into(),
        other => other.into(),
    }
}
