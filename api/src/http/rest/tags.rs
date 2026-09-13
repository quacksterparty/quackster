//! `/api/tags` — list + by id + write. Tags have no draft state and no
//! delete route; the registry is always fully published. Writes are
//! admin-gated and reload the dataset on success.

use std::sync::Arc;

use axum::{
    Json, Router,
    extract::{Path, State},
    http::StatusCode,
    response::{IntoResponse, Response},
    routing::get,
};
use serde::Deserialize;

use crate::data::write::json_merge;
use crate::data::Tag;
use crate::http::rest::{
    ListParams, absolute_file, bad_request, conflict, internal_error, not_found, read_list,
    reject_relpath, reload_dataset, write_list,
};
use crate::state::AppState;

pub fn router() -> Router<Arc<AppState>> {
    Router::new()
        .route("/tags", get(list_tags).post(create_tag))
        .route("/tags/{id}", get(get_tag).put(put_tag).patch(patch_tag))
}

#[derive(Deserialize)]
struct CreateBody {
    file: String,
    item: Tag,
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
    Query(_params): Query<ListParams>,
) -> impl IntoResponse {
    let data = state.read_dataset();
    match data.tags.get(&id) {
        Some(entry) => Json(entry.item.clone()).into_response(),
        None => StatusCode::NOT_FOUND.into_response(),
    }
}

use axum::extract::Query;

fn category_from_id(id: &str) -> Option<&str> {
    id.split(':').next()
}

async fn create_tag(
    State(state): State<Arc<AppState>>,
    Json(body): Json<CreateBody>,
) -> Result<Response, Response> {
    reject_relpath(&body.file)?;

    let id = body.item.id.clone();

    {
        let data = state.read_dataset();
        if data.tags.contains_key(&id) {
            return Err(conflict("id", format!("tag id '{id}' already exists")));
        }
    }

    let file_abs = absolute_file(&state, &body.file);
    let mut items = read_list::<Tag>(&file_abs)?;
    if items.iter().any(|t| t.id == id) {
        return Err(conflict("id", format!("tag id '{id}' already exists in file")));
    }
    items.push(body.item.clone());
    write_list(&file_abs, &items)?;
    reload_dataset(&state).await?;

    Ok((StatusCode::CREATED, Json(body.item)).into_response())
}

async fn put_tag(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
    Json(item): Json<Tag>,
) -> Result<Response, Response> {
    if item.id != id {
        return Err(bad_request("id", "url id and body id must match"));
    }
    if item.id.split(':').next() != category_from_id(&id) {
        return Err(bad_request("id", "category prefix cannot change"));
    }

    let file_rel = state
        .read_dataset()
        .tags
        .get(&id)
        .ok_or_else(not_found)?
        .file
        .clone();
    let file_abs = absolute_file(&state, &file_rel);

    let mut items = read_list::<Tag>(&file_abs)?;
    let slot = items.iter_mut().find(|t| t.id == id).ok_or_else(not_found)?;
    *slot = item.clone();
    write_list(&file_abs, &items)?;
    reload_dataset(&state).await?;

    Ok((StatusCode::OK, Json(item)).into_response())
}

async fn patch_tag(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
    Json(patch): Json<serde_json::Value>,
) -> Result<Response, Response> {
    let patch_obj = patch.as_object().ok_or_else(|| bad_request("", "patch body must be a JSON object"))?;
    if patch_obj.contains_key("id") {
        return Err(bad_request("id", "id is immutable"));
    }

    let file_rel = state
        .read_dataset()
        .tags
        .get(&id)
        .ok_or_else(not_found)?
        .file
        .clone();
    let file_abs = absolute_file(&state, &file_rel);

    let mut items = read_list::<Tag>(&file_abs)?;
    let existing = items.iter_mut().find(|t| t.id == id).ok_or_else(not_found)?;

    let mut merged_value = serde_json::to_value(&*existing).map_err(|e| internal_error(e.to_string()))?;
    json_merge(&mut merged_value, patch);
    let merged: Tag = serde_json::from_value(merged_value).map_err(|e| internal_error(e.to_string()))?;

    *existing = merged.clone();
    write_list(&file_abs, &items)?;
    reload_dataset(&state).await?;

    Ok((StatusCode::OK, Json(merged)).into_response())
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

    fn load(extra: &[(&str, &str)]) -> (TempDir, Dataset) {
        let tmp = fixture(extra);
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
        let (tmp, ds) = load(&[("tags/subject.yaml", TAG_FIXTURE)]);
        let app = router().with_state(test_state(&tmp, ds));
        let body = axum::body::to_bytes(
            app.oneshot(Request::builder().uri("/tags").body(Body::empty()).unwrap())
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
        let (tmp, ds) = load(&[("tags/subject.yaml", TAG_FIXTURE)]);
        let app = router().with_state(test_state(&tmp, ds));
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

    #[tokio::test]
    async fn post_appends_to_category_file() {
        let (tmp, ds) = load(&[("tags/subject.yaml", TAG_FIXTURE)]);
        let state = test_state(&tmp, ds);
        let app = router().with_state(state.clone());
        let body = serde_json::json!({
            "file": "tags/subject.yaml",
            "item": { "id": "subject:music", "label": "Music", "default_lang": "en" }
        });
        let response = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/tags")
                    .header("content-type", "application/json")
                    .body(Body::from(serde_json::to_vec(&body).unwrap()))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::CREATED);
        let data = state.read_dataset();
        assert!(data.tags.contains_key("subject:music"));
    }

    #[tokio::test]
    async fn post_rejects_id_collision() {
        let (tmp, ds) = load(&[("tags/subject.yaml", TAG_FIXTURE)]);
        let app = router().with_state(test_state(&tmp, ds));
        let body = serde_json::json!({
            "file": "tags/subject.yaml",
            "item": { "id": "subject:geo", "label": "Dup", "default_lang": "en" }
        });
        let response = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/tags")
                    .header("content-type", "application/json")
                    .body(Body::from(serde_json::to_vec(&body).unwrap()))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::CONFLICT);
    }

    #[tokio::test]
    async fn put_replaces_in_file() {
        let (tmp, ds) = load(&[("tags/subject.yaml", TAG_FIXTURE)]);
        let state = test_state(&tmp, ds);
        let app = router().with_state(state.clone());
        let body = serde_json::json!({
            "id": "subject:geo",
            "label": "Geography Updated",
            "default_lang": "en"
        });
        let response = app
            .oneshot(
                Request::builder()
                    .method("PUT")
                    .uri("/tags/subject:geo")
                    .header("content-type", "application/json")
                    .body(Body::from(serde_json::to_vec(&body).unwrap()))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        let data = state.read_dataset();
        assert_eq!(data.tags.get("subject:geo").unwrap().item.label, "Geography Updated");
    }

    #[tokio::test]
    async fn patch_merges_label_only() {
        let (tmp, ds) = load(&[("tags/subject.yaml", TAG_FIXTURE)]);
        let state = test_state(&tmp, ds);
        let app = router().with_state(state.clone());
        let patch = serde_json::json!({ "label": "Patched" });
        let response = app
            .oneshot(
                Request::builder()
                    .method("PATCH")
                    .uri("/tags/subject:geo")
                    .header("content-type", "application/json")
                    .body(Body::from(serde_json::to_vec(&patch).unwrap()))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        let data = state.read_dataset();
        assert_eq!(data.tags.get("subject:geo").unwrap().item.label, "Patched");
        assert_eq!(data.tags.get("subject:geo").unwrap().item.default_lang, "en");
    }

    #[tokio::test]
    async fn patch_rejects_immutable_id() {
        let (tmp, ds) = load(&[("tags/subject.yaml", TAG_FIXTURE)]);
        let app = router().with_state(test_state(&tmp, ds));
        let patch = serde_json::json!({ "id": "subject:other" });
        let response = app
            .oneshot(
                Request::builder()
                    .method("PATCH")
                    .uri("/tags/subject:geo")
                    .header("content-type", "application/json")
                    .body(Body::from(serde_json::to_vec(&patch).unwrap()))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    }

    #[tokio::test]
    async fn delete_route_is_not_registered() {
        // Spec: tags are never hard-deleted. Confirm the DELETE route doesn't exist.
        let (tmp, ds) = load(&[("tags/subject.yaml", TAG_FIXTURE)]);
        let app = router().with_state(test_state(&tmp, ds));
        let response = app
            .oneshot(
                Request::builder()
                    .method("DELETE")
                    .uri("/tags/subject:geo")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::METHOD_NOT_ALLOWED);
    }
}
