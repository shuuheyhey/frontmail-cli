use std::time::Duration;

use reqwest::{Client, StatusCode};
use secrecy::{ExposeSecret, SecretString};
use serde::de::DeserializeOwned;
use url::Url;

use crate::models::{ConversationResponse, InboxResponse, MessageResponse, Page};
use crate::resources::validate_api_path;

pub const PRODUCTION_BASE_URL: &str = "https://api2.frontapp.com";

#[derive(Debug, thiserror::Error)]
pub enum ClientError {
    #[error("build HTTP client: {0}")]
    Build(#[source] reqwest::Error),
    #[error("invalid API base URL")]
    InvalidBaseUrl,
    #[error("HTTP transport error: {0}")]
    Transport(#[source] reqwest::Error),
    #[error("HTTP {status}: {message}")]
    Http { status: u16, message: String },
    #[error("decode Front API response: {0}")]
    Decode(#[source] serde_json::Error),
}

#[derive(Clone)]
pub struct FrontClient {
    base_url: Url,
    token: SecretString,
    user_agent: String,
    http: Client,
}

impl FrontClient {
    pub fn production(
        token: SecretString,
        user_agent: impl Into<String>,
    ) -> Result<Self, ClientError> {
        Self::new(
            PRODUCTION_BASE_URL
                .parse()
                .map_err(|_| ClientError::InvalidBaseUrl)?,
            token,
            user_agent,
        )
    }

    pub fn new(
        base_url: Url,
        token: SecretString,
        user_agent: impl Into<String>,
    ) -> Result<Self, ClientError> {
        let http = Client::builder()
            .timeout(Duration::from_secs(30))
            .redirect(reqwest::redirect::Policy::none())
            .build()
            .map_err(ClientError::Build)?;
        Ok(Self {
            base_url,
            token,
            user_agent: user_agent.into(),
            http,
        })
    }

    pub async fn search_conversations(
        &self,
        query: &str,
        limit: u32,
        page_token: Option<&str>,
    ) -> Result<Page<ConversationResponse>, ClientError> {
        let mut url = self.url(&["conversations", "search", query])?;
        {
            let mut pairs = url.query_pairs_mut();
            pairs.append_pair("limit", &limit.to_string());
            if let Some(page_token) = page_token {
                pairs.append_pair("page_token", page_token);
            }
        }
        self.get_json(url).await
    }

    pub async fn get_conversation(
        &self,
        conversation_id: &str,
    ) -> Result<ConversationResponse, ClientError> {
        self.get_json(self.url(&["conversations", conversation_id])?)
            .await
    }

    pub async fn list_conversation_messages(
        &self,
        conversation_id: &str,
        limit: u32,
    ) -> Result<Page<MessageResponse>, ClientError> {
        let mut url = self.url(&["conversations", conversation_id, "messages"])?;
        url.query_pairs_mut()
            .append_pair("limit", &limit.to_string());
        self.get_json(url).await
    }

    pub async fn list_inboxes(&self) -> Result<Page<InboxResponse>, ClientError> {
        self.get_json(self.url(&["inboxes"])?).await
    }

    pub async fn list_teammate_inboxes(
        &self,
        teammate_id: &str,
    ) -> Result<Page<InboxResponse>, ClientError> {
        self.get_json(self.url(&["teammates", teammate_id, "inboxes"])?)
            .await
    }

    pub async fn get_value(
        &self,
        segments: &[String],
        query: &[(String, String)],
    ) -> Result<serde_json::Value, ClientError> {
        let segments: Vec<_> = segments.iter().map(String::as_str).collect();
        let mut url = self.url(&segments)?;
        {
            let mut pairs = url.query_pairs_mut();
            for (name, value) in query {
                pairs.append_pair(name, value);
            }
        }
        self.get_json(url).await
    }

    fn url(&self, segments: &[&str]) -> Result<Url, ClientError> {
        let mut url = self.base_url.clone();
        let mut path = url
            .path_segments_mut()
            .map_err(|_| ClientError::InvalidBaseUrl)?;
        path.pop_if_empty();
        path.extend(segments);
        drop(path);
        Ok(url)
    }

    async fn get_json<T: DeserializeOwned>(&self, mut url: Url) -> Result<T, ClientError> {
        for redirects_followed in 0..=3 {
            let response = self
                .http
                .get(url.clone())
                .bearer_auth(self.token.expose_secret())
                .header(reqwest::header::USER_AGENT, &self.user_agent)
                .send()
                .await
                .map_err(ClientError::Transport)?;
            let status = response.status();
            if status == StatusCode::MOVED_PERMANENTLY
                && redirects_followed < 3
                && let Some(redirect_url) = self.approved_redirect_url(&url, &response)
            {
                url = redirect_url;
                continue;
            }

            let body = response.bytes().await.map_err(ClientError::Transport)?;
            if !status.is_success() {
                return Err(ClientError::Http {
                    status: status.as_u16(),
                    message: api_error_message(&body)
                        .unwrap_or_else(|| format!("HTTP {}", status.as_u16())),
                });
            }
            return serde_json::from_slice(&body).map_err(ClientError::Decode);
        }
        unreachable!("redirect loop always returns or follows at most three redirects")
    }

    fn approved_redirect_url(
        &self,
        current_url: &Url,
        response: &reqwest::Response,
    ) -> Option<Url> {
        let location = response
            .headers()
            .get(reqwest::header::LOCATION)?
            .to_str()
            .ok()?;
        let segments = validate_api_path(redirect_location_path(location)?).ok()?;
        let target = current_url.join(location).ok()?;
        if target.fragment().is_some() || !self.redirect_origin_is_approved(&target) {
            return None;
        }
        let segment_refs: Vec<_> = segments.iter().map(String::as_str).collect();
        let mut approved = self.url(&segment_refs).ok()?;
        approved.set_query(target.query());
        Some(approved)
    }

    fn redirect_origin_is_approved(&self, target: &Url) -> bool {
        target.origin() == self.base_url.origin()
            || (self.uses_production_base()
                && target.scheme() == "https"
                && target.host_str().is_some_and(|host| {
                    host == "api2.frontapp.com" || host.ends_with(".api.frontapp.com")
                }))
    }

    fn uses_production_base(&self) -> bool {
        self.base_url.scheme() == "https"
            && self.base_url.host_str() == Some("api2.frontapp.com")
            && self.base_url.port_or_known_default() == Some(443)
    }
}

fn redirect_location_path(location: &str) -> Option<&str> {
    let path_and_query = if let Some(scheme) = location.find("://") {
        let authority_and_path = &location[scheme + 3..];
        let path_start = authority_and_path.find(['/', '?', '#'])?;
        &authority_and_path[path_start..]
    } else {
        location
    };
    path_and_query.split(['?', '#']).next()
}

fn api_error_message(body: &[u8]) -> Option<String> {
    #[derive(serde::Deserialize)]
    struct Body {
        #[serde(rename = "_error")]
        error: Option<Detail>,
    }
    #[derive(serde::Deserialize)]
    struct Detail {
        message: Option<String>,
    }

    serde_json::from_slice::<Body>(body)
        .ok()?
        .error?
        .message
        .filter(|message| !message.is_empty())
}

pub fn classify_http(status: StatusCode) -> (&'static str, String) {
    match status.as_u16() {
        401 => (
            "UNAUTHORIZED",
            "Set FRONT_API_TOKEN or configure token_command in config file".into(),
        ),
        403 => (
            "FORBIDDEN",
            "Check that your API token has the required scopes".into(),
        ),
        404 => ("NOT_FOUND", "Check the resource ID and try again".into()),
        429 => ("RATE_LIMITED", "Wait and retry".into()),
        code => ("API_ERROR", format!("API returned status {code}")),
    }
}
