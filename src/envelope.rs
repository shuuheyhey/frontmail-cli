use std::collections::BTreeMap;

use serde::Serialize;

#[derive(Clone, Debug, Serialize)]
pub struct ParamSpec {
    pub description: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub value: Option<String>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub values: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub default: Option<String>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub r#enum: Vec<String>,
    #[serde(skip_serializing_if = "std::ops::Not::not")]
    pub required: bool,
}

impl ParamSpec {
    pub fn new(description: impl Into<String>) -> Self {
        Self {
            description: description.into(),
            value: None,
            values: vec![],
            default: None,
            r#enum: vec![],
            required: false,
        }
    }
}

#[derive(Clone, Debug, Serialize)]
pub struct Action {
    pub command: String,
    pub description: String,
    #[serde(skip_serializing_if = "BTreeMap::is_empty")]
    pub params: BTreeMap<String, ParamSpec>,
}

#[derive(Debug, Serialize)]
pub struct Success<T> {
    pub ok: bool,
    pub command: String,
    pub result: T,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub next_actions: Vec<Action>,
}

#[derive(Debug, Serialize)]
pub struct ErrorDetail {
    pub message: String,
    pub code: String,
}

#[derive(Debug, Serialize)]
pub struct Failure {
    pub ok: bool,
    pub command: String,
    pub error: ErrorDetail,
    #[serde(skip_serializing_if = "String::is_empty")]
    pub fix: String,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub next_actions: Vec<Action>,
}

pub fn success<T: Serialize>(
    command: impl Into<String>,
    result: T,
    next_actions: Vec<Action>,
) -> serde_json::Result<String> {
    pretty(&Success {
        ok: true,
        command: command.into(),
        result,
        next_actions,
    })
}

pub fn failure(
    command: impl Into<String>,
    message: impl Into<String>,
    code: impl Into<String>,
    fix: impl Into<String>,
    next_actions: Vec<Action>,
) -> serde_json::Result<String> {
    pretty(&Failure {
        ok: false,
        command: command.into(),
        error: ErrorDetail {
            message: message.into(),
            code: code.into(),
        },
        fix: fix.into(),
        next_actions,
    })
}

fn pretty<T: Serialize>(value: &T) -> serde_json::Result<String> {
    serde_json::to_string_pretty(value).map(|json| format!("{json}\n"))
}
