//! REST sub-router — cold, cacheable content reads under `/api/*`. One file per
//! resource. `router()` merges the per-resource routers.
//!
//! Shared write helpers live here so every resource handler avoids the same
//! `load → mutate → save → reload` boilerplate.

use std::path::{Path, PathBuf};
use std::sync::Arc;

use axum::{
    Json, Router,
    extract::State,
    http::StatusCode,
    response::{IntoResponse, Response},
};
use serde::{Deserialize, Serialize};
use serde::de::DeserializeOwned;
use serde_json::json;

use crate::data::write::{load_list_or_empty, read_yaml_file, write_yaml_file_atomic};
use crate::state::AppState;

pub mod games;
pub mod i18n;
pub mod packs;
pub mod questions;
pub mod rooms;
pub mod stats;
pub mod tags;

/// Shared query flag for list + by-id endpoints that support drafts.
/// `?include_drafts=true` surfaces the draft registry alongside published.
#[derive(Deserialize)]
pub struct ListParams {
    #[serde(default)]
    pub include_drafts: bool,
}

/// One reference to a resource that depends on the item being deleted.
#[derive(Serialize)]
pub struct Dependent {
    pub kind: &'static str,
    pub id: String,
}

#[derive(Serialize)]
pub struct DependentsError {
    pub dependents: Vec<Dependent>,
}

pub fn router() -> Router<Arc<AppState>> {
    Router::new()
        .merge(rooms::router())
        .merge(games::router())
        .merge(questions::router())
        .merge(packs::router())
        .merge(tags::router())
        .merge(i18n::router())
}

pub async fn health(State(state): State<Arc<AppState>>) -> String {
    let data = state.read_dataset();
    format!(
        "ok, with dataset: {} questions, {} packs, {} tags, {} games and {} open rooms",
        data.questions.len(),
        data.packs.len(),
        data.tags.len(),
        data.games.len(),
        state.rooms.len()
    )
}

pub fn data_dir(state: &AppState) -> PathBuf {
    PathBuf::from(&state.config.data_dir)
}

/// Sync dataset reload after every successful write. Existing rooms keep their
/// captured `Arc<Dataset>` snapshot (see `state.rs`).
pub async fn reload_dataset(state: &AppState) -> Result<usize, Response> {
    state
        .replace_dataset(data_dir(state).as_path())
        .map_err(|e| {
            tracing::error!(
                error = %e,
                data_dir = %state.config.data_dir,
                "dataset reload failed after successful write; on-disk write landed but in-memory state is stale — caller will see 500 and may retry"
            );
            internal_error(e)
        })
}

/// Reject paths that try to escape `data_dir`.
pub fn reject_relpath(file: &str) -> Result<(), Response> {
    if file.contains("..") || file.starts_with('/') {
        return Err(bad_request("file", "must be a relative path without '..'"));
    }
    Ok(())
}

pub fn absolute_file(state: &AppState, file: &str) -> PathBuf {
    data_dir(state).join(file)
}

pub fn read_list<T: DeserializeOwned>(path: &Path) -> Result<Vec<T>, Response> {
    load_list_or_empty::<T>(path).map_err(|e| internal_error(e.to_string()))
}

pub fn write_list<T: Serialize>(path: &Path, items: &Vec<T>) -> Result<(), Response> {
    write_yaml_file_atomic(path, items).map_err(|e| internal_error(e.to_string()))
}

/// Read a single-object YAML file. `None` if the file is missing or fails to
/// parse — callers treat both as "item not on disk yet".
pub fn read_single<T: DeserializeOwned>(path: &Path) -> Result<Option<T>, Response> {
    if !path.exists() {
        return Ok(None);
    }
    read_yaml_file::<T>(path)
        .map(Some)
        .map_err(|e| internal_error(e.to_string()))
}

pub fn write_single<T: Serialize>(path: &Path, item: &T) -> Result<(), Response> {
    write_yaml_file_atomic(path, item).map_err(|e| internal_error(e.to_string()))
}

pub fn delete_file(path: &Path) -> Result<(), Response> {
    std::fs::remove_file(path).map_err(|e| internal_error(e.to_string()))
}

pub fn bad_request(path: &str, message: impl Into<String>) -> Response {
    (
        StatusCode::BAD_REQUEST,
        Json(json!({ "path": path, "message": message.into() })),
    )
        .into_response()
}

pub fn conflict(path: &str, message: impl Into<String>) -> Response {
    (
        StatusCode::CONFLICT,
        Json(json!({ "path": path, "message": message.into() })),
    )
        .into_response()
}

pub fn dependents_conflict(deps: Vec<Dependent>) -> Response {
    (
        StatusCode::CONFLICT,
        Json(DependentsError { dependents: deps }),
    )
        .into_response()
}

pub fn not_found() -> Response {
    StatusCode::NOT_FOUND.into_response()
}

pub fn internal_error(message: impl Into<String>) -> Response {
    (
        StatusCode::INTERNAL_SERVER_ERROR,
        Json(json!({ "message": message.into() })),
    )
        .into_response()
}
