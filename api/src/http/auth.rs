use std::sync::Arc;

use axum::{
    extract::{Request, State},
    http::{StatusCode, header},
    middleware::Next,
    response::Response,
};

use crate::state::AppState;

pub async fn auth(
    State(state): State<Arc<AppState>>,
    req: Request,
    next: Next,
) -> Result<Response, StatusCode> {
    let ok = req
        .headers()
        .get(header::AUTHORIZATION)
        .and_then(|header_value| header_value.to_str().ok())
        .and_then(|value| value.strip_prefix("Bearer "))
        .map(|token| {
            state
                .config
                .admin_secret
                .as_ref()
                .is_none_or(|secret| secret == token)
        });
    if state.config.admin_secret.is_none() || ok.unwrap_or(false) {
        Ok(next.run(req).await)
    } else {
        Err(StatusCode::UNAUTHORIZED)
    }
}
