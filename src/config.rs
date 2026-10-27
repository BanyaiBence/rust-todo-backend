use std::net::SocketAddr;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LogFormat {
    Pretty,
    Json,
}

#[derive(Debug, Clone)]
pub struct Config {
    pub database_url: String,
    pub jwt_secret: String,
    pub jwt_ttl_seconds: u64,
    pub bind_addr: SocketAddr,
    pub log_format: LogFormat,
}

#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum ConfigError {
    #[error("missing required environment variable `{0}`")]
    Missing(&'static str),
    #[error("invalid value for environment variable `{name}`: {reason}")]
    Invalid { name: &'static str, reason: String },
}

impl Config {
    /// Loads `.env` (if present) and reads configuration from the process environment.
    pub fn from_env() -> Result<Self, ConfigError> {
        dotenvy::dotenv().ok();
        Self::from_lookup(|key| std::env::var(key).ok())
    }

    /// Builds the config from an arbitrary key lookup, so parsing can be tested
    /// without touching the process environment.
    pub fn from_lookup(lookup: impl Fn(&str) -> Option<String>) -> Result<Self, ConfigError> {
        let required = |name: &'static str| lookup(name).ok_or(ConfigError::Missing(name));
        let optional =
            |name: &str, default: &str| lookup(name).unwrap_or_else(|| default.to_string());

        let database_url = required("DATABASE_URL")?;
        let jwt_secret = required("JWT_SECRET")?;
        if jwt_secret.is_empty() {
            return Err(invalid("JWT_SECRET", "must not be empty"));
        }

        let jwt_ttl_seconds = optional("JWT_TTL_SECONDS", "3600")
            .parse()
            .map_err(|e| invalid("JWT_TTL_SECONDS", e))?;

        let bind_addr = optional("BIND_ADDR", "0.0.0.0:3000")
            .parse()
            .map_err(|e| invalid("BIND_ADDR", e))?;

        let log_format = match optional("LOG_FORMAT", "pretty")
            .to_ascii_lowercase()
            .as_str()
        {
            "pretty" => LogFormat::Pretty,
            "json" => LogFormat::Json,
            other => {
                return Err(invalid(
                    "LOG_FORMAT",
                    format!("expected `pretty` or `json`, got `{other}`"),
                ));
            }
        };

        Ok(Self {
            database_url,
            jwt_secret,
            jwt_ttl_seconds,
            bind_addr,
            log_format,
        })
    }
}

fn invalid(name: &'static str, reason: impl ToString) -> ConfigError {
    ConfigError::Invalid {
        name,
        reason: reason.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    fn lookup_from(pairs: &[(&str, &str)]) -> impl Fn(&str) -> Option<String> {
        let map: HashMap<String, String> = pairs
            .iter()
            .map(|(k, v)| (k.to_string(), v.to_string()))
            .collect();
        move |key| map.get(key).cloned()
    }

    const REQUIRED: &[(&str, &str)] = &[
        ("DATABASE_URL", "postgres://todo:todo@localhost:5432/todo"),
        ("JWT_SECRET", "secret"),
    ];

    #[test]
    fn parses_required_values_and_applies_defaults() {
        let config = Config::from_lookup(lookup_from(REQUIRED)).unwrap();

        assert_eq!(
            config.database_url,
            "postgres://todo:todo@localhost:5432/todo"
        );
        assert_eq!(config.jwt_secret, "secret");
        assert_eq!(config.jwt_ttl_seconds, 3600);
        assert_eq!(config.bind_addr, "0.0.0.0:3000".parse().unwrap());
        assert_eq!(config.log_format, LogFormat::Pretty);
    }

    #[test]
    fn optional_values_override_defaults() {
        let mut pairs = REQUIRED.to_vec();
        pairs.extend([
            ("JWT_TTL_SECONDS", "60"),
            ("BIND_ADDR", "127.0.0.1:8080"),
            ("LOG_FORMAT", "json"),
        ]);

        let config = Config::from_lookup(lookup_from(&pairs)).unwrap();

        assert_eq!(config.jwt_ttl_seconds, 60);
        assert_eq!(config.bind_addr, "127.0.0.1:8080".parse().unwrap());
        assert_eq!(config.log_format, LogFormat::Json);
    }

    #[test]
    fn missing_database_url_is_an_error() {
        let err = Config::from_lookup(lookup_from(&[("JWT_SECRET", "secret")])).unwrap_err();
        assert_eq!(err, ConfigError::Missing("DATABASE_URL"));
    }

    #[test]
    fn missing_jwt_secret_is_an_error() {
        let err =
            Config::from_lookup(lookup_from(&[("DATABASE_URL", "postgres://x")])).unwrap_err();
        assert_eq!(err, ConfigError::Missing("JWT_SECRET"));
    }

    #[test]
    fn empty_jwt_secret_is_rejected() {
        let err = Config::from_lookup(lookup_from(&[
            ("DATABASE_URL", "postgres://x"),
            ("JWT_SECRET", ""),
        ]))
        .unwrap_err();
        assert!(matches!(
            err,
            ConfigError::Invalid {
                name: "JWT_SECRET",
                ..
            }
        ));
    }

    #[test]
    fn invalid_values_are_rejected() {
        for (name, value) in [
            ("JWT_TTL_SECONDS", "soon"),
            ("BIND_ADDR", "not-an-addr"),
            ("LOG_FORMAT", "xml"),
        ] {
            let mut pairs = REQUIRED.to_vec();
            pairs.push((name, value));

            let err = Config::from_lookup(lookup_from(&pairs)).unwrap_err();
            assert!(
                matches!(err, ConfigError::Invalid { name: n, .. } if n == name),
                "{name}={value} should be invalid, got {err:?}"
            );
        }
    }
}
