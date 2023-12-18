pub use rand;
pub use retrying_core::retry;
pub use std::thread::sleep as sleep_sync;
pub use std::time::Duration;
use std::time::SystemTime;

pub mod envs;
pub mod stop;
pub mod wait;

#[cfg(all(feature = "tokio", feature = "async_std"))]
compile_error!(
    "feature \"tokio\" and \"async_std\" cannot be enabled at the same time for retrying"
);

#[doc(hidden)]
#[cfg(feature = "tokio")]
pub use tokio::time::sleep as sleep_async;

#[doc(hidden)]
#[cfg(feature = "async_std")]
pub use async_std::task::sleep as sleep_async;

use std::fmt;
use std::str::FromStr;

#[derive(Debug, Clone, PartialEq)]
pub struct RetryingError {
    pub msg: String,
}

impl RetryingError {
    pub fn new(msg: &str) -> RetryingError {
        RetryingError {
            msg: msg.to_string(),
        }
    }
}

impl fmt::Display for RetryingError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "Retry failed with error: {}", self.msg)
    }
}

#[derive(Debug)]
pub struct RetryingContext {
    attempt_num: u32,
    start_time: SystemTime,
}

impl RetryingContext {
    pub fn new() -> Self {
        RetryingContext {
            attempt_num: 1,
            start_time: ::std::time::SystemTime::now(),
        }
    }

    pub fn started_at(&self) -> SystemTime {
        self.start_time
    }

    pub fn add_attempt(&mut self) {
        self.attempt_num += 1;
    }
}

impl Default for RetryingContext {
    fn default() -> Self {
        RetryingContext::new()
    }
}

/// read retrying environment using `prefix` and `name` and return the value from environment if it exists and has correct format.
/// Otherwise method prints error to stderr and returns `original` value.
/// This method is a part of developer API and should not be used directly (it is public because `retry` macros uses it together with `envs_prefix` configuration option).
pub fn override_by_env<T: FromStr>(original: T, prefix: &str, name: &str) -> T {
    let os_variable = format!("{}__{}", prefix, name);

    match get_env_case_insensitive(&os_variable) {
        Ok(Some(v)) => match v.parse::<T>() {
            Ok(parsed) => parsed,
            Err(_) => {
                eprint!(
                    "Failed to parse OS env variable '{}' with value '{}'.",
                    os_variable, v
                );
                original
            }
        },
        Ok(None) => original,
        Err(RetryingError { msg }) => {
            eprint!(
                "Failed to get OS env variable '{}'. Error: {} ",
                os_variable, msg
            );
            original
        }
    }
}

fn get_env_case_insensitive(environment: &String) -> Result<Option<String>, RetryingError> {
    if environment.is_empty() {
        Ok(None)
    } else {
        let mut vars = std::env::vars_os()
            .filter_map(|(k, v)| {
                let name = k.to_str();
                let value = v.to_str();
                if let Some(name) = name {
                    if name.to_uppercase() == environment.to_uppercase() {
                        return value.map(|v| Some((name.to_string(), v.to_string())));
                    }
                }
                None
            })
            .map(|f| f.unwrap());

        if let Some((_, value)) = vars.next() {
            if vars.next().is_some() {
                Err(RetryingError { msg: format!("More than one environment is available for pattern {}. Please unset unnecessary variables and leave exactly one.", environment) })
            } else {
                Ok(Some(value))
            }
        } else {
            Ok(None)
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::*;

    #[test]
    fn test_override_by_env() {
        let result = override_by_env::<f32>(0.5f32, "MY_METHOD", "TEST");
        assert_eq!(result, 0.5f32);

        std::env::set_var("MY_METHOD__TEST", "3.5");
        let result = override_by_env::<f32>(0.5f32, "MY_METHOD", "TEST");
        assert_eq!(result, 3.5f32);

        std::env::remove_var("MY_METHOD__TEST")
    }

    #[test]
    fn test_get_env_case_insensitive() {
        let testing_env = String::from("MY_METHOD__RETRYING__STOP__ATTEMPTS");

        std::env::set_var(&testing_env, "3");
        let result = get_env_case_insensitive(&testing_env.to_lowercase());
        assert_eq!(result.ok(), Some(Some(String::from("3"))));

        std::env::set_var(testing_env.to_lowercase(), "5");
        let result = get_env_case_insensitive(&testing_env.to_lowercase());
        assert_eq!(result.err(), Some(RetryingError{msg: String::from("More than one environment is available for pattern my_method__retrying__stop__attempts. Please unset unnecessary variables and leave exactly one.")}));

        std::env::remove_var(&testing_env);
        std::env::remove_var(testing_env.to_lowercase());
    }
}
