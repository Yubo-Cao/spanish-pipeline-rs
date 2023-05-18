use std::{error::Error, fmt::Display};

/// Represents an error that can occur during the execution of the CLI.
#[derive(Debug)]
pub struct CliError {
    message: String,
}

impl CliError {
    pub fn new(message: &str) -> Self {
        Self {
            message: message.to_string(),
        }
    }
}

impl Display for CliError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", &self.message)
    }
}

impl Error for CliError {
    fn description(&self) -> &str {
        &self.message
    }
}
