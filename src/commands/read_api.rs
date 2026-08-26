use std::collections::BTreeMap;

use serde::Serialize;
use serde_json::Value;
use url::Url;

use crate::{
    client::FrontClient,
    envelope::{self, Action, ParamSpec},
};

use super::CommandError;

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct OutputOptions {
    pub count_only: bool,
    pub keys_only: bool,
    pub fields: Vec<String>,
    pub max_items: Option<usize>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ReadRequest {
    pub command: String,
    pub segments: Vec<String>,
    pub query: Vec<(String, String)>,
    pub pagination_command: Option<String>,
    pub output: OutputOptions,
}

#[derive(Serialize)]
struct GenericResult {
    #[serde(skip_serializing_if = "Option::is_none")]
    data: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    count: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    returned: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    projection: Option<Projection>,
    #[serde(skip_serializing_if = "Option::is_none")]
    truncated: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    next_page_token: Option<String>,
}

#[derive(Serialize)]
#[serde(tag = "mode", rename_all = "kebab-case")]
enum Projection {
    CountOnly,
    KeysOnly,
    Fields { fields: Vec<String> },
}

struct TransformedOutput {
    data: Option<Value>,
    count: Option<usize>,
    returned: Option<usize>,
    projection: Option<Projection>,
    truncated: Option<bool>,
}

pub async fn execute_read(
    client: &FrontClient,
    request: ReadRequest,
) -> Result<String, CommandError> {
    let data = client.get_value(&request.segments, &request.query).await?;
    let next_page_token = data
        .pointer("/_pagination/next")
        .and_then(Value::as_str)
        .and_then(page_token);
    let transformed = transform_output(data, &request.output);
    let next_actions = match (&request.pagination_command, &next_page_token) {
        (Some(command), Some(token)) => vec![Action {
            command: command.clone(),
            description: "Next page of results".into(),
            params: pagination_params(&request.query, &request.output, token),
        }],
        _ => vec![],
    };

    Ok(envelope::success(
        request.command,
        GenericResult {
            data: transformed.data,
            count: transformed.count,
            returned: transformed.returned,
            projection: transformed.projection,
            truncated: transformed.truncated,
            next_page_token,
        },
        next_actions,
    )?)
}

fn transform_output(mut data: Value, options: &OutputOptions) -> TransformedOutput {
    let legacy_count = result_items(&data).map(<[_]>::len);
    if !options.is_active() {
        return TransformedOutput {
            data: Some(data),
            count: legacy_count,
            returned: None,
            projection: None,
            truncated: None,
        };
    }

    let collection_count = legacy_count.or_else(|| data.as_array().map(Vec::len));
    if options.count_only {
        return TransformedOutput {
            data: None,
            count: collection_count,
            returned: Some(0),
            projection: Some(Projection::CountOnly),
            truncated: None,
        };
    }

    let truncated = options
        .max_items
        .is_some_and(|max_items| truncate_items(&mut data, max_items));
    let returned = collection_count
        .map(|count| count.min(options.max_items.unwrap_or(count)))
        .unwrap_or(1);
    let projection = if options.keys_only {
        data = project_keys(data);
        Some(Projection::KeysOnly)
    } else if !options.fields.is_empty() {
        data = project_fields(data, &options.fields);
        Some(Projection::Fields {
            fields: options.fields.clone(),
        })
    } else {
        None
    };

    TransformedOutput {
        data: Some(data),
        count: collection_count,
        returned: Some(returned),
        projection,
        truncated: truncated.then_some(true),
    }
}

impl OutputOptions {
    fn is_active(&self) -> bool {
        self.count_only || self.keys_only || !self.fields.is_empty() || self.max_items.is_some()
    }
}

fn result_items(data: &Value) -> Option<&[Value]> {
    data.get("_results")
        .and_then(Value::as_array)
        .map(Vec::as_slice)
}

fn truncate_items(data: &mut Value, max_items: usize) -> bool {
    let items = match data {
        Value::Object(object) => object.get_mut("_results").and_then(Value::as_array_mut),
        Value::Array(items) => Some(items),
        _ => None,
    };
    let Some(items) = items else {
        return false;
    };
    if items.len() <= max_items {
        return false;
    }
    items.truncate(max_items);
    true
}

fn project_keys(data: Value) -> Value {
    match data {
        Value::Object(mut object) if object.get("_results").is_some_and(Value::is_array) => {
            let Value::Array(items) = object.remove("_results").expect("array checked above")
            else {
                unreachable!("array checked above")
            };
            serde_json::json!({
                "_results": items.into_iter().map(object_keys).collect::<Vec<_>>()
            })
        }
        Value::Object(object) => object_keys(Value::Object(object)),
        Value::Array(items) => Value::Array(items.into_iter().map(object_keys).collect()),
        _ => Value::Array(vec![]),
    }
}

fn object_keys(value: Value) -> Value {
    let Value::Object(object) = value else {
        return Value::Array(vec![]);
    };
    let mut keys: Vec<_> = object.into_iter().map(|(key, _)| key).collect();
    keys.sort();
    Value::Array(keys.into_iter().map(Value::String).collect())
}

fn project_fields(data: Value, fields: &[String]) -> Value {
    match data {
        Value::Object(mut object) if object.get("_results").is_some_and(Value::is_array) => {
            let Value::Array(items) = object.remove("_results").expect("array checked above")
            else {
                unreachable!("array checked above")
            };
            serde_json::json!({
                "_results": items
                    .into_iter()
                    .map(|item| object_fields(item, fields))
                    .collect::<Vec<_>>()
            })
        }
        Value::Object(object) => object_fields(Value::Object(object), fields),
        Value::Array(items) => Value::Array(
            items
                .into_iter()
                .map(|item| object_fields(item, fields))
                .collect(),
        ),
        _ => serde_json::json!({}),
    }
}

fn object_fields(value: Value, fields: &[String]) -> Value {
    let Value::Object(mut object) = value else {
        return serde_json::json!({});
    };
    let projected = fields
        .iter()
        .filter_map(|field| object.remove(field).map(|value| (field.clone(), value)))
        .collect();
    Value::Object(projected)
}

fn pagination_params(
    query: &[(String, String)],
    output: &OutputOptions,
    next_token: &str,
) -> BTreeMap<String, ParamSpec> {
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
    if output.count_only {
        params.insert(
            "--count-only".into(),
            ParamSpec::new("Return collection counts without response data"),
        );
    }
    if output.keys_only {
        params.insert(
            "--keys-only".into(),
            ParamSpec::new("Return sorted object keys without object values"),
        );
    }
    if !output.fields.is_empty() {
        params.insert(
            "--fields".into(),
            ParamSpec {
                value: Some(output.fields.join(",")),
                ..ParamSpec::new("Keep only these literal top-level fields")
            },
        );
    }
    if let Some(max_items) = output.max_items {
        params.insert(
            "--max-items".into(),
            ParamSpec {
                value: Some(max_items.to_string()),
                ..ParamSpec::new("Maximum decoded collection items returned locally")
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
            output: OutputOptions::default(),
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
