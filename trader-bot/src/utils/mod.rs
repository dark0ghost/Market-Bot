use std::env;
use std::ffi::OsStr;

pub fn from_env<K: AsRef<OsStr>>(name: K) -> String {
    env::var(name).unwrap()
}