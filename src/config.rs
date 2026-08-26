use std::{
    collections::BTreeMap,
    fs,
    path::{Path, PathBuf},
    process::Command,
};

use directories::BaseDirs;
use secrecy::SecretString;
use serde::{Deserialize, Serialize};

use crate::error::AppError;

#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
pub struct Config {
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub token_command: Vec<String>,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub user: String,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ConfigSource {
    Environment,
    TokenCommand,
    Config,
    None,
}

pub fn path() -> PathBuf {
    BaseDirs::new()
        .map(|dirs| dirs.config_dir().join("front").join("config.yaml"))
        .unwrap_or_else(|| PathBuf::from("front/config.yaml"))
}

pub fn load() -> Result<Config, AppError> {
    load_from(&path())
}

pub fn load_from(path: &Path) -> Result<Config, AppError> {
    let data = match fs::read(path) {
        Ok(data) => data,
        Err(source) if source.kind() == std::io::ErrorKind::NotFound => {
            return Ok(Config::default());
        }
        Err(source) => {
            return Err(AppError::ReadConfig {
                path: path.to_owned(),
                source,
            });
        }
    };

    serde_yaml_ng::from_slice(&data).map_err(|_| AppError::ParseConfig {
        path: path.to_owned(),
    })
}

pub fn resolve_token(
    config: &Config,
    env: &BTreeMap<String, String>,
) -> Result<SecretString, AppError> {
    if let Some(token) = env.get("FRONT_API_TOKEN").filter(|token| !token.is_empty()) {
        return Ok(SecretString::from(token.clone()));
    }

    let (program, args) = config
        .token_command
        .split_first()
        .ok_or(AppError::NoToken)?;
    if program.is_empty() {
        return Err(AppError::EmptyTokenCommand);
    }

    let output = Command::new(program)
        .args(args)
        .output()
        .map_err(AppError::TokenCommand)?;
    if !output.status.success() {
        return Err(AppError::TokenCommandStatus(output.status));
    }
    let token = String::from_utf8(output.stdout).map_err(|_| AppError::InvalidTokenOutput)?;
    let token = token.trim().to_owned();
    if token.is_empty() {
        return Err(AppError::EmptyTokenOutput);
    }
    Ok(SecretString::from(token))
}

pub fn token_source(config: &Config, env: &BTreeMap<String, String>) -> ConfigSource {
    if env
        .get("FRONT_API_TOKEN")
        .is_some_and(|token| !token.is_empty())
    {
        ConfigSource::Environment
    } else if !config.token_command.is_empty() {
        ConfigSource::TokenCommand
    } else {
        ConfigSource::None
    }
}

pub fn resolve_user(config: &Config, env: &BTreeMap<String, String>) -> String {
    env.get("FRONT_USER")
        .filter(|user| !user.is_empty())
        .cloned()
        .unwrap_or_else(|| config.user.clone())
}

pub fn user_source(config: &Config, env: &BTreeMap<String, String>) -> ConfigSource {
    if env.get("FRONT_USER").is_some_and(|user| !user.is_empty()) {
        ConfigSource::Environment
    } else if !config.user.is_empty() {
        ConfigSource::Config
    } else {
        ConfigSource::None
    }
}

pub fn current_env() -> BTreeMap<String, String> {
    ["FRONT_API_TOKEN", "FRONT_USER"]
        .into_iter()
        .filter_map(|name| {
            std::env::var(name)
                .ok()
                .map(|value| (name.to_owned(), value))
        })
        .collect()
}
