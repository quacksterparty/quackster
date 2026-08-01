//! REST sub-router — cold, cacheable content reads under `/api/*`. One file per
//! resource. `router()` merges the per-resource routers.

use std::sync::Arc;

use axum::{Router, extract::State};

use crate::state::AppState;

pub mod games;
pub mod packs;
pub mod rooms;
pub mod stats;

pub fn router() -> Router<Arc<AppState>> {
    Router::new().merge(rooms::router()).merge(games::router())
}

pub async fn health(State(state): State<Arc<AppState>>) -> String {
    format!(
        "ok, with dataset: {} questions, {} packs, {} tags, {} games and {} open rooms",
        state.data.questions.len(),
        state.data.packs.len(),
        state.data.tags.len(),
        state.data.games.len(),
        state.rooms.len()
    )
}
