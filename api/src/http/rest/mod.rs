//! REST sub-router — cold, cacheable content reads under `/api/*`. One file per
//! resource. `router()` merges the per-resource routers.

use std::sync::Arc;

use axum::{Router, extract::State};
use serde::Deserialize;

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
