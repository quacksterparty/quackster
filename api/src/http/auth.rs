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

#[cfg(test)]
mod tests {
    use super::*;

    use std::sync::{Arc as StdArc, RwLock};

    use axum::body::Body;
    use axum::http::Request as HttpRequest;
    use axum::{Router, routing::get};
    use dashmap::DashMap;
    use tower::ServiceExt;

    use crate::config::AppConfig;
    use crate::data::load_dataset;
    use crate::media::MediaFetcher;

    fn state_with_secret(secret: Option<&str>) -> StdArc<AppState> {
        let tmp = tempfile::tempdir().unwrap();
        for sub in ["questions", "packs", "tags", "i18n", "media"] {
            std::fs::create_dir_all(tmp.path().join(sub)).unwrap();
        }
        let ds = load_dataset(tmp.path()).expect("load empty dataset");
        let mut config = AppConfig::default();
        config.admin_secret = secret.map(str::to_owned);
        StdArc::new(AppState {
            config,
            data: StdArc::new(RwLock::new(ds)),
            rooms: DashMap::new(),
            media: StdArc::new(MediaFetcher::disabled()),
        })
    }

    fn protected_app(state: StdArc<AppState>) -> Router {
        Router::new()
            .route("/probe", get(|| async { "ok" }))
            .layer(axum::middleware::from_fn_with_state(
                state.clone(),
                auth,
            ))
            .with_state(state)
    }

    #[tokio::test]
    async fn no_secret_configured_passes_everything() {
        let state = state_with_secret(None);
        let app = protected_app(state);
        let response = app
            .oneshot(HttpRequest::builder().uri("/probe").body(Body::empty()).unwrap())
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn secret_configured_without_header_returns_401() {
        let state = state_with_secret(Some("s3cret"));
        let app = protected_app(state);
        let response = app
            .oneshot(HttpRequest::builder().uri("/probe").body(Body::empty()).unwrap())
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
    }

    #[tokio::test]
    async fn secret_configured_with_wrong_bearer_returns_401() {
        let state = state_with_secret(Some("s3cret"));
        let app = protected_app(state);
        let response = app
            .oneshot(
                HttpRequest::builder()
                    .uri("/probe")
                    .header(header::AUTHORIZATION, "Bearer not-the-secret")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
    }

    #[tokio::test]
    async fn secret_configured_with_malformed_auth_header_returns_401() {
        let state = state_with_secret(Some("s3cret"));
        let app = protected_app(state);
        let response = app
            .oneshot(
                HttpRequest::builder()
                    .uri("/probe")
                    .header(header::AUTHORIZATION, "Basic dXNlcjpwYXNz")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
    }

    #[tokio::test]
    async fn secret_configured_with_correct_bearer_returns_200() {
        let state = state_with_secret(Some("s3cret"));
        let app = protected_app(state);
        let response = app
            .oneshot(
                HttpRequest::builder()
                    .uri("/probe")
                    .header(header::AUTHORIZATION, "Bearer s3cret")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
    }
}
