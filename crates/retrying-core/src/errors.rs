use std::fmt;

#[derive(Debug, Clone)]
pub struct RetryConfigurationError {
    msg: String,
}

impl RetryConfigurationError {
    pub fn from_str(msg: &str) -> RetryConfigurationError {
        RetryConfigurationError {
            msg: msg.to_string(),
        }
    }

    pub fn new(msg: String) -> RetryConfigurationError {
        RetryConfigurationError { msg }
    }
}

impl fmt::Display for RetryConfigurationError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "Retrying macros `retry` has incorrect configuration. Error: {}",
            self.msg
        )
    }
}
