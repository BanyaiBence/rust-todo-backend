use todo_backend::{error::AppError, models::user::Credentials};

fn creds(username: &str, password: &str) -> Credentials {
    Credentials {
        username: username.to_string(),
        password: password.to_string(),
    }
}

#[test]
fn accepts_valid_credentials() {
    assert!(
        creds("alice_01-x", "longenough")
            .validate_for_registration()
            .is_ok()
    );
}

#[test]
fn rejects_invalid_usernames() {
    for username in [
        "ab",
        &"a".repeat(33),
        "has space",
        "ünïcode",
        "semi;colon",
        "",
    ] {
        assert!(
            matches!(
                creds(username, "longenough").validate_for_registration(),
                Err(AppError::Validation(_))
            ),
            "username {username:?} should be rejected"
        );
    }
}

#[test]
fn rejects_invalid_passwords() {
    for password in ["short", &"p".repeat(129)] {
        assert!(
            matches!(
                creds("alice", password).validate_for_registration(),
                Err(AppError::Validation(_))
            ),
            "password of length {} should be rejected",
            password.len()
        );
    }
}

#[test]
fn password_length_counts_characters_not_bytes() {
    assert!(creds("alice", "ééééé").validate_for_registration().is_err());
    assert!(
        creds("alice", "éééééééé")
            .validate_for_registration()
            .is_ok()
    );
}
