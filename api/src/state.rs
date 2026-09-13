//! Shared application state, held in an `Arc` and handed to handlers via the
//! axum `State` extractor.
//!
//! Sits ABOVE both `http` and `game`: both need it, neither owns it.
//! - `data`   — file-backed content, mutable behind a `RwLock` so writers can
//!   sync-reload after every successful write (`replace_dataset`). Handlers
//!   acquire the lock short-term and drop before any `await`.
//! - `rooms`  — the live-game registry: `DashMap<JoinCode, RoomHandle>`. The
//!   ONLY mutable shared state; DashMap gives concurrent interior
//!   mutability so no lock is written by hand.
//! - `config` — server config (host/port, creation secret).

use std::path::Path;
use std::sync::{Arc, RwLock, RwLockReadGuard};

use dashmap::DashMap;

use crate::{
    config::AppConfig,
    data::{Dataset, load_dataset, run_cross_file_checks},
    game::room::{JoinCode, RoomHandle},
    media::MediaFetcher,
};

pub struct AppState {
    pub config: AppConfig,
    pub data: Arc<RwLock<Dataset>>,
    pub rooms: DashMap<JoinCode, RoomHandle>,
    pub media: Arc<MediaFetcher>,
}

impl AppState {
    /// Borrow the dataset. Drops the guard on scope exit. Held briefly; never
    /// across `.await`.
    pub fn read_dataset(&self) -> RwLockReadGuard<'_, Dataset> {
        match self.data.read() {
            Ok(g) => g,
            Err(poisoned) => {
                tracing::error!("dataset lock poisoned; serving stale snapshot");
                poisoned.into_inner()
            }
        }
    }

    /// Owned `Arc<Dataset>` snapshot — for spawning long-lived tasks (rooms,
    /// WS connections) that must outlive any lock scope.
    pub fn snapshot_dataset(&self) -> Arc<Dataset> {
        Arc::new((*self.read_dataset()).clone())
    }

    pub fn replace_dataset(&self, data_dir: &Path) -> Result<usize, String> {
        let new_dataset =
            load_dataset(data_dir).map_err(|e| format!("reload failed: {e}"))?;
        let mut checked = new_dataset;
        checked.issues.extend(run_cross_file_checks(&checked));
        let issues = checked.issues.len();
        match self.data.write() {
            Ok(mut guard) => *guard = checked,
            Err(_) => return Err("dataset lock poisoned".into()),
        }
        Ok(issues)
    }
}
