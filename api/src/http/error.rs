use axum::{
    Json,
    http::StatusCode,
    response::{IntoResponse, Response},
};
use garde::Report;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct FieldError {
    pub path: String,
    pub message: String,
}

#[derive(Debug, Serialize)]
pub struct ValidationError(pub Vec<FieldError>);

impl From<Report> for ValidationError {
    fn from(report: Report) -> Self {
        Self(
            report
                .iter()
                .map(|(path, error)| FieldError {
                    path: path.to_string(),
                    message: error.message().to_string(),
                })
                .collect(),
        )
    }
}

impl From<Vec<Report>> for ValidationError {
    fn from(reports: Vec<Report>) -> Self {
        Self(
            reports
                .into_iter()
                .flat_map(|r| {
                    r.iter()
                        .map(|(path, error)| FieldError {
                            path: path.to_string(),
                            message: error.message().to_string(),
                        })
                        .collect::<Vec<_>>()
                })
                .collect(),
        )
    }
}

impl IntoResponse for ValidationError {
    fn into_response(self) -> Response {
        (StatusCode::UNPROCESSABLE_ENTITY, Json(self.0)).into_response()
    }
}

#[cfg(test)]
mod tests;
