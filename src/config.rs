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
    /// Origins allowed to call the API from a browser; `["*"]` allows any. Empty disables CORS.
    pub cors_allowed_origins: Vec<String>,
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

        let cors_allowed_origins = parse_origins(&optional("CORS_ALLOWED_ORIGINS", ""))?;

        Ok(Self {
            database_url,
            jwt_secret,
            jwt_ttl_seconds,
            bind_addr,
            log_format,
            cors_allowed_origins,
        })
    }
}

/// Parses a comma-separated origin list; `*` alone allows any origin.
fn parse_origins(raw: &str) -> Result<Vec<String>, ConfigError> {
    let origins: Vec<String> = raw
        .split(',')
        .map(str::trim)
        .filter(|origin| !origin.is_empty())
        .map(str::to_string)
        .collect();

    if origins == ["*"] {
        return Ok(origins);
    }
    if let Some(bad) = origins
        .iter()
        .find(|origin| !(origin.starts_with("http://") || origin.starts_with("https://")))
    {
        return Err(invalid(
            "CORS_ALLOWED_ORIGINS",
            format!("`{bad}` is not an http(s) origin (or use `*` alone)"),
        ));
    }
    Ok(origins)
}

fn invalid(name: &'static str, reason: impl ToString) -> ConfigError {
    ConfigError::Invalid {
        name,
        reason: reason.to_string(),
    }
}
