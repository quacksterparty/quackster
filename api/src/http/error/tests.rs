use axum::{body::to_bytes, response::IntoResponse};
use garde::Validate;
use serde::Deserialize;

use super::{FieldError, ValidationError};

#[derive(Debug, Deserialize, Validate)]
struct Sample {
    #[garde(length(min = 3))]
    name: String,
    #[garde(range(min = 1, max = 10))]
    n: u32,
}

#[test]
fn garde_report_becomes_flat_paths() {
    let bad = Sample {
        name: "x".into(),
        n: 99,
    };
    let report = bad.validate().unwrap_err();
    let err: ValidationError = report.into();

    let paths: Vec<&str> = err.0.iter().map(|f| f.path.as_str()).collect();
    assert!(paths.contains(&"name"), "missing path 'name': {paths:?}");
    assert!(paths.contains(&"n"), "missing path 'n': {paths:?}");
    assert_eq!(err.0.len(), 2);
}

#[tokio::test]
async fn returns_422_with_flat_json_body() {
    let mut err = ValidationError::empty();
    err.push("kind", "missing variant");
    err.push("license", "SPDX id not on allowlist");
    let response = err.into_response();
    assert_eq!(response.status(), 422);

    let body = to_bytes(response.into_body(), 1024).await.unwrap();
    let parsed: Vec<FieldError> = serde_json::from_slice(&body).unwrap();
    assert_eq!(parsed.len(), 2);
    assert_eq!(parsed[0].path, "kind");
    assert_eq!(parsed[1].path, "license");
}

#[test]
fn push_appends_in_order() {
    let mut err = ValidationError::empty();
    err.push("a", "one");
    err.push("b", "two");
    assert_eq!(err.0[0].message, "one");
    assert_eq!(err.0[1].message, "two");
}

#[test]
fn serializes_as_flat_array() {
    let mut err = ValidationError::empty();
    err.push("a.b", "bad");
    let json = serde_json::to_string(&err.0).unwrap();
    assert_eq!(json, r#"[{"path":"a.b","message":"bad"}]"#);
}
