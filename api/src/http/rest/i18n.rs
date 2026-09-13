//! `/api/i18n/{lang}/{questions,packs,games,tags}` — locale overlay documents.
//!
//! Returns the raw overlay as parsed from `data/i18n/{lang}/...`. Frontend
//! merges these into canonical reads (e.g. overlay `label` over canonical
//! `label`). 404 if the locale has no overlay document for the resource.

use std::sync::Arc;

use axum::{
    Json, Router,
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
    routing::get,
};

use crate::data::{GameConfigOverlay, PackOverlay, QuestionOverlay, TagOverlay};
use crate::state::AppState;

pub fn router() -> Router<Arc<AppState>> {
    Router::new()
        .route("/i18n/{lang}/questions", get(questions_overlay))
        .route("/i18n/{lang}/packs", get(packs_overlay))
        .route("/i18n/{lang}/games", get(games_overlay))
        .route("/i18n/{lang}/tags", get(tags_overlay))
}

fn not_found() -> impl IntoResponse {
    StatusCode::NOT_FOUND
}

async fn questions_overlay(
    State(state): State<Arc<AppState>>,
    Path(lang): Path<String>,
) -> impl IntoResponse {
    let data = state.read_dataset();
    let Some(overlays) = data.overlays.get(&lang) else {
        return not_found().into_response();
    };
    let items: Vec<&QuestionOverlay> = overlays.questions.values().map(|e| &e.item).collect();
    if items.is_empty() {
        return not_found().into_response();
    }
    Json(items).into_response()
}

async fn packs_overlay(
    State(state): State<Arc<AppState>>,
    Path(lang): Path<String>,
) -> impl IntoResponse {
    let data = state.read_dataset();
    let Some(overlays) = data.overlays.get(&lang) else {
        return not_found().into_response();
    };
    let items: Vec<&PackOverlay> = overlays.packs.values().map(|e| &e.item).collect();
    if items.is_empty() {
        return not_found().into_response();
    }
    Json(items).into_response()
}

async fn games_overlay(
    State(state): State<Arc<AppState>>,
    Path(lang): Path<String>,
) -> impl IntoResponse {
    let data = state.read_dataset();
    let Some(overlays) = data.overlays.get(&lang) else {
        return not_found().into_response();
    };
    let items: Vec<&GameConfigOverlay> = overlays.games.values().map(|e| &e.item).collect();
    if items.is_empty() {
        return not_found().into_response();
    }
    Json(items).into_response()
}

async fn tags_overlay(
    State(state): State<Arc<AppState>>,
    Path(lang): Path<String>,
) -> impl IntoResponse {
    let data = state.read_dataset();
    let Some(overlays) = data.overlays.get(&lang) else {
        return not_found().into_response();
    };
    let items: Vec<&TagOverlay> = overlays.tags.values().map(|e| &e.item).collect();
    if items.is_empty() {
        return not_found().into_response();
    }
    Json(items).into_response()
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

    fn tags_yaml() -> Vec<(&'static str, &'static str)> {
        vec![
            ("tags/audience.yaml", "[]\n"),
            ("tags/difficulty.yaml", "[]\n"),
            ("tags/format.yaml", "[]\n"),
            ("tags/region.yaml", "[]\n"),
            (
                "tags/subject.yaml",
                "- id: subject:geo\n  default_lang: en\n  label: Geography\n",
            ),
            ("tags/warning.yaml", "[]\n"),
        ]
    }

    fn load(extra: &[(&str, &str)]) -> Dataset {
        let mut files: Vec<(&str, &str)> = tags_yaml();
        files.extend_from_slice(extra);
        let mut ds = load_dataset(fixture(&files).path()).expect("load");
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

    const Q_FIXTURE: &str = r#"
- id: q_alpha
  kind: text
  tags: [subject:geo]
  content:
    default_lang: en
    prompt: { text: "?" }
    answer: a
    variants:
      open:
        accepted: ["a"]
"#;

    const OVERLAY_DE: &str = r#"
- id: q_alpha
  content:
    explanation: Eine Erklärung auf Deutsch.
"#;

    const TAG_OVERLAY_DE: &str = r#"
- id: subject:geo
  label: Geografie
"#;

    #[tokio::test]
    async fn questions_overlay_returns_overlay_for_locale() {
        let ds = load(&[
            ("questions/q.yaml", Q_FIXTURE),
            ("i18n/de/questions/q.yaml", OVERLAY_DE),
        ]);
        let app = router().with_state(test_state(ds));
        let response = app
            .oneshot(
                Request::builder()
                    .uri("/i18n/de/questions")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        let body = axum::body::to_bytes(response.into_body(), 4096).await.unwrap();
        let parsed: Vec<serde_json::Value> = serde_json::from_slice(&body).unwrap();
        assert_eq!(parsed.len(), 1);
        assert_eq!(parsed[0].get("id").and_then(|v| v.as_str()), Some("q_alpha"));
    }

    #[tokio::test]
    async fn unknown_locale_returns_404() {
        let ds = load(&[("questions/q.yaml", Q_FIXTURE)]);
        let app = router().with_state(test_state(ds));
        let response = app
            .oneshot(
                Request::builder()
                    .uri("/i18n/fr/questions")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::NOT_FOUND);
    }

    #[tokio::test]
    async fn tags_overlay_returns_overlay_for_locale() {
        let ds = load(&[("i18n/de/tags/subject.yaml", TAG_OVERLAY_DE)]);
        let app = router().with_state(test_state(ds));
        let response = app
            .oneshot(
                Request::builder()
                    .uri("/i18n/de/tags")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        let body = axum::body::to_bytes(response.into_body(), 4096).await.unwrap();
        let parsed: Vec<serde_json::Value> = serde_json::from_slice(&body).unwrap();
        assert_eq!(parsed.len(), 1);
        assert_eq!(parsed[0].get("id").and_then(|v| v.as_str()), Some("subject:geo"));
    }
}
