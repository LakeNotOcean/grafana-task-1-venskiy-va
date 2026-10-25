use std::env::{self, VarError};
use std::fmt;
use std::str::FromStr;

/// Параметры запуска сервиса.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Config {
    pub port: u16,
    pub pod_id: String,
    pub datacenter: String,
    /// Каждый N-й запрос завершается 500. `0` отключает искусственные сбои.
    pub fail_every_n: u64,
}

/// Ошибка чтения конфигурации: имя переменной, её фактическое значение и причина.
#[derive(Debug, PartialEq, Eq)]
pub struct InvalidConfig {
    pub name: &'static str,
    pub value: String,
    pub reason: String,
}

impl fmt::Display for InvalidConfig {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "некорректное значение {} = {:?}: {}",
            self.name, self.value, self.reason
        )
    }
}

impl std::error::Error for InvalidConfig {}

impl Config {
    /// Читает конфигурацию из переменных окружения процесса.
    pub fn from_env() -> Result<Self, InvalidConfig> {
        // Замыкание, а не `env::var` напрямую: fn-item дженериковой `env::var`
        // не приводится к `for<'a> Fn(&'a str)`.
        Self::from_lookup(|name| env::var(name))
    }

    /// То же, что [`Config::from_env`], но источник значений передаётся явно —
    /// так конфигурацию можно тестировать, не трогая окружение процесса.
    pub fn from_lookup<F>(lookup: F) -> Result<Self, InvalidConfig>
    where
        F: Fn(&str) -> Result<String, VarError>,
    {
        Ok(Self {
            port: number_var(&lookup, "PORT", 8080)?,
            pod_id: string_var(&lookup, "POD_ID", "pod-unknown"),
            datacenter: string_var(&lookup, "DATACENTER", "dc-unknown"),
            fail_every_n: number_var(&lookup, "FAIL_EVERY_N", 50)?,
        })
    }
}

fn string_var<F>(lookup: &F, name: &str, default: &str) -> String
where
    F: Fn(&str) -> Result<String, VarError>,
{
    lookup(name).unwrap_or_else(|_| default.to_owned())
}

fn number_var<F, T>(lookup: &F, name: &'static str, default: T) -> Result<T, InvalidConfig>
where
    F: Fn(&str) -> Result<String, VarError>,
    T: FromStr,
    T::Err: std::error::Error,
{
    let raw = match lookup(name) {
        Ok(raw) => raw,
        Err(_) => return Ok(default),
    };
    raw.parse::<T>().map_err(|err| InvalidConfig {
        name,
        value: raw,
        reason: err.to_string(),
    })
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use super::*;

    /// Источник значений для тестов
    fn map_lookup(
        values: HashMap<&'static str, &'static str>,
    ) -> impl Fn(&str) -> Result<String, VarError> {
        move |name: &str| match values.get(name) {
            Some(value) => Ok((*value).to_owned()),
            None => Err(VarError::NotPresent),
        }
    }

    #[test]
    fn empty_environment_gives_defaults() {
        let config = Config::from_lookup(map_lookup(HashMap::new())).expect("конфигурация");

        assert_eq!(
            config,
            Config {
                port: 8080,
                pod_id: "pod-unknown".to_owned(),
                datacenter: "dc-unknown".to_owned(),
                fail_every_n: 50,
            }
        );
    }

    #[test]
    fn values_are_read_from_environment_source() {
        let config = Config::from_lookup(map_lookup(HashMap::from([
            ("PORT", "9001"),
            ("POD_ID", "pod-7"),
            ("DATACENTER", "dc-north"),
            ("FAIL_EVERY_N", "0"),
        ])))
        .expect("конфигурация");

        assert_eq!(
            config,
            Config {
                port: 9001,
                pod_id: "pod-7".to_owned(),
                datacenter: "dc-north".to_owned(),
                fail_every_n: 0,
            }
        );
    }

    #[test]
    fn invalid_number_is_reported_with_variable_name() {
        let error = Config::from_lookup(map_lookup(HashMap::from([("FAIL_EVERY_N", "много")])))
            .expect_err("ожидали ошибку разбора");

        assert_eq!(error.name, "FAIL_EVERY_N");
        assert_eq!(error.value, "много");
    }
}
