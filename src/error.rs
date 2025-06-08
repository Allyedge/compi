use std::fmt;

use crate::util::{CommandError, FileError};

#[derive(Debug)]
pub enum CompiError {
    Task(String),
    Dependency(String),
    Io(std::io::Error),
    File(FileError),
    Command(CommandError),
    Parse(String),
}

impl fmt::Display for CompiError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            CompiError::Task(msg) => write!(f, "Task error: {}", msg),
            CompiError::Dependency(msg) => write!(f, "Dependency error: {}", msg),
            CompiError::Io(err) => write!(f, "IO error: {}", err),
            CompiError::File(err) => write!(f, "File error: {}", err),
            CompiError::Command(err) => write!(f, "Command error: {}", err),
            CompiError::Parse(msg) => write!(f, "Parse error: {}", msg),
        }
    }
}

impl std::error::Error for CompiError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            CompiError::Io(err) => Some(err),
            CompiError::File(err) => Some(err),
            CompiError::Command(err) => Some(err),
            _ => None,
        }
    }
}

impl From<std::io::Error> for CompiError {
    fn from(err: std::io::Error) -> Self {
        CompiError::Io(err)
    }
}

impl From<FileError> for CompiError {
    fn from(err: FileError) -> Self {
        CompiError::File(err)
    }
}

impl From<CommandError> for CompiError {
    fn from(err: CommandError) -> Self {
        CompiError::Command(err)
    }
}

impl From<toml::de::Error> for CompiError {
    fn from(err: toml::de::Error) -> Self {
        CompiError::Parse(err.to_string())
    }
}

pub type Result<T> = std::result::Result<T, CompiError>;
