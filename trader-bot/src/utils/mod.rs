use std::env;
use std::ffi::OsStr;

/// Получить переменную окружения
///
/// # Errors
/// Возвращает ошибку, если переменная окружения не установлена
pub fn from_env<K: AsRef<OsStr>>(name: K) -> Result<String, std::env::VarError> {
    env::var(name)
}
