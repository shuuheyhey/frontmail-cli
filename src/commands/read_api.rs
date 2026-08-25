use std::collections::BTreeMap;

use serde::Serialize;
use serde_json::Value;
use url::Url;

use crate::{
    client::FrontClient,
    envelope::{self, Action, ParamSpec},
};

use super::CommandError;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ReadRequest {
    pub command: String,
    pub segments: Vec<String>,
    pub query: Vec<(String, String)>,
    pub pagination_command: Option<String>,
}

#[derive(Serialize)]
struct GenericResult {
    data: Value,
    #[serde(skip_serializing_if = "Option::is_none")]
    count: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    next_page_token: Option<String>,
}

pub async fn execute_read(
    client: &FrontClient,
    request: ReadRequest,
) -> Result<String, CommandError> {
    let data = client.get_value(&request.segments, &request.query).await?;
    let count = data.get("_results").and_then(Value::as_array).map(Vec::len);
    let next_page_token = data
        .pointer("/_pagination/next")
        .and_then(Value::as_str)
        .and_then(page_token);
    let next_actions = match (&request.pagination_command, &next_page_token) {
        (Some(command), Some(token)) => vec![Action {
            command: command.clone(),
            description: "Next page of results".into(),
            params: pagination_params(&request.query, token),
        }],
        _ => vec![],
    };

    Ok(envelope::success(
        request.command,
        GenericResult {
            data,
            count,
            next_page_token,
        },
        next_actions,
    )?)
}

fn pagination_params(query: &[(String, String)], next_token: &str) -> BTreeMap<String, ParamSpec> {
    let last_limit = query.iter().rposition(|(name, _)| name == "limit");
    let mut params = BTreeMap::new();
    let repeated: Vec<_> = query
        .iter()
        .enumerate()
        .filter(|(index, (name, _))| name != "page_token" && Some(*index) != last_limit)
        .map(|(_, (name, value))| format!("{name}={value}"))
        .collect();

    if let Some(index) = last_limit {
        params.insert(
            "--limit".into(),
            ParamSpec {
                value: Some(query[index].1.clone()),
                ..ParamSpec::new("Maximum results requested from Front")
            },
        );
    }
    if !repeated.is_empty() {
        params.insert(
            "--param".into(),
            ParamSpec {
                values: repeated,
                ..ParamSpec::new("Additional query parameters; repeat this flag for each value")
            },
        );
    }
    params.insert(
        "--page-token".into(),
        ParamSpec {
            value: Some(next_token.into()),
            ..ParamSpec::new("Next page token")
        },
    );
    params
}

pub async fn whoami_json(client: &FrontClient) -> Result<String, CommandError> {
    execute_read(
        client,
        ReadRequest {
            command: "front whoami".into(),
            segments: vec!["me".into()],
            query: vec![],
            pagination_command: None,
        },
    )
    .await
}

fn page_token(next: &str) -> Option<String> {
    Url::parse(next)
        .ok()?
        .query_pairs()
        .find(|(name, _)| name == "page_token")
        .map(|(_, value)| value.into_owned())
}
