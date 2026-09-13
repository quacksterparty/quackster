//! `/api/packs` — list + by id + write. Drafts hidden by default;
//! `?include_drafts=true` surfaces them. Writes are admin-gated and reload
//! the dataset on success.

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
use crate::data::{Dataset, GameMode, LinearSource, Pack};
use crate::http::error::ValidationError;
use crate::http::rest::{
    Dependent, ListParams, absolute_file, bad_request, conflict, delete_file,
    dependents_conflict, internal_error, not_found, read_single, reject_relpath,
    reload_dataset, write_single,
};
use crate::state::AppState;

pub fn router() -> Router<Arc<AppState>> {
    Router::new()
        .route("/packs", get(list_packs).post(create_pack))
        .route(
            "/packs/{id}",
            get(get_pack)
                .put(put_pack)
                .patch(patch_pack)
                .delete(delete_pack),
        )
}

#[derive(Deserialize)]
struct CreateBody {
    file: String,
    item: Pack,
}

async fn list_packs(
    State(state): State<Arc<AppState>>,
    Query(params): Query<ListParams>,
) -> Json<Vec<Pack>> {
    let data = state.read_dataset();
    let mut out: Vec<Pack> = data.packs.values().map(|e| e.item.clone()).collect();
    if params.include_drafts {
        out.extend(data.drafts.packs.values().map(|e| e.item.clone()));
    }
    out.sort_by(|a, b| a.id.cmp(&b.id));
    Json(out)
}

async fn get_pack(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
    Query(params): Query<ListParams>,
) -> impl IntoResponse {
    let data = state.read_dataset();
    if let Some(entry) = data.packs.get(&id) {
        return Json(entry.item.clone()).into_response();
    }
    if params.include_drafts
        && let Some(entry) = data.drafts.packs.get(&id)
    {
        return Json(entry.item.clone()).into_response();
    }
    StatusCode::NOT_FOUND.into_response()
}

async fn create_pack(
    State(state): State<Arc<AppState>>,
    Json(body): Json<CreateBody>,
) -> Result<Response, Response> {
    reject_relpath(&body.file)?;

    if let Err(report) = body.item.validate() {
        return Err(ValidationError::from(report).into_response());
    }

    let id = body.item.id.clone();

    {
        let data = state.read_dataset();
        if data.packs.contains_key(&id) || data.drafts.packs.contains_key(&id) {
            return Err(conflict("id", format!("pack id '{id}' already exists")));
        }
    }

    let file_abs = absolute_file(&state, &body.file);
    if file_abs.exists() {
        return Err(conflict("file", format!("file '{}' already exists", body.file)));
    }
    write_single(&file_abs, &body.item)?;
    reload_dataset(&state).await?;

    Ok((StatusCode::CREATED, Json(body.item)).into_response())
}

async fn put_pack(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
    Json(item): Json<Pack>,
) -> Result<Response, Response> {
    if item.id != id {
        return Err(bad_request("id", "url id and body id must match"));
    }
    if let Err(report) = item.validate() {
        return Err(ValidationError::from(report).into_response());
    }

    let file_rel = {
        let data = state.read_dataset();
        data.packs
            .get(&id)
            .or_else(|| data.drafts.packs.get(&id))
            .ok_or_else(not_found)?
            .file
            .clone()
    };
    let file_abs = absolute_file(&state, &file_rel);
    write_single(&file_abs, &item)?;
    reload_dataset(&state).await?;

    Ok((StatusCode::OK, Json(item)).into_response())
}

async fn patch_pack(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
    Json(patch): Json<serde_json::Value>,
) -> Result<Response, Response> {
    let patch_obj = patch.as_object().ok_or_else(|| bad_request("", "patch body must be a JSON object"))?;
    if patch_obj.contains_key("id") {
        return Err(bad_request("id", "id is immutable"));
    }

    let file_rel = {
        let data = state.read_dataset();
        data.packs
            .get(&id)
            .or_else(|| data.drafts.packs.get(&id))
            .ok_or_else(not_found)?
            .file
            .clone()
    };
    let file_abs = absolute_file(&state, &file_rel);

    let existing = read_single::<Pack>(&file_abs)?.ok_or_else(not_found)?;
    let mut merged_value = serde_json::to_value(&existing).map_err(|e| internal_error(e.to_string()))?;
    json_merge(&mut merged_value, patch);
    let merged: Pack =
        serde_json::from_value(merged_value).map_err(|e| bad_request("patch", e.to_string()))?;
    if let Err(report) = merged.validate() {
        return Err(ValidationError::from(report).into_response());
    }

    write_single(&file_abs, &merged)?;
    reload_dataset(&state).await?;

    Ok((StatusCode::OK, Json(merged)).into_response())
}

async fn delete_pack(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> Result<Response, Response> {
    let (file_rel, deps) = {
        let data = state.read_dataset();
        let entry = data
            .packs
            .get(&id)
            .or_else(|| data.drafts.packs.get(&id))
            .cloned()
            .ok_or_else(not_found)?;
        (entry.file, pack_dependents(&data, &id))
    };
    if !deps.is_empty() {
        return Err(dependents_conflict(deps));
    }

    let file_abs = absolute_file(&state, &file_rel);
    if !file_abs.exists() {
        return Err(not_found());
    }
    delete_file(&file_abs)?;
    reload_dataset(&state).await?;

    Ok(StatusCode::NO_CONTENT.into_response())
}

/// Games that reference this pack via `LinearSource::Pack` or a grid category
/// `pack_ref`. Filter-based inclusions are dynamic and not checked here.
fn pack_dependents(data: &Dataset, id: &str) -> Vec<Dependent> {
    let mut out = Vec::new();
    for (game_id, entry) in data.games.iter().chain(data.drafts.games.iter()) {
        let gc = &entry.item;
        let mut hit = false;
        for game in &gc.games {
            let matches = match &game.mode {
                GameMode::GridQuiz(g) => g
                    .board
                    .categories
                    .iter()
                    .any(|c| c.pack_ref.as_deref() == Some(id)),
                GameMode::Linear(l) => matches!(l.questions, LinearSource::Pack { ref pack_id } if pack_id == id),
            };
            if matches {
                hit = true;
                break;
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

    fn empty_tags() -> Vec<(&'static str, &'static str)> {
        vec![
            ("tags/audience.yaml", "[]\n"),
            ("tags/difficulty.yaml", "[]\n"),
            ("tags/format.yaml", "[]\n"),
            ("tags/region.yaml", "[]\n"),
            ("tags/subject.yaml", "[]\n"),
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

    const PACK_PUB: &str = r#"
id: pack_pub
title: Public
questions: [q_dummy]
"#;

    const PACK_DRAFT: &str = r#"
id: pack_draft
status: draft
title: Draft
questions: [q_dummy]
"#;

    const Q_DUMMY: &str = r#"
- id: q_dummy
  kind: text
  tags: []
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
        let (tmp, ds) = load(&[
            ("packs/pub.yaml", PACK_PUB),
            ("packs/draft.yaml", PACK_DRAFT),
        ]);
        let app = router().with_state(test_state(&tmp, ds));
        let body = axum::body::to_bytes(
            app.oneshot(Request::builder().uri("/packs").body(Body::empty()).unwrap())
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
        assert_eq!(ids, vec!["pack_pub"]);
    }

    #[tokio::test]
    async fn by_id_404_when_missing() {
        let (tmp, ds) = load(&[("packs/p.yaml", PACK_PUB)]);
        let app = router().with_state(test_state(&tmp, ds));
        let response = app
            .oneshot(
                Request::builder()
                    .uri("/packs/pack_nope")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::NOT_FOUND);
    }

    #[tokio::test]
    async fn post_creates_new_file_and_persists() {
        let (tmp, ds) = load(&[("questions/q.yaml", Q_DUMMY)]);
        let state = test_state(&tmp, ds);
        let app = router().with_state(state.clone());
        let body = serde_json::json!({
            "file": "packs/new.yaml",
            "item": { "id": "pack_new", "title": "New", "questions": ["q_dummy"] }
        });
        let response = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/packs")
                    .header("content-type", "application/json")
                    .body(Body::from(serde_json::to_vec(&body).unwrap()))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::CREATED);
        let data = state.read_dataset();
        assert!(data.packs.contains_key("pack_new"));
    }

    #[tokio::test]
    async fn post_rejects_id_collision() {
        let (tmp, ds) = load(&[
            ("packs/p.yaml", PACK_PUB),
            ("questions/q.yaml", Q_DUMMY),
        ]);
        let app = router().with_state(test_state(&tmp, ds));
        let body = serde_json::json!({
            "file": "packs/other.yaml",
            "item": { "id": "pack_pub", "title": "Dup", "questions": ["q_dummy"] }
        });
        let response = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/packs")
                    .header("content-type", "application/json")
                    .body(Body::from(serde_json::to_vec(&body).unwrap()))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::CONFLICT);
    }

    #[tokio::test]
    async fn put_round_trip() {
        let (tmp, ds) = load(&[
            ("packs/p.yaml", PACK_PUB),
            ("questions/q.yaml", Q_DUMMY),
        ]);
        let state = test_state(&tmp, ds);
        let app = router().with_state(state.clone());
        let body = serde_json::json!({
            "id": "pack_pub",
            "title": "Updated",
            "questions": ["q_dummy"]
        });
        let response = app
            .oneshot(
                Request::builder()
                    .method("PUT")
                    .uri("/packs/pack_pub")
                    .header("content-type", "application/json")
                    .body(Body::from(serde_json::to_vec(&body).unwrap()))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        let data = state.read_dataset();
        assert_eq!(data.packs.get("pack_pub").unwrap().item.title, "Updated");
    }

    #[tokio::test]
    async fn patch_merges_title_only() {
        let (tmp, ds) = load(&[
            ("packs/p.yaml", PACK_PUB),
            ("questions/q.yaml", Q_DUMMY),
        ]);
        let state = test_state(&tmp, ds);
        let app = router().with_state(state.clone());
        let patch = serde_json::json!({ "title": "Patched" });
        let response = app
            .oneshot(
                Request::builder()
                    .method("PATCH")
                    .uri("/packs/pack_pub")
                    .header("content-type", "application/json")
                    .body(Body::from(serde_json::to_vec(&patch).unwrap()))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        let data = state.read_dataset();
        assert_eq!(data.packs.get("pack_pub").unwrap().item.title, "Patched");
    }

    #[tokio::test]
    async fn patch_rejects_immutable_id() {
        let (tmp, ds) = load(&[("packs/p.yaml", PACK_PUB)]);
        let app = router().with_state(test_state(&tmp, ds));
        let patch = serde_json::json!({ "id": "pack_other" });
        let response = app
            .oneshot(
                Request::builder()
                    .method("PATCH")
                    .uri("/packs/pack_pub")
                    .header("content-type", "application/json")
                    .body(Body::from(serde_json::to_vec(&patch).unwrap()))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    }

    const GAME_REFS_PACK: &str = r#"
id: game_pack_user
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
        source: pack
        pack_id: pack_pub
"#;

    #[tokio::test]
    async fn delete_returns_409_with_dependents() {
        let (tmp, ds) = load(&[
            ("packs/p.yaml", PACK_PUB),
            ("questions/q.yaml", Q_DUMMY),
            ("games/g.yaml", GAME_REFS_PACK),
        ]);
        let app = router().with_state(test_state(&tmp, ds));
        let response = app
            .oneshot(
                Request::builder()
                    .method("DELETE")
                    .uri("/packs/pack_pub")
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
        assert!(ids.contains(&"game_pack_user"));
    }

    #[tokio::test]
    async fn delete_succeeds_when_no_dependents() {
        let (tmp, ds) = load(&[
            ("packs/p.yaml", PACK_PUB),
            ("questions/q.yaml", Q_DUMMY),
        ]);
        let state = test_state(&tmp, ds);
        let app = router().with_state(state.clone());
        let response = app
            .oneshot(
                Request::builder()
                    .method("DELETE")
                    .uri("/packs/pack_pub")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::NO_CONTENT);
        let data = state.read_dataset();
        assert!(!data.packs.contains_key("pack_pub"));
    }
}
