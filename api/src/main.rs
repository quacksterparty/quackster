mod config;
mod data;
mod game;
mod http;
mod media;
mod protocol;
mod state;

use std::sync::Arc;
use std::{fs, process};

use axum::Router;
use dashmap::DashMap;
use tower_http::services::{ServeDir, ServeFile};
use tower_http::trace::TraceLayer;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

use crate::config::load;
use crate::http::{rest, ws};
use crate::state::AppState;

#[tokio::main]
async fn main() {
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env().unwrap_or_else(|_| {
                format!("{}=debug,tower_http=debug", env!("CARGO_CRATE_NAME")).into()
            }),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    let config = load().expect("couldn't load config");

    if config.ytdlp_enabled {
        let status = process::Command::new(&config.ytdlp_path)
            .arg("--version")
            .output();
        if let Err(e) = status {
            panic!(
                "APP_YTDLP_ENABLED is set but '{}' is not runnable: {e}",
                config.ytdlp_path
            );
        }
        fs::create_dir_all(&config.media_cache_dir).expect("failed to create media cache dir");
    }

    let data = data::load("../data").expect("failed to load data");
    if data.issues.is_empty() {
        tracing::info!(
            "dataset loaded: {} questions, {} packs, {} tags, {} games",
            data.questions.len(),
            data.packs.len(),
            data.tags.len(),
            data.games.len()
        );
    } else {
        tracing::info!(
            "dataset loaded: {} questions, {} packs, {} tags, {} games ({} issues)",
            data.questions.len(),
            data.packs.len(),
            data.tags.len(),
            data.games.len(),
            data.issues.len()
        );
        for issue in &data.issues {
            tracing::error!("{issue}");
        }
    }

    let media = Arc::new(media::MediaFetcher::from_config(&config));
    let state = Arc::new(AppState {
        config,
        data: Arc::new(data),
        rooms: DashMap::new(),
        media,
    });
    let addr = format!("{}:{}", state.config.host, state.config.port);

    let serve_dir =
        ServeDir::new("../build").not_found_service(ServeFile::new("../build/index.html"));

    let app = Router::new()
        .nest("/api", rest::router())
        .nest_service("/media", ServeDir::new("../data/media"))
        .nest_service(
            "/media-cache",
            ServeDir::new(state.config.media_cache_dir.clone()),
        )
        .merge(ws::router())
        .fallback_service(serve_dir)
        .layer(TraceLayer::new_for_http())
        .with_state(state);

    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    tracing::debug!("listening on {}", listener.local_addr().unwrap());
    axum::serve(listener, app).await.unwrap();
}
