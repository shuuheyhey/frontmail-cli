use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Default, Deserialize)]
pub struct Page<T> {
    #[serde(rename = "_results", default)]
    pub results: Vec<T>,
    #[serde(rename = "_pagination", default)]
    pub pagination: Option<Pagination>,
    #[serde(rename = "_total", default)]
    pub total: usize,
}

#[derive(Clone, Debug, Default, Deserialize)]
pub struct Pagination {
    #[serde(default)]
    pub next: Option<String>,
}

#[derive(Clone, Debug, Default, Deserialize)]
pub struct InboxResponse {
    #[serde(default)]
    pub id: Option<String>,
    #[serde(default)]
    pub name: Option<String>,
}

#[derive(Clone, Debug, Default, Deserialize)]
pub struct ConversationResponse {
    #[serde(default)]
    pub id: String,
    #[serde(default)]
    pub subject: String,
    #[serde(default)]
    pub status: String,
    #[serde(default)]
    pub recipient: Option<RecipientResponse>,
    #[serde(default)]
    pub assignee: Option<TeammateResponse>,
    #[serde(default)]
    pub created_at: Option<f64>,
    #[serde(default)]
    pub updated_at: Option<f64>,
    #[serde(default)]
    pub waiting_since: Option<f64>,
    #[serde(default)]
    pub tags: Option<Vec<TagResponse>>,
}

#[derive(Clone, Debug, Default, Deserialize)]
pub struct RecipientResponse {
    #[serde(default)]
    pub handle: String,
    #[serde(default)]
    pub name: Option<String>,
    #[serde(default)]
    pub role: String,
}

#[derive(Clone, Debug, Default, Deserialize)]
pub struct TeammateResponse {
    #[serde(default)]
    pub first_name: String,
    #[serde(default)]
    pub last_name: String,
    #[serde(default)]
    pub email: String,
}

#[derive(Clone, Debug, Default, Deserialize)]
pub struct TagResponse {
    #[serde(default)]
    pub name: String,
}

#[derive(Clone, Debug, Default, Deserialize)]
pub struct MessageResponse {
    #[serde(default)]
    pub id: Option<String>,
    #[serde(default)]
    pub author: Option<TeammateResponse>,
    #[serde(default)]
    pub recipients: Vec<RecipientResponse>,
    #[serde(default)]
    pub created_at: Option<f64>,
    #[serde(default)]
    pub is_inbound: Option<bool>,
    #[serde(default)]
    pub text: Option<String>,
    #[serde(default)]
    pub body: Option<String>,
}

#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize)]
pub struct ContactSummary {
    #[serde(skip_serializing_if = "String::is_empty")]
    pub handle: String,
    #[serde(skip_serializing_if = "String::is_empty")]
    pub name: String,
    #[serde(skip_serializing_if = "String::is_empty")]
    pub email: String,
}

#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize)]
pub struct ConversationSummary {
    pub id: String,
    pub subject: String,
    pub status: String,
    pub from: ContactSummary,
    #[serde(skip_serializing_if = "String::is_empty")]
    pub date: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub assignee: Option<ContactSummary>,
    #[serde(skip_serializing_if = "String::is_empty")]
    pub waiting_since: String,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub tags: Vec<String>,
}

#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize)]
pub struct MessageSummary {
    pub id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub from: Option<ContactSummary>,
    #[serde(skip_serializing_if = "String::is_empty")]
    pub date: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub is_inbound: Option<bool>,
    pub text: String,
}
