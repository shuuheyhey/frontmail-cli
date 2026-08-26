use serde::Serialize;
use serde_json::Value;

use crate::{
    client::{ClientError, FrontClient},
    config::ConfigSource,
    envelope,
};

use super::CommandError;

#[derive(Clone, Copy, Debug, Eq, PartialEq, thiserror::Error)]
pub enum DoctorAuthenticationError {
    #[error("doctor authentication check failed (HTTP {status})")]
    Http { status: u16 },
    #[error("doctor authentication check failed (transport)")]
    Transport,
    #[error("doctor authentication check failed (invalid response)")]
    Decode,
    #[error("doctor authentication check failed (client configuration)")]
    ClientConfiguration,
}

impl From<ClientError> for DoctorAuthenticationError {
    fn from(error: ClientError) -> Self {
        match error {
            ClientError::Http { status, .. } => Self::Http { status },
            ClientError::Transport(_) => Self::Transport,
            ClientError::Decode(_) => Self::Decode,
            ClientError::Build(_) | ClientError::InvalidBaseUrl => Self::ClientConfiguration,
        }
    }
}

#[derive(Serialize)]
struct DoctorResult {
    token_source: ConfigSource,
    authentication: &'static str,
    configured_user_source: ConfigSource,
    configured_user_matches_token: UserMatch,
    checks: DoctorChecks,
}

#[derive(Serialize)]
struct DoctorChecks {
    tags_read: CheckStatus,
    inboxes_read: CheckStatus,
    teammates_read: CheckStatus,
}

#[derive(Serialize)]
#[serde(rename_all = "snake_case")]
enum CheckStatus {
    Ok,
    Forbidden,
    Error,
}

#[derive(Serialize)]
#[serde(untagged)]
enum UserMatch {
    Matches(bool),
    Status(UserMatchStatus),
}

#[derive(Serialize)]
#[serde(rename_all = "snake_case")]
enum UserMatchStatus {
    Unavailable,
    NotConfigured,
}

pub async fn doctor_json(
    client: &FrontClient,
    token_source: ConfigSource,
    configured_user_source: ConfigSource,
    effective_user: &str,
) -> Result<String, CommandError> {
    let authenticated_id = response_id(
        client
            .get_value(&["me".into()], &[])
            .await
            .map_err(DoctorAuthenticationError::from)?,
    );

    let configured_user_matches_token = if effective_user.is_empty() {
        UserMatch::Status(UserMatchStatus::NotConfigured)
    } else {
        let configured_id = client
            .get_value(
                &["teammates".into(), format!("alt:email:{effective_user}")],
                &[],
            )
            .await
            .ok()
            .and_then(response_id);
        match (authenticated_id.as_deref(), configured_id.as_deref()) {
            (Some(authenticated), Some(configured)) => {
                UserMatch::Matches(authenticated == configured)
            }
            _ => UserMatch::Status(UserMatchStatus::Unavailable),
        }
    };

    let tags_read = optional_check(
        client
            .get_value(&["tags".into()], &[("limit".into(), "1".into())])
            .await,
    );
    let inboxes_read = optional_check(client.get_value(&["inboxes".into()], &[]).await);
    let teammates_read = optional_check(client.get_value(&["teammates".into()], &[]).await);

    Ok(envelope::success(
        "front doctor",
        DoctorResult {
            token_source,
            authentication: "ok",
            configured_user_source,
            configured_user_matches_token,
            checks: DoctorChecks {
                tags_read,
                inboxes_read,
                teammates_read,
            },
        },
        vec![],
    )?)
}

fn response_id(response: Value) -> Option<String> {
    response
        .get("id")
        .and_then(Value::as_str)
        .map(str::to_owned)
}

fn optional_check(result: Result<Value, ClientError>) -> CheckStatus {
    match result {
        Ok(_) => CheckStatus::Ok,
        Err(ClientError::Http { status: 403, .. }) => CheckStatus::Forbidden,
        Err(_) => CheckStatus::Error,
    }
}
