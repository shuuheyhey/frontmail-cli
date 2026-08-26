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
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub profiles: BTreeMap<String, Profile>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub default_profile: Option<String>,
}

#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
pub struct Profile {
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

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ProfileSource {
    Explicit,
    Default,
    Single,
}

pub struct EffectiveConfig {
    profile_name: Option<String>,
    profile_source: Option<ProfileSource>,
    environment_token: Option<SecretString>,
    token_command: Vec<String>,
    user: String,
    token_source: ConfigSource,
    user_source: ConfigSource,
}

impl std::fmt::Debug for EffectiveConfig {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("EffectiveConfig")
            .field("profile_name", &self.profile_name)
            .field("profile_source", &self.profile_source)
            .field("token_source", &self.token_source)
            .field("user_source", &self.user_source)
            .field("token_command_configured", &self.token_command_configured())
            .field("user_configured", &!self.user.is_empty())
            .finish()
    }
}

impl EffectiveConfig {
    pub fn profile_name(&self) -> Option<&str> {
        self.profile_name.as_deref()
    }

    pub fn profile_source(&self) -> Option<ProfileSource> {
        self.profile_source
    }

    pub fn token_command_configured(&self) -> bool {
        !self.token_command.is_empty()
    }

    pub fn token_source(&self) -> ConfigSource {
        self.token_source
    }

    pub fn user(&self) -> &str {
        &self.user
    }

    pub fn user_source(&self) -> ConfigSource {
        self.user_source
    }

    pub fn resolve_token(&self) -> Result<SecretString, AppError> {
        if let Some(token) = &self.environment_token {
            return Ok(token.clone());
        }
        resolve_token_command(&self.token_command)
    }
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

pub fn select_effective_config(
    config: &Config,
    env: &BTreeMap<String, String>,
    explicit_profile: Option<&str>,
) -> Result<EffectiveConfig, AppError> {
    if let Some(name) = explicit_profile {
        let profile = config
            .profiles
            .get(name)
            .ok_or_else(|| AppError::UnknownProfile {
                name: name.into(),
                available: available_profiles(config),
            })?;
        return Ok(profile_config(name, ProfileSource::Explicit, profile));
    }

    let legacy = legacy_config(config, env);
    if legacy.token_source != ConfigSource::None || legacy.user_source != ConfigSource::None {
        return Ok(legacy);
    }

    if let Some(name) = config.default_profile.as_deref() {
        let profile = config
            .profiles
            .get(name)
            .ok_or_else(|| AppError::UnknownDefaultProfile {
                name: name.into(),
                available: available_profiles(config),
            })?;
        return Ok(profile_config(name, ProfileSource::Default, profile));
    }

    match config.profiles.len() {
        0 => Ok(legacy),
        1 => {
            let (name, profile) = config
                .profiles
                .first_key_value()
                .expect("one profile is present");
            Ok(profile_config(name, ProfileSource::Single, profile))
        }
        _ => Err(AppError::ProfileRequired {
            available: available_profiles(config),
        }),
    }
}

fn legacy_config(config: &Config, env: &BTreeMap<String, String>) -> EffectiveConfig {
    let environment_token = env
        .get("FRONT_API_TOKEN")
        .filter(|token| !token.is_empty())
        .cloned()
        .map(SecretString::from);
    EffectiveConfig {
        profile_name: None,
        profile_source: None,
        token_command: config.token_command.clone(),
        user: resolve_user(config, env),
        token_source: token_source(config, env),
        user_source: user_source(config, env),
        environment_token,
    }
}

fn profile_config(name: &str, source: ProfileSource, profile: &Profile) -> EffectiveConfig {
    EffectiveConfig {
        profile_name: Some(name.into()),
        profile_source: Some(source),
        environment_token: None,
        token_command: profile.token_command.clone(),
        user: profile.user.clone(),
        token_source: if profile.token_command.is_empty() {
            ConfigSource::None
        } else {
            ConfigSource::TokenCommand
        },
        user_source: if profile.user.is_empty() {
            ConfigSource::None
        } else {
            ConfigSource::Config
        },
    }
}

fn available_profiles(config: &Config) -> String {
    if config.profiles.is_empty() {
        "(none)".into()
    } else {
        config
            .profiles
            .keys()
            .cloned()
            .collect::<Vec<_>>()
            .join(", ")
    }
}

pub fn resolve_token(
    config: &Config,
    env: &BTreeMap<String, String>,
) -> Result<SecretString, AppError> {
    if let Some(token) = env.get("FRONT_API_TOKEN").filter(|token| !token.is_empty()) {
        return Ok(SecretString::from(token.clone()));
    }

    resolve_token_command(&config.token_command)
}

fn resolve_token_command(token_command: &[String]) -> Result<SecretString, AppError> {
    let (program, args) = token_command.split_first().ok_or(AppError::NoToken)?;
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
