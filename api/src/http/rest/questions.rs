//! `/api/questions` — list + by id + write. Drafts hidden by default;
//! `?include_drafts=true` surfaces them. Writes are admin-gated via the
//! `auth` middleware and trigger a sync dataset reload on success.
//!
//! TODO(quackster-#): POST /api/questions/{id}/move with body {file: ...}.
//! Read old file, remove question from list, append to new file (creates if
//! not exists), write both. Add once editors want to tidy topics after
//! publishing.

use std::sync::Arc;

use axum::{
    Json, Router,
    extract::{Path, Query, State},
    http::StatusCode,
    response::{IntoResponse, Response},
    routing::get,
};
use garde::Validate;
use serde::Deserialize;

use crate::data::write::json_merge;
use crate::data::{Dataset, GameMode, LinearSource, Question};
use crate::http::error::ValidationError;
use crate::http::rest::{
    Dependent, ListParams, absolute_file, bad_request, conflict, dependents_conflict,
    not_found, read_list, reject_relpath, reload_dataset, write_list,
};
use crate::state::AppState;

pub fn router() -> Router<Arc<AppState>> {
    Router::new()
        .route("/questions", get(list_questions).post(create_question))
        .route(
            "/questions/{id}",
            get(get_question)
                .put(put_question)
                .patch(patch_question)
                .delete(delete_question),
        )
}

#[derive(Deserialize)]
struct CreateBody {
    file: String,
    item: Question,
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

async fn create_question(
    State(state): State<Arc<AppState>>,
    Json(body): Json<CreateBody>,
) -> Result<Response, Response> {
    reject_relpath(&body.file)?;

    if let Err(report) = body.item.validate() {
        return Err(ValidationError::from(report).into_response());
    }

    let id = body.item.id().to_owned();

    {
        let data = state.read_dataset();
        if data.questions.contains_key(&id) || data.drafts.questions.contains_key(&id) {
            return Err(conflict("id", format!("question id '{id}' already exists")));
        }
    }

    let file_abs = absolute_file(&state, &body.file);
    let mut items = read_list::<Question>(&file_abs)?;
    if items.iter().any(|q| q.id() == id) {
        return Err(conflict("id", format!("question id '{id}' already exists in file")));
    }
    items.push(body.item.clone());
    write_list(&file_abs, &items)?;
    reload_dataset(&state).await?;

    Ok((StatusCode::CREATED, Json(body.item)).into_response())
}

async fn put_question(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
    Json(item): Json<Question>,
) -> Result<Response, Response> {
    if item.id() != id {
        return Err(bad_request("id", "url id and body id must match"));
    }
    if let Err(report) = item.validate() {
        return Err(ValidationError::from(report).into_response());
    }

    let file_rel = {
        let data = state.read_dataset();
        let entry = data
            .questions
            .get(&id)
            .or_else(|| data.drafts.questions.get(&id))
            .ok_or_else(not_found)?;
        entry.file.clone()
    };
    let file_abs = absolute_file(&state, &file_rel);

    let mut items = read_list::<Question>(&file_abs)?;
    let slot = items.iter_mut().find(|q| q.id() == id).ok_or_else(not_found)?;
    *slot = item.clone();
    write_list(&file_abs, &items)?;
    reload_dataset(&state).await?;

    Ok((StatusCode::OK, Json(item)).into_response())
}

async fn patch_question(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
    Json(patch): Json<serde_json::Value>,
) -> Result<Response, Response> {
    let patch_obj = patch.as_object().ok_or_else(|| bad_request("", "patch body must be a JSON object"))?;
    if patch_obj.contains_key("id") {
        return Err(bad_request("id", "id is immutable"));
    }
    if patch_obj.contains_key("kind") {
        return Err(bad_request("kind", "kind is immutable"));
    }

    let file_rel = {
        let data = state.read_dataset();
        data.questions
            .get(&id)
            .or_else(|| data.drafts.questions.get(&id))
            .ok_or_else(not_found)?
            .file
            .clone()
    };
    let file_abs = absolute_file(&state, &file_rel);

    let mut items = read_list::<Question>(&file_abs)?;
    let existing = items.iter_mut().find(|q| q.id() == id).ok_or_else(not_found)?;

    let mut merged_value = serde_json::to_value(&*existing).map_err(|e| crate::http::rest::internal_error(e.to_string()))?;
    json_merge(&mut merged_value, patch);
    let merged: Question = serde_json::from_value(merged_value)
        .map_err(|e| crate::http::rest::bad_request("patch", e.to_string()))?;
    if let Err(report) = merged.validate() {
        return Err(ValidationError::from(report).into_response());
    }

    *existing = merged.clone();
    write_list(&file_abs, &items)?;
    reload_dataset(&state).await?;

    Ok((StatusCode::OK, Json(merged)).into_response())
}

async fn delete_question(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> Result<Response, Response> {
    let (file_rel, deps) = {
        let data = state.read_dataset();
        let entry = data
            .questions
            .get(&id)
            .or_else(|| data.drafts.questions.get(&id))
            .cloned()
            .ok_or_else(not_found)?;
        (entry.file, question_dependents(&data, &id))
    };
    if !deps.is_empty() {
        return Err(dependents_conflict(deps));
    }

    let file_abs = absolute_file(&state, &file_rel);
    let mut items = read_list::<Question>(&file_abs)?;
    let before = items.len();
    items.retain(|q| q.id() != id);
    if items.len() == before {
        return Err(not_found());
    }
    write_list(&file_abs, &items)?;
    reload_dataset(&state).await?;

    Ok(StatusCode::NO_CONTENT.into_response())
}

/// Static references to this question across packs (explicit `questions: [...]`)
/// and game boards/linear lists. Filter-based deps are dynamic and not checked
/// here.
fn question_dependents(data: &Dataset, id: &str) -> Vec<Dependent> {
    let mut out = Vec::new();

    for (pack_id, entry) in data.packs.iter().chain(data.drafts.packs.iter()) {
        let pack = &entry.item;
        if pack
            .questions
            .as_ref()
            .is_some_and(|qids| qids.iter().any(|q| q == id))
        {
            out.push(Dependent { kind: "pack", id: pack_id.clone() });
        }
    }

    for (game_id, entry) in data.games.iter().chain(data.drafts.games.iter()) {
        let gc = &entry.item;
        let mut hit = false;
        for game in &gc.games {
            match &game.mode {
                GameMode::GridQuiz(g) => {
                    if g.board.categories.iter().any(|cat| {
                        cat.question_ids
                            .as_ref()
                            .is_some_and(|cells| cells.values().any(|c| c.id() == id))
                    }) {
                        hit = true;
                        break;
                    }
                }
                GameMode::Linear(l) => {
                    if let LinearSource::Questions { question_ids } = &l.questions
                        && question_ids.iter().any(|q| q == id)
                    {
                        hit = true;
                        break;
                    }
                }
            }
        }
        if hit {
            out.push(Dependent { kind: "game", id: game_id.clone() });
        }
    }

    out
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

    fn load(extra: &[(&str, &str)]) -> (TempDir, Dataset) {
        let mut files: Vec<(&str, &str)> = tags_yaml();
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
        let (tmp, ds) = load(&[("questions/q.yaml", &format!("{Q_PUB}\n{Q_DRAFT}"))]);
        let app = router().with_state(test_state(&tmp, ds));
        let body = axum::body::to_bytes(
            app.oneshot(Request::builder().uri("/questions").body(Body::empty()).unwrap())
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
        let (tmp, ds) = load(&[("questions/q.yaml", &format!("{Q_PUB}\n{Q_DRAFT}"))]);
        let app = router().with_state(test_state(&tmp, ds));
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
        let (tmp, ds) = load(&[("questions/q.yaml", Q_PUB)]);
        let app = router().with_state(test_state(&tmp, ds));
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
        let (tmp, ds) = load(&[("questions/q.yaml", Q_DRAFT)]);
        let app = router().with_state(test_state(&tmp, ds));
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

    #[tokio::test]
    async fn put_round_trip_persists_and_reloads() {
        let (tmp, ds) = load(&[("questions/q.yaml", Q_PUB)]);
        let state = test_state(&tmp, ds);
        let app = router().with_state(state.clone());

        let body = serde_json::json!({
            "kind": "text",
            "id": "q_pub",
            "tags": ["subject:geo"],
            "content": {
                "default_lang": "en",
                "prompt": { "text": "Updated?" },
                "answer": "a",
                "variants": { "open": { "accepted": ["a"] } }
            }
        });
        let response = app
            .oneshot(
                Request::builder()
                    .method("PUT")
                    .uri("/questions/q_pub")
                    .header("content-type", "application/json")
                    .body(Body::from(serde_json::to_vec(&body).unwrap()))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);

        // Re-read the dataset after reload and verify the change landed
        let data = state.read_dataset();
        let q = &data.questions.get("q_pub").unwrap().item;
        assert_eq!(
            q.prompt().text,
            "Updated?",
            "in-memory dataset should reflect the put"
        );
    }

    #[tokio::test]
    async fn put_rejects_url_id_body_id_mismatch() {
        let (tmp, ds) = load(&[("questions/q.yaml", Q_PUB)]);
        let app = router().with_state(test_state(&tmp, ds));
        let body = serde_json::json!({
            "kind": "text",
            "id": "q_other",
            "tags": [],
            "content": {
                "default_lang": "en",
                "prompt": { "text": "?" },
                "answer": "a",
                "variants": { "open": { "accepted": ["a"] } }
            }
        });
        let response = app
            .oneshot(
                Request::builder()
                    .method("PUT")
                    .uri("/questions/q_pub")
                    .header("content-type", "application/json")
                    .body(Body::from(serde_json::to_vec(&body).unwrap()))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    }

    #[tokio::test]
    async fn patch_merges_preserving_untouched_fields() {
        let (tmp, ds) = load(&[("questions/q.yaml", Q_PUB)]);
        let state = test_state(&tmp, ds);
        let app = router().with_state(state.clone());

        let patch = serde_json::json!({
            "content": { "explanation": "Because yes." }
        });
        let response = app
            .oneshot(
                Request::builder()
                    .method("PATCH")
                    .uri("/questions/q_pub")
                    .header("content-type", "application/json")
                    .body(Body::from(serde_json::to_vec(&patch).unwrap()))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);

        let data = state.read_dataset();
        let q = &data.questions.get("q_pub").unwrap().item;
        assert_eq!(q.explanation(), Some("Because yes."));
        // prompt and answer preserved by merge
        assert_eq!(q.prompt().text, "?");
        assert_eq!(q.tags(), &["subject:geo".to_string()]);
    }

    #[tokio::test]
    async fn patch_rejects_immutable_id_field() {
        let (tmp, ds) = load(&[("questions/q.yaml", Q_PUB)]);
        let app = router().with_state(test_state(&tmp, ds));
        let patch = serde_json::json!({ "id": "q_other" });
        let response = app
            .oneshot(
                Request::builder()
                    .method("PATCH")
                    .uri("/questions/q_pub")
                    .header("content-type", "application/json")
                    .body(Body::from(serde_json::to_vec(&patch).unwrap()))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    }

    #[tokio::test]
    async fn patch_rejects_unknown_field_with_400() {
        let (tmp, ds) = load(&[("questions/q.yaml", Q_PUB)]);
        let app = router().with_state(test_state(&tmp, ds));
        let patch = serde_json::json!({ "kontent": "typo for content" });
        let response = app
            .oneshot(
                Request::builder()
                    .method("PATCH")
                    .uri("/questions/q_pub")
                    .header("content-type", "application/json")
                    .body(Body::from(serde_json::to_vec(&patch).unwrap()))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
        let items = crate::data::write::read_yaml_file::<Vec<Question>>(
            &tmp.path().join("questions/q.yaml"),
        )
        .unwrap();
        assert_eq!(items[0].prompt().text, "?");
    }

    #[tokio::test]
    async fn post_creates_new_file_and_persists() {
        let (tmp, ds) = load(&[]);
        let state = test_state(&tmp, ds);
        let app = router().with_state(state.clone());

        let body = serde_json::json!({
            "file": "questions/geography/q_new.yaml",
            "item": {
                "kind": "text",
                "id": "q_new",
                "tags": ["subject:geo"],
                "content": {
                    "default_lang": "en",
                    "prompt": { "text": "?" },
                    "answer": "a",
                    "variants": { "open": { "accepted": ["a"] } }
                }
            }
        });
        let response = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/questions")
                    .header("content-type", "application/json")
                    .body(Body::from(serde_json::to_vec(&body).unwrap()))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::CREATED);

        // File should exist on disk
        let new_file = tmp.path().join("questions/geography/q_new.yaml");
        assert!(new_file.exists(), "POST should have created the file");

        // And the dataset reloaded
        let data = state.read_dataset();
        assert!(data.questions.contains_key("q_new"));
    }

    #[tokio::test]
    async fn post_rejects_id_collision_with_409() {
        let (tmp, ds) = load(&[("questions/q.yaml", Q_PUB)]);
        let app = router().with_state(test_state(&tmp, ds));
        let body = serde_json::json!({
            "file": "questions/q_pub.yaml",
            "item": {
                "kind": "text",
                "id": "q_pub",
                "tags": ["subject:geo"],
                "content": {
                    "default_lang": "en",
                    "prompt": { "text": "?" },
                    "answer": "a",
                    "variants": { "open": { "accepted": ["a"] } }
                }
            }
        });
        let response = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/questions")
                    .header("content-type", "application/json")
                    .body(Body::from(serde_json::to_vec(&body).unwrap()))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::CONFLICT);
    }

    #[tokio::test]
    async fn post_rejects_path_traversal() {
        let (tmp, ds) = load(&[]);
        let app = router().with_state(test_state(&tmp, ds));
        let body = serde_json::json!({
            "file": "../escape.yaml",
            "item": {
                "kind": "text",
                "id": "q_evil",
                "tags": [],
                "content": {
                    "default_lang": "en",
                    "prompt": { "text": "?" },
                    "answer": "a",
                    "variants": { "open": { "accepted": ["a"] } }
                }
            }
        });
        let response = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/questions")
                    .header("content-type", "application/json")
                    .body(Body::from(serde_json::to_vec(&body).unwrap()))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    }

    const PACK_REF: &str = r#"
id: pack_ref
title: References q_pub
questions: [q_pub]
"#;

    const GAME_REF: &str = r#"
id: game_ref
title: T
description: D
games:
  - title: R1
    rules:
      buzz_policy: open_floor
      scoring_mode: first_correct
      lockout_policy: none
      steal_policy: none
      judge: auto
    mode:
      kind: linear
      questions:
        source: questions
        question_ids: [q_pub]
"#;

    #[tokio::test]
    async fn delete_returns_409_with_dependents_list() {
        let (tmp, ds) = load(&[
            ("questions/q.yaml", Q_PUB),
            ("packs/p.yaml", PACK_REF),
            ("games/g.yaml", GAME_REF),
        ]);
        let app = router().with_state(test_state(&tmp, ds));
        let response = app
            .oneshot(
                Request::builder()
                    .method("DELETE")
                    .uri("/questions/q_pub")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::CONFLICT);
        let body = axum::body::to_bytes(response.into_body(), 4096).await.unwrap();
        let parsed: serde_json::Value = serde_json::from_slice(&body).unwrap();
        let deps = parsed.get("dependents").and_then(|d| d.as_array()).unwrap();
        let ids: Vec<&str> = deps
            .iter()
            .map(|d| d.get("id").and_then(|x| x.as_str()).unwrap())
            .collect();
        assert!(ids.contains(&"pack_ref"));
        assert!(ids.contains(&"game_ref"));
    }

    #[tokio::test]
    async fn delete_succeeds_when_no_dependents() {
        let (tmp, ds) = load(&[("questions/q.yaml", Q_PUB)]);
        let state = test_state(&tmp, ds);
        let app = router().with_state(state.clone());
        let response = app
            .oneshot(
                Request::builder()
                    .method("DELETE")
                    .uri("/questions/q_pub")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::NO_CONTENT);
        let data = state.read_dataset();
        assert!(!data.questions.contains_key("q_pub"));
    }

    #[tokio::test]
    async fn existing_rooms_keep_their_dataset_after_replace() {
        // The Phase A invariant: RoomHandle.spawn captures Arc<Dataset>; a
        // replace_dataset only swaps the AppState Arc. Reproduce the snapshot
        // isolation here by capturing the dataset, replacing, and confirming
        // the snapshot still resolves the question.
        let (tmp, ds) = load(&[("questions/q.yaml", Q_PUB)]);
        let state = test_state(&tmp, ds);

        let captured = state.snapshot_dataset();
        assert!(captured.questions.contains_key("q_pub"));

        // Replace the dataset; q_pub is removed.
        let replacement = state.snapshot_dataset();
        let mut stripped = (*replacement).clone();
        stripped.questions.remove("q_pub");
        *state.data.write().unwrap() = stripped;

        // Captured snapshot still has the original
        assert!(captured.questions.contains_key("q_pub"));
        // Live dataset reflects the change
        assert!(!state.read_dataset().questions.contains_key("q_pub"));
    }
}
