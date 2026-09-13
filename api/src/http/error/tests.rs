use axum::response::IntoResponse;
use garde::Validate;
use serde::Deserialize;

use super::ValidationError;

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
    let bad = Sample {
        name: "x".into(),
        n: 99,
    };
    let report = bad.validate().unwrap_err();
    let err: ValidationError = report.into();
    let response = err.into_response();
    assert_eq!(response.status(), 422);
    let json: Vec<serde_json::Value> =
        serde_json::from_slice(&axum::body::to_bytes(response.into_body(), 1024).await.unwrap())
            .unwrap();
    assert_eq!(json.len(), 2);
}
