use std::path::PathBuf;

#[derive(Debug, thiserror::Error)]
pub enum AppError {
    #[error("read config {path}: {source}")]
    ReadConfig {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
    #[error("parse config {path}")]
    ParseConfig { path: PathBuf },
    #[error("no API token configured")]
    NoToken,
    #[error("token_command is empty")]
    EmptyTokenCommand,
    #[error("token_command failed: {0}")]
    TokenCommand(#[source] std::io::Error),
    #[error("token_command exited with status {0}")]
    TokenCommandStatus(std::process::ExitStatus),
    #[error("token_command returned empty output")]
    EmptyTokenOutput,
    #[error("invalid UTF-8 from token_command")]
    InvalidTokenOutput,
}

impl AppError {
    pub fn code(&self) -> &'static str {
        match self {
            Self::ReadConfig { .. } | Self::ParseConfig { .. } => "CONFIG_ERROR",
            Self::NoToken
            | Self::EmptyTokenCommand
            | Self::TokenCommand(_)
            | Self::TokenCommandStatus(_)
            | Self::EmptyTokenOutput
            | Self::InvalidTokenOutput => "UNAUTHORIZED",
        }
    }
}
