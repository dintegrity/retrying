pub use rand;
pub use retrying_core::retry;
use std::str::FromStr;
pub use std::thread::sleep as sleep_sync;
pub use std::time::Duration;

#[doc(hidden)]
#[cfg(feature = "tokio")]
pub use tokio::time::sleep as sleep_async;

#[doc(hidden)]
#[cfg(feature = "async_std")]
pub use async_std::task::sleep as sleep_async;

use std::fmt;

#[derive(Debug, Clone)]
pub struct RetryingError {
    pub msg: String,
}

impl RetryingError {
    pub fn from_str(msg: &str) -> RetryingError {
        RetryingError {
            msg: msg.to_string(),
        }
    }

    pub fn from_string(msg: String) -> RetryingError {
        RetryingError { msg: msg }
    }
}

impl fmt::Display for RetryingError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "Retry failed with error: {}", self.msg)
    }
}

////////////////////////////////////////////////////////////
//                      Public functions
////////////////////////////////////////////////////////////


pub fn overrite_by_env<T: FromStr>(
    original: T,
    prefix: &str,
    name: &str,
) -> T {

    let os_variable = format!("{}__{}", prefix, name);

    match get_env_case_insensitive(&os_variable) {
        Ok(Some(v)) => match v.parse::<T>() {
            Ok(parsed) => parsed,
            Err(_) => {
                eprint!("Failed to parse OS env variable '{}' with value '{}'.", os_variable, v);
                original
            }
        },
        Err(RetryingError { msg }) => {
            eprint!("Failed to get OS env variable '{}'. Error: {} ", os_variable, msg);
            original
        },
        Ok(None) => original
    }
}

////////////////////////////////////////////////////////////
//                      Private functions
////////////////////////////////////////////////////////////

fn get_env_case_insensitive(environment: &String) -> Result<Option<String>, RetryingError> {
    if environment.is_empty() {
        Ok(None)
    } else {
        let vars: Vec<(std::ffi::OsString, std::ffi::OsString)> = std::env::vars_os()
            .filter(|(k, _)| {
                k.to_str().unwrap_or("").to_string().to_uppercase() == environment.to_uppercase()
            })
            .collect();

        if vars.is_empty() {
            Ok(None)
        } else if vars.len() > 1 {
            let vars_str = vars
                .iter()
                .map(|(k, _)| k.to_str().unwrap().to_string())
                .collect::<Vec<_>>()
                .join(", ");
            let msg = format!("More than one environment is available for pattern {}. Available variables: {}. Please unset unnecessary variables and leave exactly one.", environment, vars_str);
            Err(RetryingError::from_string(msg))
        } else {
            let (name, value) = vars.first().unwrap();
            let name = name.to_str().unwrap_or("<Unspecified>");

            match value.to_str() {
                Some(v) if !v.is_empty() => Ok(Some(v.to_string())),
                _ => Err(RetryingError::from_string(format!(
                    "OS environment {} is is empty or has non-UTF-8 format",
                    name
                ))),
            }
        }
    }
}
