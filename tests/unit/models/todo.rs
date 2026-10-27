use serde_json::json;
use todo_backend::{
    error::AppError,
    models::todo::{CreateTodo, ListParams, Page, UpdateTodo},
};

fn create(body: serde_json::Value) -> CreateTodo {
    serde_json::from_value(body).unwrap()
}

fn update(body: serde_json::Value) -> UpdateTodo {
    serde_json::from_value(body).unwrap()
}

fn is_validation_error<T: std::fmt::Debug>(result: Result<T, AppError>) -> bool {
    matches!(result, Err(AppError::Validation(_)))
}

#[test]
fn create_accepts_title_and_optional_description() {
    let new = create(json!({ "title": "buy milk", "description": "2 litres" }))
        .validate()
        .unwrap();
    assert_eq!(new.title, "buy milk");
    assert_eq!(new.description.as_deref(), Some("2 litres"));

    let new = create(json!({ "title": "buy milk" })).validate().unwrap();
    assert_eq!(new.description, None);
}

#[test]
fn create_trims_the_title() {
    let new = create(json!({ "title": "  buy milk \n" }))
        .validate()
        .unwrap();
    assert_eq!(new.title, "buy milk");
}

#[test]
fn create_rejects_empty_or_blank_title() {
    assert!(is_validation_error(
        create(json!({ "title": "" })).validate()
    ));
    assert!(is_validation_error(
        create(json!({ "title": "   " })).validate()
    ));
}

#[test]
fn title_limit_is_200_characters() {
    assert!(
        create(json!({ "title": "é".repeat(200) }))
            .validate()
            .is_ok()
    );
    assert!(is_validation_error(
        create(json!({ "title": "é".repeat(201) })).validate()
    ));
}

#[test]
fn description_limit_is_2000_characters() {
    let ok = json!({ "title": "x", "description": "d".repeat(2000) });
    let too_long = json!({ "title": "x", "description": "d".repeat(2001) });

    assert!(create(ok).validate().is_ok());
    assert!(is_validation_error(create(too_long).validate()));
}

#[test]
fn create_rejects_unknown_fields() {
    let result = serde_json::from_value::<CreateTodo>(json!({ "title": "x", "done": true }));
    assert!(result.is_err());
}

#[test]
fn update_distinguishes_omitted_and_null_description() {
    let omitted = update(json!({ "completed": true })).validate().unwrap();
    assert_eq!(omitted.description, None);

    let cleared = update(json!({ "description": null })).validate().unwrap();
    assert_eq!(cleared.description, Some(None));

    let set = update(json!({ "description": "new" })).validate().unwrap();
    assert_eq!(set.description, Some(Some("new".to_string())));
}

#[test]
fn update_passes_through_given_fields() {
    let changes = update(json!({ "title": " renamed ", "completed": true }))
        .validate()
        .unwrap();

    assert_eq!(changes.title.as_deref(), Some("renamed"));
    assert_eq!(changes.completed, Some(true));
}

#[test]
fn update_validates_title_and_description_like_create() {
    assert!(is_validation_error(
        update(json!({ "title": " " })).validate()
    ));
    assert!(is_validation_error(
        update(json!({ "title": "t".repeat(201) })).validate()
    ));
    assert!(is_validation_error(
        update(json!({ "description": "d".repeat(2001) })).validate()
    ));
}

#[test]
fn update_without_any_field_is_rejected() {
    assert!(is_validation_error(update(json!({})).validate()));
}

#[test]
fn update_rejects_unknown_fields() {
    let result = serde_json::from_value::<UpdateTodo>(json!({ "complete": true }));
    assert!(result.is_err());
}

fn params(limit: Option<u32>, offset: Option<u32>) -> ListParams {
    ListParams { limit, offset }
}

#[test]
fn list_params_default_to_first_page_of_50() {
    assert_eq!(
        ListParams::default().validate().unwrap(),
        Page {
            limit: 50,
            offset: 0
        }
    );
}

#[test]
fn list_params_accept_limit_and_offset() {
    assert_eq!(
        params(Some(100), Some(250)).validate().unwrap(),
        Page {
            limit: 100,
            offset: 250
        }
    );
}

#[test]
fn list_params_reject_limit_outside_1_to_100() {
    assert!(is_validation_error(params(Some(0), None).validate()));
    assert!(is_validation_error(params(Some(101), None).validate()));
}
