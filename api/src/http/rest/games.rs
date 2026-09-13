//! `/api/games` — list + by id + write. Drafts hidden by default;
//! `?include_drafts=true` surfaces them. Writes are admin-gated and reload
//! the dataset on success. Games are stored 1-per-file.

use std::sync::Arc;

use axum::{
    Json, Router,
    extract::{Path, Query, State},
    http::{HeaderMap, StatusCode, header},
    response::{IntoResponse, Response},
    routing::get,
};
use garde::Validate;
use serde::{Deserialize, Serialize};

use crate::data::write::json_merge;
use crate::data::{
    BoardCategory, Dataset, GameConfig, GameConfigOverlay, GameMode, LinearSource, PackFilter,
    Question, Registry, TagOverlay,
};
use crate::http::error::ValidationError;
use crate::http::locale::preferred_locale;
use crate::http::rest::{
    ListParams, absolute_file, bad_request, conflict, internal_error, not_found,
    read_single, reject_relpath, reload_dataset, write_single,
};
use crate::state::AppState;

#[derive(Serialize, Deserialize)]
#[cfg_attr(test, derive(ts_rs::TS))]
#[cfg_attr(test, ts(export, export_to = "Games.ts"))]
struct Game {
    id: String,
    title: String,
    description: String,
    auto_advance: bool,
    modes: Vec<String>,
    tags: Vec<TagDto>,
    question_count: Option<u32>,
    entries: Vec<GameEntry>,
}

#[derive(Serialize, Deserialize)]
#[cfg_attr(test, derive(ts_rs::TS))]
#[cfg_attr(test, ts(export, export_to = "Games.ts"))]
struct GameEntry {
    title: String,
    mode: String,
    question_count: Option<u32>,
    tags: Vec<TagDto>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
#[cfg_attr(test, derive(ts_rs::TS))]
#[cfg_attr(test, ts(export, export_to = "Games.ts"))]
struct TagDto {
    id: String,
    label: String,
}

#[derive(Deserialize)]
struct CreateBody {
    file: String,
    item: GameConfig,
}

pub fn router() -> Router<Arc<AppState>> {
    Router::new()
        .route("/games", get(list_games).post(create_game))
        .route(
            "/games/{id}",
            get(get_game)
                .put(put_game)
                .patch(patch_game)
                .delete(delete_game),
        )
}

async fn list_games(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Query(params): Query<ListParams>,
) -> impl IntoResponse {
    let data = state.read_dataset();
    let locale = preferred_locale(&headers, |locale| {
        data.overlays_for(locale)
            .iter()
            .any(|overlays| !overlays.games.is_empty())
    });
    let (game_overlays, tag_overlays) = collect_overlays(&data, locale.as_deref());
    let mut games: Vec<Game> = data
        .games
        .iter()
        .map(|(id, entry)| build_game(id, &entry.item, &data, &game_overlays, &tag_overlays))
        .collect();
    if params.include_drafts {
        games.extend(data.drafts.games.iter().map(|(id, entry)| {
            build_game(id, &entry.item, &data, &game_overlays, &tag_overlays)
        }));
    }
    ([(header::VARY, "Accept-Language")], Json(games))
}

async fn get_game(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
    Query(params): Query<ListParams>,
    headers: HeaderMap,
) -> impl IntoResponse {
    let data = state.read_dataset();
    let locale = preferred_locale(&headers, |locale| {
        data.overlays_for(locale)
            .iter()
            .any(|overlays| !overlays.games.is_empty())
    });
    let (game_overlays, tag_overlays) = collect_overlays(&data, locale.as_deref());

    if let Some(entry) = data.games.get(&id) {
        let game = build_game(&id, &entry.item, &data, &game_overlays, &tag_overlays);
        return ([(header::VARY, "Accept-Language")], Json(game)).into_response();
    }
    if params.include_drafts
        && let Some(entry) = data.drafts.games.get(&id)
    {
        let game = build_game(&id, &entry.item, &data, &game_overlays, &tag_overlays);
        return ([(header::VARY, "Accept-Language")], Json(game)).into_response();
    }
    StatusCode::NOT_FOUND.into_response()
}

fn collect_overlays<'a>(
    data: &'a Dataset,
    locale: Option<&str>,
) -> (Vec<&'a GameConfigOverlay>, Vec<&'a TagOverlay>) {
    let Some(locale) = locale else {
        return (Vec::new(), Vec::new());
    };
    let layers = data.overlays_for(locale);
    let game_overlays = layers
        .iter()
        .flat_map(|l| l.games.values().map(|e| &e.item))
        .collect();
    let tag_overlays = layers
        .iter()
        .flat_map(|l| l.tags.values().map(|e| &e.item))
        .collect();
    (game_overlays, tag_overlays)
}

fn build_game(
    id: &str,
    game_config: &GameConfig,
    data: &Dataset,
    game_overlays: &[&GameConfigOverlay],
    tag_overlays: &[&TagOverlay],
) -> Game {
    let entry_overlays: Vec<&GameConfigOverlay> = game_overlays
        .iter()
        .copied()
        .filter(|o| o.id == id)
        .collect();

    let entries: Vec<GameEntry> = game_config
        .games
        .iter()
        .enumerate()
        .map(|(index, game)| build_entry(game, data, &entry_overlays, tag_overlays, index))
        .collect();

    let mut tag_ids: std::collections::BTreeSet<&str> = std::collections::BTreeSet::new();
    for entry in &entries {
        for tag in &entry.tags {
            tag_ids.insert(tag.id.as_str());
        }
    }
    let tags: Vec<TagDto> = tag_ids
        .into_iter()
        .map(|id| resolve_tag(id, data, tag_overlays))
        .collect();

    let modes: Vec<String> = game_config
        .games
        .iter()
        .map(|g| g.mode.mode_name().to_owned())
        .collect::<std::collections::BTreeSet<_>>()
        .into_iter()
        .collect();

    Game {
        id: id.to_owned(),
        title: overlay_string(
            &entry_overlays,
            |overlay| overlay.title.as_ref(),
            &game_config.title,
        ),
        description: overlay_string(
            &entry_overlays,
            |overlay| overlay.description.as_ref(),
            &game_config.description,
        ),
        auto_advance: game_config.auto_advance,
        modes,
        tags,
        question_count: total_question_count(&game_config.games),
        entries,
    }
}

fn build_entry(
    game: &crate::data::Game,
    data: &Dataset,
    entry_overlays: &[&GameConfigOverlay],
    tag_overlays: &[&TagOverlay],
    index: usize,
) -> GameEntry {
    let tag_ids = entry_tag_ids(&game.mode, &data.questions);
    let tags = tag_ids
        .into_iter()
        .map(|id| resolve_tag(&id, data, tag_overlays))
        .collect();
    let title = entry_overlays
        .iter()
        .find_map(|o| o.games.get(index).and_then(|g| g.title.clone()))
        .unwrap_or_else(|| game.title.clone());

    GameEntry {
        title,
        mode: game.mode.mode_name().to_owned(),
        question_count: entry_question_count(game),
        tags,
    }
}

fn overlay_string<T: ToOwned + ?Sized>(
    overlays: &[&GameConfigOverlay],
    field: impl Fn(&GameConfigOverlay) -> Option<&T>,
    canonical: &T,
) -> T::Owned {
    overlays
        .iter()
        .find_map(|overlay| field(overlay).map(ToOwned::to_owned))
        .unwrap_or_else(|| canonical.to_owned())
}

fn resolve_tag(id: &str, data: &Dataset, tag_overlays: &[&TagOverlay]) -> TagDto {
    let label = tag_overlays
        .iter()
        .find(|overlay| overlay.id == id)
        .and_then(|overlay| overlay.label.clone())
        .or_else(|| data.tags.get(id).map(|entry| entry.item.label.clone()))
        .unwrap_or_else(|| id.to_owned());
    TagDto {
        id: id.to_owned(),
        label,
    }
}

fn entry_tag_ids(mode: &GameMode, questions: &Registry<Question>) -> std::collections::BTreeSet<String> {
    match mode {
        GameMode::GridQuiz(g) => g
            .board
            .categories
            .iter()
            .flat_map(|cat| category_tag_ids(cat, questions))
            .collect(),
        GameMode::Linear(g) => linear_tag_ids(g, questions),
    }
}

fn category_tag_ids(cat: &BoardCategory, questions: &Registry<Question>) -> std::collections::BTreeSet<String> {
    let mut set: std::collections::BTreeSet<String> = std::collections::BTreeSet::new();
    if let Some(filter) = &cat.filter {
        set.extend(filter_tag_ids(filter));
    }
    if let Some(qids) = &cat.question_ids {
        for cell in qids.values() {
            if let Some(entry) = questions.get(cell.id()) {
                set.extend(entry.item.tags().iter().cloned());
            }
        }
    }
    set
}

fn linear_tag_ids(g: &crate::data::LinearGame, questions: &Registry<Question>) -> std::collections::BTreeSet<String> {
    let mut set: std::collections::BTreeSet<String> = std::collections::BTreeSet::new();
    if let LinearSource::Questions { question_ids } = &g.questions {
        for qid in question_ids {
            if let Some(entry) = questions.get(qid) {
                set.extend(entry.item.tags().iter().cloned());
            }
        }
    }
    set
}

fn filter_tag_ids(filter: &PackFilter) -> std::collections::BTreeSet<String> {
    let mut set: std::collections::BTreeSet<String> = std::collections::BTreeSet::new();
    if let Some(tags) = &filter.tags_all {
        set.extend(tags.iter().cloned());
    }
    if let Some(tags) = &filter.tags_any {
        set.extend(tags.iter().cloned());
    }
    set
}

fn entry_question_count(game: &crate::data::Game) -> Option<u32> {
    match &game.mode {
        GameMode::GridQuiz(g) => {
            let n = (g.board.points.len() as u32).saturating_mul(g.board.categories.len() as u32);
            Some(n)
        }
        GameMode::Linear(g) => match &g.questions {
            LinearSource::Questions { question_ids } => Some(question_ids.len() as u32),
            LinearSource::Pack { .. } | LinearSource::Filter { .. } => None,
        },
    }
}

fn total_question_count(games: &[crate::data::Game]) -> Option<u32> {
    let mut total = 0u32;
    for g in games {
        total = total.saturating_add(entry_question_count(g)?);
    }
    Some(total)
}

async fn create_game(
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
        if data.games.contains_key(&id) || data.drafts.games.contains_key(&id) {
            return Err(conflict("id", format!("game id '{id}' already exists")));
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

async fn put_game(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
    Json(item): Json<GameConfig>,
) -> Result<Response, Response> {
    if item.id != id {
        return Err(bad_request("id", "url id and body id must match"));
    }
    if let Err(report) = item.validate() {
        return Err(ValidationError::from(report).into_response());
    }

    let file_rel = {
        let data = state.read_dataset();
        data.games
            .get(&id)
            .or_else(|| data.drafts.games.get(&id))
            .ok_or_else(not_found)?
            .file
            .clone()
    };
    let file_abs = absolute_file(&state, &file_rel);
    write_single(&file_abs, &item)?;
    reload_dataset(&state).await?;

    Ok((StatusCode::OK, Json(item)).into_response())
}

async fn patch_game(
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
        data.games
            .get(&id)
            .or_else(|| data.drafts.games.get(&id))
            .ok_or_else(not_found)?
            .file
            .clone()
    };
    let file_abs = absolute_file(&state, &file_rel);

    let existing = read_single::<GameConfig>(&file_abs)?.ok_or_else(not_found)?;
    let mut merged_value = serde_json::to_value(&existing).map_err(|e| internal_error(e.to_string()))?;
    json_merge(&mut merged_value, patch);
    let merged: GameConfig =
        serde_json::from_value(merged_value).map_err(|e| internal_error(e.to_string()))?;
    if let Err(report) = merged.validate() {
        return Err(ValidationError::from(report).into_response());
    }

    write_single(&file_abs, &merged)?;
    reload_dataset(&state).await?;

    Ok((StatusCode::OK, Json(merged)).into_response())
}

async fn delete_game(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> Result<Response, Response> {
    let file_rel = {
        let data = state.read_dataset();
        data.games
            .get(&id)
            .or_else(|| data.drafts.games.get(&id))
            .cloned()
            .ok_or_else(not_found)?
            .file
    };
    let file_abs = absolute_file(&state, &file_rel);
    if !file_abs.exists() {
        return Err(not_found());
    }
    crate::http::rest::delete_file(&file_abs)?;
    reload_dataset(&state).await?;

    Ok(StatusCode::NO_CONTENT.into_response())
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
            (
                "tags/difficulty.yaml",
                "- id: difficulty:general\n  default_lang: en\n  label: General\n",
            ),
            ("tags/format.yaml", "[]\n"),
            ("tags/region.yaml", "[]\n"),
            (
                "tags/subject.yaml",
                "- id: subject:geo\n  default_lang: en\n  label: Geography\n\
                 - id: subject:history\n  default_lang: en\n  label: History\n",
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

    const LINEAR_GAME: &str = r#"
id: game_pub
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
        question_ids: [q_dummy]
"#;

    const DRAFT_GAME: &str = r#"
id: game_draft
status: draft
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
        question_ids: [q_dummy]
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
    async fn post_creates_new_file() {
        let (tmp, ds) = load(&[("questions/q.yaml", Q_DUMMY)]);
        let state = test_state(&tmp, ds);
        let app = router().with_state(state.clone());
        let item = serde_yaml::from_str::<serde_json::Value>(LINEAR_GAME).unwrap();
        let body = serde_json::json!({
            "file": "games/new.yaml",
            "item": item
        });
        let response = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/games")
                    .header("content-type", "application/json")
                    .body(Body::from(serde_json::to_vec(&body).unwrap()))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::CREATED);
        let data = state.read_dataset();
        assert!(data.games.contains_key("game_pub"));
    }

    #[tokio::test]
    async fn post_rejects_id_collision() {
        let (tmp, ds) = load(&[
            ("games/g.yaml", LINEAR_GAME),
            ("questions/q.yaml", Q_DUMMY),
        ]);
        let app = router().with_state(test_state(&tmp, ds));
        let item = serde_yaml::from_str::<serde_json::Value>(LINEAR_GAME).unwrap();
        let body = serde_json::json!({
            "file": "games/other.yaml",
            "item": item
        });
        let response = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/games")
                    .header("content-type", "application/json")
                    .body(Body::from(serde_json::to_vec(&body).unwrap()))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::CONFLICT);
    }

    #[tokio::test]
    async fn post_rejects_file_already_exists() {
        let (tmp, ds) = load(&[
            ("games/g.yaml", LINEAR_GAME),
            ("questions/q.yaml", Q_DUMMY),
        ]);
        let app = router().with_state(test_state(&tmp, ds));
        let item = serde_yaml::from_str::<serde_json::Value>(LINEAR_GAME).unwrap();
        let body = serde_json::json!({
            "file": "games/g.yaml",
            "item": item
        });
        let response = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/games")
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
            ("games/g.yaml", LINEAR_GAME),
            ("questions/q.yaml", Q_DUMMY),
        ]);
        let state = test_state(&tmp, ds);
        let app = router().with_state(state.clone());
        let updated = serde_json::json!({
            "id": "game_pub",
            "title": "Updated",
            "description": "D",
            "auto_advance": false,
            "games": [{
                "title": "R1",
                "rules": {
                    "buzz_policy": "open_floor",
                    "scoring_mode": "first_correct",
                    "lockout_policy": "none",
                    "steal_policy": "none",
                    "judge": "auto"
                },
                "mode": {
                    "kind": "linear",
                    "questions": { "source": "questions", "question_ids": ["q_dummy"] }
                }
            }]
        });
        let response = app
            .oneshot(
                Request::builder()
                    .method("PUT")
                    .uri("/games/game_pub")
                    .header("content-type", "application/json")
                    .body(Body::from(serde_json::to_vec(&updated).unwrap()))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        let data = state.read_dataset();
        assert_eq!(data.games.get("game_pub").unwrap().item.title, "Updated");
    }

    #[tokio::test]
    async fn patch_merges_title() {
        let (tmp, ds) = load(&[
            ("games/g.yaml", LINEAR_GAME),
            ("questions/q.yaml", Q_DUMMY),
        ]);
        let state = test_state(&tmp, ds);
        let app = router().with_state(state.clone());
        let patch = serde_json::json!({ "title": "Patched" });
        let response = app
            .oneshot(
                Request::builder()
                    .method("PATCH")
                    .uri("/games/game_pub")
                    .header("content-type", "application/json")
                    .body(Body::from(serde_json::to_vec(&patch).unwrap()))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        let data = state.read_dataset();
        assert_eq!(data.games.get("game_pub").unwrap().item.title, "Patched");
    }

    #[tokio::test]
    async fn patch_rejects_immutable_id() {
        let (tmp, ds) = load(&[("games/g.yaml", LINEAR_GAME)]);
        let app = router().with_state(test_state(&tmp, ds));
        let patch = serde_json::json!({ "id": "game_other" });
        let response = app
            .oneshot(
                Request::builder()
                    .method("PATCH")
                    .uri("/games/game_pub")
                    .header("content-type", "application/json")
                    .body(Body::from(serde_json::to_vec(&patch).unwrap()))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    }

    #[tokio::test]
    async fn delete_removes_file() {
        let (tmp, ds) = load(&[
            ("games/g.yaml", LINEAR_GAME),
            ("questions/q.yaml", Q_DUMMY),
        ]);
        let state = test_state(&tmp, ds);
        let app = router().with_state(state.clone());
        let response = app
            .oneshot(
                Request::builder()
                    .method("DELETE")
                    .uri("/games/game_pub")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::NO_CONTENT);
        let data = state.read_dataset();
        assert!(!data.games.contains_key("game_pub"));
        assert!(!tmp.path().join("games/g.yaml").exists());
    }

    #[tokio::test]
    async fn list_excludes_drafts_by_default() {
        let (tmp, ds) = load(&[
            ("games/g.yaml", LINEAR_GAME),
            ("games/d.yaml", DRAFT_GAME),
            ("questions/q.yaml", Q_DUMMY),
        ]);
        let app = router().with_state(test_state(&tmp, ds));
        let body = axum::body::to_bytes(
            app.oneshot(Request::builder().uri("/games").body(Body::empty()).unwrap())
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
        assert_eq!(ids, vec!["game_pub"]);
    }

    #[test]
    fn build_game_unions_modes_subjects_and_sums_counts() {
        let (_tmp, ds) = load(&[
            ("questions/one.yaml", Q_ONE),
            ("questions/two.yaml", Q_TWO),
            ("games/g.yaml", GRID_4_QUESTIONS_2_TAGS),
        ]);
        let gc = ds.games.get("game_g4").unwrap();
        let g = build_game("game_g4", &gc.item, &ds, &[], &[]);

        assert_eq!(g.id, "game_g4");
        assert!(!g.auto_advance);
        assert_eq!(g.modes, vec!["grid_quiz".to_string()]);
        assert_eq!(g.question_count, Some(4));
        assert_eq!(g.entries.len(), 1);
        assert_eq!(g.entries[0].question_count, Some(4));
    }

    #[test]
    fn tags_come_from_filter_and_explicit_question_tags() {
        let (_tmp, ds) = load(&[
            ("questions/one.yaml", Q_ONE),
            ("questions/two.yaml", Q_TWO),
            ("games/g.yaml", GRID_4_QUESTIONS_2_TAGS),
        ]);
        let gc = ds.games.get("game_g4").unwrap();
        let g = build_game("game_g4", &gc.item, &ds, &[], &[]);

        assert_eq!(
            g.tags,
            vec![
                TagDto { id: "difficulty:general".to_string(), label: "General".to_string() },
                TagDto { id: "subject:geo".to_string(), label: "Geography".to_string() },
                TagDto { id: "subject:history".to_string(), label: "History".to_string() },
            ]
        );
        assert_eq!(g.entries[0].tags, g.tags);
    }

    #[test]
    fn linear_pack_source_yields_none_question_count() {
        let (_tmp, ds) = load(&[("games/lin.yaml", LINEAR_PACK)]);
        let gc = ds.games.get("game_lin_pack").unwrap();
        let g = build_game("game_lin_pack", &gc.item, &ds, &[], &[]);
        assert_eq!(g.entries[0].question_count, None);
        assert_eq!(g.question_count, None);
        assert!(g.entries[0].tags.is_empty());
    }

    #[test]
    fn linear_explicit_questions_have_known_count_and_tag_union() {
        let (_tmp, ds) = load(&[
            ("questions/one.yaml", Q_ONE),
            ("questions/two.yaml", Q_TWO),
            ("games/lin.yaml", LINEAR_EXPLICIT),
        ]);
        let gc = ds.games.get("game_lin_explicit").unwrap();
        let g = build_game("game_lin_explicit", &gc.item, &ds, &[], &[]);
        assert_eq!(g.entries[0].question_count, Some(2));
        assert_eq!(g.question_count, Some(2));
        assert_eq!(
            g.tags,
            vec![
                TagDto { id: "difficulty:general".to_string(), label: "General".to_string() },
                TagDto { id: "subject:geo".to_string(), label: "Geography".to_string() },
                TagDto { id: "subject:history".to_string(), label: "History".to_string() },
            ]
        );
    }

    #[test]
    fn accept_language_localizes_games_with_fallback_and_quality() {
        let (_tmp, ds) = load(&[
            ("questions/one.yaml", Q_ONE),
            ("questions/two.yaml", Q_TWO),
            ("games/g.yaml", GRID_4_QUESTIONS_2_TAGS),
            (
                "i18n/de/games/g.yaml",
                "id: game_g4\ntitle: Deutsch\ndescription: Beschreibung\ngames:\n  - title: Runde 1\n",
            ),
        ]);
        let mut headers = HeaderMap::new();
        headers.insert(
            header::ACCEPT_LANGUAGE,
            "fr;q=0.5, de-CH;q=0.9".parse().unwrap(),
        );

        let locale = preferred_locale(&headers, |locale| {
            ds.overlays_for(locale)
                .iter()
                .any(|overlays| !overlays.games.is_empty())
        });
        let (game_overlays, tag_overlays) = collect_overlays(&ds, locale.as_deref());
        let gc = ds.games.get("game_g4").unwrap();
        let game = build_game("game_g4", &gc.item, &ds, &game_overlays, &tag_overlays);

        assert_eq!(locale.as_deref(), Some("de-CH"));
        assert_eq!(game.title, "Deutsch");
        assert_eq!(game.description, "Beschreibung");
        assert_eq!(game.entries[0].title, "Runde 1");
    }

    #[test]
    fn accept_language_returns_none_when_all_unavailable() {
        let (_tmp, ds) = load(&[(
            "games/g.yaml",
            GRID_4_QUESTIONS_2_TAGS,
        )]);
        let mut headers = HeaderMap::new();
        headers.insert(
            header::ACCEPT_LANGUAGE,
            "de;q=0, invalid_locale".parse().unwrap(),
        );

        assert_eq!(
            preferred_locale(&headers, |locale| {
                ds.overlays_for(locale)
                    .iter()
                    .any(|overlays| !overlays.games.is_empty())
            }),
            None
        );
    }

    const Q_ONE: &str = r#"
- id: q_one
  kind: text
  tags: [subject:geo, difficulty:general]
  content:
    default_lang: en
    prompt: { text: "?" }
    answer: a
    variants:
      open:
        accepted: ["a"]
"#;

    const Q_TWO: &str = r#"
- id: q_two
  kind: text
  tags: [subject:history, difficulty:general]
  content:
    default_lang: en
    prompt: { text: "?" }
    answer: b
    variants:
      open:
        accepted: ["b"]
"#;

    const GRID_4_QUESTIONS_2_TAGS: &str = r#"
id: game_g4
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
      kind: grid_quiz
      board:
        points: [100, 200]
        categories:
          - name: Geo
            filter:
              tags_any: [subject:geo]
          - name: Mix
            question_ids: { 100: { id: q_one }, 200: { id: q_two } }
"#;

    const LINEAR_PACK: &str = r#"
id: game_lin_pack
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
        pack_id: pack_x
"#;

    const LINEAR_EXPLICIT: &str = r#"
id: game_lin_explicit
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
        question_ids: [q_one, q_two]
"#;
}
