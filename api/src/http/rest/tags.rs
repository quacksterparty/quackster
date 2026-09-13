//! `/api/tags` — list + by id. Tags have no draft state; the registry is
//! always fully published.

use std::sync::Arc;

use axum::{
    Json, Router,
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
    routing::get,
};

use crate::data::Tag;
use crate::state::AppState;

pub fn router() -> Router<Arc<AppState>> {
    Router::new()
        .route("/tags", get(list_tags))
        .route("/tags/{id}", get(get_tag))
}

async fn list_tags(State(state): State<Arc<AppState>>) -> Json<Vec<Tag>> {
    let data = state.read_dataset();
    let mut out: Vec<Tag> = data.tags.values().map(|e| e.item.clone()).collect();
    out.sort_by(|a, b| a.id.cmp(&b.id));
    Json(out)
}

async fn get_tag(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> impl IntoResponse {
    let data = state.read_dataset();
    match data.tags.get(&id) {
        Some(entry) => Json(entry.item.clone()).into_response(),
        None => StatusCode::NOT_FOUND.into_response(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use std::fs;
    use std::sync::{Arc as StdArc, RwLock};

    use axum::{body::Body, http::Request};
    use dashmap::DashMap;
    use tempfile::TempDir;
    use tower::ServiceExt;

    use crate::config::AppConfig;
    use crate::data::{load_dataset, run_cross_file_checks, Dataset};
    use crate::media::MediaFetcher;

    fn fixture(files: &[(&str, &str)]) -> TempDir {
        let tmp = tempfile::tempdir().expect("tempdir");
        for dir in &["questions", "packs", "tags", "i18n", "media"] {
            fs::create_dir_all(tmp.path().join(dir)).unwrap();
        }
        for (path, content) in files {
            let full = tmp.path().join(path);
            if let Some(parent) = full.parent() {
                fs::create_dir_all(parent).unwrap();
            }
            fs::write(&full, content).unwrap();
        }
        tmp
    }

    fn load(files: &[(&str, &str)]) -> Dataset {
        let mut ds = load_dataset(fixture(files).path()).expect("load");
        ds.issues.extend(run_cross_file_checks(&ds));
        ds
    }

    fn test_state(ds: Dataset) -> StdArc<AppState> {
        StdArc::new(AppState {
            config: AppConfig::default(),
            data: StdArc::new(RwLock::new(ds)),
            rooms: DashMap::new(),
            media: StdArc::new(MediaFetcher::disabled()),
        })
    }

    const TAG_FIXTURE: &str = r#"
- id: subject:geo
  default_lang: en
  label: Geography
- id: subject:history
  default_lang: en
  label: History
"#;

    #[tokio::test]
    async fn list_returns_tags_sorted_by_id() {
        let ds = load(&[("tags/subject.yaml", TAG_FIXTURE)]);
        let app = router().with_state(test_state(ds));
        let body = axum::body::to_bytes(
            app.oneshot(
                Request::builder()
                    .uri("/tags")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap()
            .into_body(),
            4096,
        )
        .await
        .unwrap();
        let parsed: Vec<serde_json::Value> = serde_json::from_slice(&body).unwrap();
        let ids: Vec<&str> = parsed
            .iter()
            .map(|v| v.get("id").and_then(|x| x.as_str()).unwrap())
            .collect();
        assert_eq!(ids, vec!["subject:geo", "subject:history"]);
    }

    #[tokio::test]
    async fn by_id_returns_404_when_missing() {
        let ds = load(&[("tags/subject.yaml", TAG_FIXTURE)]);
        let app = router().with_state(test_state(ds));
        let response = app
            .oneshot(
                Request::builder()
                    .uri("/tags/subject:nope")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::NOT_FOUND);
    }
}
