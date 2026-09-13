//! `/api/i18n/{lang}/{questions,packs,games,tags}` — locale overlay documents.
//!
//! GET returns the raw overlay entries merged across files for that locale.
//! PUT/PATCH/DELETE mutate a single overlay by id. Storage shape per
//! resource mirrors the canonical loader: questions + tags are list-per-file,
//! packs + games are 1-per-file.

use std::sync::Arc;

use axum::{
    Json, Router,
    extract::{Path, State},
    http::StatusCode,
    response::{IntoResponse, Response},
    routing::{get, patch},
};

use crate::data::write::json_merge;
use crate::data::{
    GameConfigOverlay, PackOverlay, QuestionOverlay, TagOverlay, TAG_CATEGORIES,
};
use crate::http::rest::{
    absolute_file, bad_request, data_dir, delete_file, internal_error, not_found, read_list,
    read_single, reload_dataset, write_list, write_single,
};
use crate::state::AppState;

pub fn router() -> Router<Arc<AppState>> {
    Router::new()
        .route("/i18n/{lang}/questions", get(questions_overlay))
        .route(
            "/i18n/{lang}/questions/{id}",
            patch(patch_question_overlay)
                .put(put_questions_overlay)
                .delete(delete_question_overlay),
        )
        .route("/i18n/{lang}/packs", get(packs_overlay))
        .route(
            "/i18n/{lang}/packs/{id}",
            patch(patch_pack_overlay).put(put_pack_overlay).delete(delete_pack_overlay),
        )
        .route("/i18n/{lang}/games", get(games_overlay))
        .route(
            "/i18n/{lang}/games/{id}",
            patch(patch_game_overlay).put(put_game_overlay).delete(delete_game_overlay),
        )
        .route("/i18n/{lang}/tags", get(tags_overlay))
        .route(
            "/i18n/{lang}/tags/{id}",
            patch(patch_tag_overlay).put(put_tag_overlay).delete(delete_tag_overlay),
        )
}

async fn questions_overlay(
    State(state): State<Arc<AppState>>,
    Path(lang): Path<String>,
) -> impl IntoResponse {
    let data = state.read_dataset();
    let Some(overlays) = data.overlays.get(&lang) else {
        return StatusCode::NOT_FOUND.into_response();
    };
    let items: Vec<&QuestionOverlay> = overlays.questions.values().map(|e| &e.item).collect();
    if items.is_empty() {
        return StatusCode::NOT_FOUND.into_response();
    }
    Json(items).into_response()
}

async fn packs_overlay(
    State(state): State<Arc<AppState>>,
    Path(lang): Path<String>,
) -> impl IntoResponse {
    let data = state.read_dataset();
    let Some(overlays) = data.overlays.get(&lang) else {
        return StatusCode::NOT_FOUND.into_response();
    };
    let items: Vec<&PackOverlay> = overlays.packs.values().map(|e| &e.item).collect();
    if items.is_empty() {
        return StatusCode::NOT_FOUND.into_response();
    }
    Json(items).into_response()
}

async fn games_overlay(
    State(state): State<Arc<AppState>>,
    Path(lang): Path<String>,
) -> impl IntoResponse {
    let data = state.read_dataset();
    let Some(overlays) = data.overlays.get(&lang) else {
        return StatusCode::NOT_FOUND.into_response();
    };
    let items: Vec<&GameConfigOverlay> = overlays.games.values().map(|e| &e.item).collect();
    if items.is_empty() {
        return StatusCode::NOT_FOUND.into_response();
    }
    Json(items).into_response()
}

async fn tags_overlay(
    State(state): State<Arc<AppState>>,
    Path(lang): Path<String>,
) -> impl IntoResponse {
    let data = state.read_dataset();
    let Some(overlays) = data.overlays.get(&lang) else {
        return StatusCode::NOT_FOUND.into_response();
    };
    let items: Vec<&TagOverlay> = overlays.tags.values().map(|e| &e.item).collect();
    if items.is_empty() {
        return StatusCode::NOT_FOUND.into_response();
    }
    Json(items).into_response()
}

fn overlay_file(state: &AppState, lang: &str, resource: &str, file: &str) -> std::path::PathBuf {
    let mut path = data_dir(state);
    path.push("i18n");
    path.push(lang);
    path.push(resource);
    path.push(file);
    path
}

fn tag_category_file(state: &AppState, lang: &str, category: &str) -> std::path::PathBuf {
    overlay_file(state, lang, "tags", &format!("{category}.yaml"))
}

async fn put_questions_overlay(
    State(state): State<Arc<AppState>>,
    Path((lang, id)): Path<(String, String)>,
    Json(item): Json<QuestionOverlay>,
) -> Result<Response, Response> {
    if item.id != id {
        return Err(bad_request("id", "url id and body id must match"));
    }
    let file_rel = overlay_file_for(&state, &lang, "questions", &id)
        .ok_or_else(|| bad_request("file", "could not derive overlay file path"))?;
    let file_abs = absolute_file(&state, &file_rel);

    if let Some(parent) = file_abs.parent() {
        std::fs::create_dir_all(parent).map_err(|e| internal_error(e.to_string()))?;
    }

    let mut items = read_list::<QuestionOverlay>(&file_abs)?;
    if let Some(slot) = items.iter_mut().find(|q| q.id == id) {
        *slot = item;
    } else {
        items.push(item);
    }
    write_list(&file_abs, &items)?;
    reload_dataset(&state).await?;
    Ok(StatusCode::NO_CONTENT.into_response())
}

async fn patch_question_overlay(
    State(state): State<Arc<AppState>>,
    Path((lang, id)): Path<(String, String)>,
    Json(patch): Json<serde_json::Value>,
) -> Result<Response, Response> {
    let patch_obj = patch
        .as_object()
        .ok_or_else(|| bad_request("", "patch body must be a JSON object"))?;
    if patch_obj.contains_key("id") {
        return Err(bad_request("id", "id is immutable"));
    }
    let file_rel = overlay_file_for(&state, &lang, "questions", &id)
        .ok_or_else(|| bad_request("file", "could not derive overlay file path"))?;
    let file_abs = absolute_file(&state, &file_rel);
    let mut items = read_list::<QuestionOverlay>(&file_abs)?;
    let existing = items
        .iter_mut()
        .find(|q| q.id == id)
        .ok_or_else(not_found)?;
    let mut value = serde_json::to_value(&*existing).map_err(|e| internal_error(e.to_string()))?;
    json_merge(&mut value, patch);
    let merged: QuestionOverlay =
        serde_json::from_value(value).map_err(|e| bad_request("patch", e.to_string()))?;
    *existing = merged;
    write_list(&file_abs, &items)?;
    reload_dataset(&state).await?;
    Ok(StatusCode::NO_CONTENT.into_response())
}

async fn delete_question_overlay(
    State(state): State<Arc<AppState>>,
    Path((lang, id)): Path<(String, String)>,
) -> Result<Response, Response> {
    let file_rel = overlay_file_for(&state, &lang, "questions", &id)
        .ok_or_else(|| bad_request("file", "could not derive overlay file path"))?;
    let file_abs = absolute_file(&state, &file_rel);
    if !file_abs.exists() {
        return Err(not_found());
    }
    let mut items = read_list::<QuestionOverlay>(&file_abs)?;
    let before = items.len();
    items.retain(|q| q.id != id);
    if items.len() == before {
        return Err(not_found());
    }
    if items.is_empty() {
        delete_file(&file_abs)?;
    } else {
        write_list(&file_abs, &items)?;
    }
    reload_dataset(&state).await?;
    Ok(StatusCode::NO_CONTENT.into_response())
}

/// Look up the on-disk file path for an overlay entry in `data.overlays`.
fn overlay_file_for(
    state: &AppState,
    lang: &str,
    resource: &str,
    id: &str,
) -> Option<String> {
    let data = state.read_dataset();
    let overlays = data.overlays.get(lang)?;
    match resource {
        "questions" => overlays.questions.get(id).map(|e| e.file.clone()),
        "packs" => overlays.packs.get(id).map(|e| e.file.clone()),
        "games" => overlays.games.get(id).map(|e| e.file.clone()),
        "tags" => overlays.tags.get(id).map(|e| e.file.clone()),
        _ => None,
    }
}

async fn put_pack_overlay(
    State(state): State<Arc<AppState>>,
    Path((lang, id)): Path<(String, String)>,
    Json(item): Json<PackOverlay>,
) -> Result<Response, Response> {
    if item.id != id {
        return Err(bad_request("id", "url id and body id must match"));
    }
    let file_rel = overlay_file_for(&state, &lang, "packs", &id)
        .ok_or_else(not_found)?;
    let file_abs = absolute_file(&state, &file_rel);
    write_single(&file_abs, &item)?;
    reload_dataset(&state).await?;
    Ok(StatusCode::NO_CONTENT.into_response())
}

async fn patch_pack_overlay(
    State(state): State<Arc<AppState>>,
    Path((lang, id)): Path<(String, String)>,
    Json(patch): Json<serde_json::Value>,
) -> Result<Response, Response> {
    let patch_obj = patch
        .as_object()
        .ok_or_else(|| bad_request("", "patch body must be a JSON object"))?;
    if patch_obj.contains_key("id") {
        return Err(bad_request("id", "id is immutable"));
    }
    let file_rel = overlay_file_for(&state, &lang, "packs", &id)
        .ok_or_else(not_found)?;
    let file_abs = absolute_file(&state, &file_rel);
    let existing = read_single::<PackOverlay>(&file_abs)?.ok_or_else(not_found)?;
    let mut value = serde_json::to_value(&existing).map_err(|e| internal_error(e.to_string()))?;
    json_merge(&mut value, patch);
    let merged: PackOverlay =
        serde_json::from_value(value).map_err(|e| bad_request("patch", e.to_string()))?;
    write_single(&file_abs, &merged)?;
    reload_dataset(&state).await?;
    Ok(StatusCode::NO_CONTENT.into_response())
}

async fn delete_pack_overlay(
    State(state): State<Arc<AppState>>,
    Path((lang, id)): Path<(String, String)>,
) -> Result<Response, Response> {
    let file_rel = overlay_file_for(&state, &lang, "packs", &id)
        .ok_or_else(not_found)?;
    let file_abs = absolute_file(&state, &file_rel);
    if !file_abs.exists() {
        return Err(not_found());
    }
    delete_file(&file_abs)?;
    reload_dataset(&state).await?;
    Ok(StatusCode::NO_CONTENT.into_response())
}

async fn put_game_overlay(
    State(state): State<Arc<AppState>>,
    Path((lang, id)): Path<(String, String)>,
    Json(item): Json<GameConfigOverlay>,
) -> Result<Response, Response> {
    if item.id != id {
        return Err(bad_request("id", "url id and body id must match"));
    }
    let file_rel = overlay_file_for(&state, &lang, "games", &id)
        .ok_or_else(not_found)?;
    let file_abs = absolute_file(&state, &file_rel);
    write_single(&file_abs, &item)?;
    reload_dataset(&state).await?;
    Ok(StatusCode::NO_CONTENT.into_response())
}

async fn patch_game_overlay(
    State(state): State<Arc<AppState>>,
    Path((lang, id)): Path<(String, String)>,
    Json(patch): Json<serde_json::Value>,
) -> Result<Response, Response> {
    let patch_obj = patch
        .as_object()
        .ok_or_else(|| bad_request("", "patch body must be a JSON object"))?;
    if patch_obj.contains_key("id") {
        return Err(bad_request("id", "id is immutable"));
    }
    let file_rel = overlay_file_for(&state, &lang, "games", &id)
        .ok_or_else(not_found)?;
    let file_abs = absolute_file(&state, &file_rel);
    let existing = read_single::<GameConfigOverlay>(&file_abs)?.ok_or_else(not_found)?;
    let mut value = serde_json::to_value(&existing).map_err(|e| internal_error(e.to_string()))?;
    json_merge(&mut value, patch);
    let merged: GameConfigOverlay =
        serde_json::from_value(value).map_err(|e| bad_request("patch", e.to_string()))?;
    write_single(&file_abs, &merged)?;
    reload_dataset(&state).await?;
    Ok(StatusCode::NO_CONTENT.into_response())
}

async fn delete_game_overlay(
    State(state): State<Arc<AppState>>,
    Path((lang, id)): Path<(String, String)>,
) -> Result<Response, Response> {
    let file_rel = overlay_file_for(&state, &lang, "games", &id)
        .ok_or_else(not_found)?;
    let file_abs = absolute_file(&state, &file_rel);
    if !file_abs.exists() {
        return Err(not_found());
    }
    delete_file(&file_abs)?;
    reload_dataset(&state).await?;
    Ok(StatusCode::NO_CONTENT.into_response())
}

async fn put_tag_overlay(
    State(state): State<Arc<AppState>>,
    Path((lang, id)): Path<(String, String)>,
    Json(item): Json<TagOverlay>,
) -> Result<Response, Response> {
    if item.id != id {
        return Err(bad_request("id", "url id and body id must match"));
    }
    let category = tag_category_from_id(&id)
        .ok_or_else(|| bad_request("id", "tag id must have a category prefix"))?;
    let file_abs = tag_category_file(&state, &lang, category);
    if let Some(parent) = file_abs.parent() {
        std::fs::create_dir_all(parent).map_err(|e| internal_error(e.to_string()))?;
    }
    let mut items = read_list::<TagOverlay>(&file_abs)?;
    if let Some(slot) = items.iter_mut().find(|t| t.id == id) {
        *slot = item;
    } else {
        items.push(item);
    }
    write_list(&file_abs, &items)?;
    reload_dataset(&state).await?;
    Ok(StatusCode::NO_CONTENT.into_response())
}

async fn patch_tag_overlay(
    State(state): State<Arc<AppState>>,
    Path((lang, id)): Path<(String, String)>,
    Json(patch): Json<serde_json::Value>,
) -> Result<Response, Response> {
    let patch_obj = patch
        .as_object()
        .ok_or_else(|| bad_request("", "patch body must be a JSON object"))?;
    if patch_obj.contains_key("id") {
        return Err(bad_request("id", "id is immutable"));
    }
    let category = tag_category_from_id(&id)
        .ok_or_else(|| bad_request("id", "tag id must have a category prefix"))?;
    let file_abs = tag_category_file(&state, &lang, category);
    let mut items = read_list::<TagOverlay>(&file_abs)?;
    let existing = items
        .iter_mut()
        .find(|t| t.id == id)
        .ok_or_else(not_found)?;
    let mut value = serde_json::to_value(&*existing).map_err(|e| internal_error(e.to_string()))?;
    json_merge(&mut value, patch);
    let merged: TagOverlay =
        serde_json::from_value(value).map_err(|e| bad_request("patch", e.to_string()))?;
    *existing = merged;
    write_list(&file_abs, &items)?;
    reload_dataset(&state).await?;
    Ok(StatusCode::NO_CONTENT.into_response())
}

async fn delete_tag_overlay(
    State(state): State<Arc<AppState>>,
    Path((lang, id)): Path<(String, String)>,
) -> Result<Response, Response> {
    let category = tag_category_from_id(&id)
        .ok_or_else(|| bad_request("id", "tag id must have a category prefix"))?;
    let file_abs = tag_category_file(&state, &lang, category);
    if !file_abs.exists() {
        return Err(not_found());
    }
    let mut items = read_list::<TagOverlay>(&file_abs)?;
    let before = items.len();
    items.retain(|t| t.id != id);
    if items.len() == before {
        return Err(not_found());
    }
    if items.is_empty() {
        delete_file(&file_abs)?;
    } else {
        write_list(&file_abs, &items)?;
    }
    reload_dataset(&state).await?;
    Ok(StatusCode::NO_CONTENT.into_response())
}

fn tag_category_from_id(id: &str) -> Option<&'static str> {
    let category = id.split(':').next()?;
    TAG_CATEGORIES
        .iter()
        .copied()
        .find(|c| *c == category)
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

    fn empty_tags() -> Vec<(&'static str, &'static str)> {
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

    fn load(extra: &[(&str, &str)]) -> (TempDir, Dataset) {
        let mut files: Vec<(&str, &str)> = empty_tags();
        files.extend_from_slice(extra);
        let tmp = fixture(&files);
        let mut ds = load_dataset(tmp.path()).expect("load");
        ds.issues.extend(run_cross_file_checks(&ds));
        (tmp, ds)
    }

    fn test_state(tmp: &TempDir, ds: Dataset) -> StdArc<AppState> {
        let mut config = AppConfig::default();
        config.data_dir = tmp.path().to_string_lossy().into_owned();
        StdArc::new(AppState {
            config,
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
        let (tmp, ds) = load(&[
            ("questions/q.yaml", Q_FIXTURE),
            ("i18n/de/questions/q.yaml", OVERLAY_DE),
        ]);
        let app = router().with_state(test_state(&tmp, ds));
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
        let (tmp, ds) = load(&[("questions/q.yaml", Q_FIXTURE)]);
        let app = router().with_state(test_state(&tmp, ds));
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
        let (tmp, ds) = load(&[("i18n/de/tags/subject.yaml", TAG_OVERLAY_DE)]);
        let app = router().with_state(test_state(&tmp, ds));
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

    #[tokio::test]
    async fn put_question_overlay_replaces_existing() {
        let (tmp, ds) = load(&[
            ("questions/q.yaml", Q_FIXTURE),
            ("i18n/de/questions/q.yaml", OVERLAY_DE),
        ]);
        let state = test_state(&tmp, ds);
        let app = router().with_state(state.clone());
        let body = serde_json::json!({
            "id": "q_alpha",
            "content": { "explanation": "Neue Erklärung." }
        });
        let response = app
            .oneshot(
                Request::builder()
                    .method("PUT")
                    .uri("/i18n/de/questions/q_alpha")
                    .header("content-type", "application/json")
                    .body(Body::from(serde_json::to_vec(&body).unwrap()))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::NO_CONTENT);
        let data = state.read_dataset();
        let overlay = data.overlays.get("de").unwrap().questions.get("q_alpha").unwrap();
        let explanation = overlay.item.content.explanation.as_deref();
        assert_eq!(explanation, Some("Neue Erklärung."));
    }

    #[tokio::test]
    async fn patch_question_overlay_merges_field() {
        let (tmp, ds) = load(&[
            ("questions/q.yaml", Q_FIXTURE),
            ("i18n/de/questions/q.yaml", OVERLAY_DE),
        ]);
        let state = test_state(&tmp, ds);
        let app = router().with_state(state.clone());
        let patch = serde_json::json!({
            "content": { "answer": "Antwort" }
        });
        let response = app
            .oneshot(
                Request::builder()
                    .method("PATCH")
                    .uri("/i18n/de/questions/q_alpha")
                    .header("content-type", "application/json")
                    .body(Body::from(serde_json::to_vec(&patch).unwrap()))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::NO_CONTENT);
        let data = state.read_dataset();
        let overlay = data.overlays.get("de").unwrap().questions.get("q_alpha").unwrap();
        // Merged: explanation kept, answer added
        assert_eq!(overlay.item.content.explanation.as_deref(), Some("Eine Erklärung auf Deutsch."));
        assert_eq!(overlay.item.content.answer.as_deref(), Some("Antwort"));
    }

    #[tokio::test]
    async fn delete_question_overlay_removes_entry() {
        let (tmp, ds) = load(&[
            ("questions/q.yaml", Q_FIXTURE),
            ("i18n/de/questions/q.yaml", OVERLAY_DE),
        ]);
        let state = test_state(&tmp, ds);
        let app = router().with_state(state.clone());
        let response = app
            .oneshot(
                Request::builder()
                    .method("DELETE")
                    .uri("/i18n/de/questions/q_alpha")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::NO_CONTENT);
        let data = state.read_dataset();
        let overlays = data.overlays.get("de").unwrap();
        assert!(overlays.questions.get("q_alpha").is_none());
    }

    #[tokio::test]
    async fn put_tag_overlay_creates_new_file() {
        let (tmp, ds) = load(&[]);
        let state = test_state(&tmp, ds);
        let app = router().with_state(state.clone());
        let body = serde_json::json!({
            "id": "subject:geo",
            "label": "Geografie"
        });
        let response = app
            .oneshot(
                Request::builder()
                    .method("PUT")
                    .uri("/i18n/de/tags/subject:geo")
                    .header("content-type", "application/json")
                    .body(Body::from(serde_json::to_vec(&body).unwrap()))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::NO_CONTENT);
        let data = state.read_dataset();
        let overlay = data.overlays.get("de").unwrap().tags.get("subject:geo").unwrap();
        assert_eq!(overlay.item.label.as_deref(), Some("Geografie"));
    }

    #[tokio::test]
    async fn delete_tag_overlay_removes_entry() {
        let (tmp, ds) = load(&[("i18n/de/tags/subject.yaml", TAG_OVERLAY_DE)]);
        let state = test_state(&tmp, ds);
        let app = router().with_state(state.clone());
        let response = app
            .oneshot(
                Request::builder()
                    .method("DELETE")
                    .uri("/i18n/de/tags/subject:geo")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::NO_CONTENT);
        let data = state.read_dataset();
        let overlay_exists = data
            .overlays
            .get("de")
            .and_then(|o| o.tags.get("subject:geo"))
            .is_some();
        assert!(!overlay_exists);
    }
}