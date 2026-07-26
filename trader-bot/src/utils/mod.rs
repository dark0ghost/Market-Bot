use std::env;
use std::ffi::OsStr;

/// Get environment variable
///
/// # Errors
/// Returns an error if the environment variable is not set
pub fn from_env<K: AsRef<OsStr>>(name: K) -> Result<String, std::env::VarError> {
    env::var(name)
}
