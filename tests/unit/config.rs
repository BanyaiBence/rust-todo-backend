use std::collections::HashMap;

use todo_backend::config::{Config, ConfigError, LogFormat};

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
    assert!(config.cors_allowed_origins.is_empty());
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
    let err = Config::from_lookup(lookup_from(&[("DATABASE_URL", "postgres://x")])).unwrap_err();
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

#[test]
fn cors_origins_are_a_comma_separated_list() {
    let mut pairs = REQUIRED.to_vec();
    pairs.push((
        "CORS_ALLOWED_ORIGINS",
        " https://app.example.com, http://localhost:5173 ,",
    ));

    let config = Config::from_lookup(lookup_from(&pairs)).unwrap();

    assert_eq!(
        config.cors_allowed_origins,
        ["https://app.example.com", "http://localhost:5173"]
    );
}

#[test]
fn cors_wildcard_is_accepted() {
    let mut pairs = REQUIRED.to_vec();
    pairs.push(("CORS_ALLOWED_ORIGINS", "*"));

    let config = Config::from_lookup(lookup_from(&pairs)).unwrap();

    assert_eq!(config.cors_allowed_origins, ["*"]);
}

#[test]
fn cors_origins_must_be_http_urls() {
    for value in [
        "app.example.com",
        "ftp://files.example.com",
        "https://ok.example, nope",
    ] {
        let mut pairs = REQUIRED.to_vec();
        pairs.push(("CORS_ALLOWED_ORIGINS", value));

        let err = Config::from_lookup(lookup_from(&pairs)).unwrap_err();
        assert!(
            matches!(
                err,
                ConfigError::Invalid {
                    name: "CORS_ALLOWED_ORIGINS",
                    ..
                }
            ),
            "{value:?} should be invalid, got {err:?}"
        );
    }
}
