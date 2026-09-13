//! `/api/questions` — list + by id. Drafts hidden by default; `?include_drafts=true`
//! surfaces them. Returns the canonical domain types; overlays arrive via
//! `/api/i18n/{lang}/questions` and are merged client-side.

use std::sync::Arc;

use axum::{
    Json, Router,
    extract::{Path, Query, State},
    http::StatusCode,
    response::IntoResponse,
    routing::get,
};

use crate::data::Question;
use crate::http::rest::ListParams;
use crate::state::AppState;

pub fn router() -> Router<Arc<AppState>> {
    Router::new()
        .route("/questions", get(list_questions))
        .route("/questions/{id}", get(get_question))
}

async fn list_questions(
    State(state): State<Arc<AppState>>,
    Query(params): Query<ListParams>,
) -> Json<Vec<Question>> {
    let data = state.read_dataset();
    let mut out: Vec<Question> = data.questions.values().map(|e| e.item.clone()).collect();
    if params.include_drafts {
        out.extend(data.drafts.questions.values().map(|e| e.item.clone()));
    }
    out.sort_by(|a, b| a.id().cmp(b.id()));
    Json(out)
}

async fn get_question(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
    Query(params): Query<ListParams>,
) -> impl IntoResponse {
    let data = state.read_dataset();
    if let Some(entry) = data.questions.get(&id) {
        return Json(entry.item.clone()).into_response();
    }
    if params.include_drafts
        && let Some(entry) = data.drafts.questions.get(&id)
    {
        return Json(entry.item.clone()).into_response();
    }
    StatusCode::NOT_FOUND.into_response()
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
            (
                "tags/difficulty.yaml",
                "- id: difficulty:general\n  default_lang: en\n  label: General\n",
            ),
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

    const Q_PUB: &str = r#"
- id: q_pub
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

    const Q_DRAFT: &str = r#"
- id: q_draft
  status: draft
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

    #[tokio::test]
    async fn list_excludes_drafts_by_default() {
        let ds = load(&[("questions/q.yaml", &format!("{Q_PUB}\n{Q_DRAFT}"))]);
        let app = router().with_state(test_state(ds));
        let body = axum::body::to_bytes(
            app.oneshot(
                Request::builder()
                    .uri("/questions")
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
        assert_eq!(ids, vec!["q_pub"]);
    }

    #[tokio::test]
    async fn list_with_include_drafts_flag_returns_both() {
        let ds = load(&[("questions/q.yaml", &format!("{Q_PUB}\n{Q_DRAFT}"))]);
        let app = router().with_state(test_state(ds));
        let body = axum::body::to_bytes(
            app.oneshot(
                Request::builder()
                    .uri("/questions?include_drafts=true")
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
        assert_eq!(ids, vec!["q_draft", "q_pub"]);
    }

    #[tokio::test]
    async fn by_id_returns_404_when_missing() {
        let ds = load(&[("questions/q.yaml", Q_PUB)]);
        let app = router().with_state(test_state(ds));
        let response = app
            .oneshot(
                Request::builder()
                    .uri("/questions/q_missing")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::NOT_FOUND);
    }

    #[tokio::test]
    async fn by_id_with_include_drafts_finds_drafts() {
        let ds = load(&[("questions/q.yaml", Q_DRAFT)]);
        let app = router().with_state(test_state(ds));
        let response = app
            .oneshot(
                Request::builder()
                    .uri("/questions/q_draft?include_drafts=true")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        let body = axum::body::to_bytes(response.into_body(), 4096).await.unwrap();
        let parsed: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(parsed.get("id").and_then(|v| v.as_str()), Some("q_draft"));
    }
}
