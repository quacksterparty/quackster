use std::collections::BTreeSet;
use std::sync::Arc;

use axum::{
    Json, Router,
    extract::State,
    http::{HeaderMap, header},
    response::IntoResponse,
    routing::get,
};
use serde::{Deserialize, Serialize};

use crate::data::{
    BoardCategory, Dataset, GameConfig, GameConfigOverlay, GameMode, LinearSource, PackFilter,
    Question, Registry, TagOverlay,
};
use crate::http::locale::preferred_locale;
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

/// Tag id + label. `id` is the stable `category:slug`; `label` is localized.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
#[cfg_attr(test, derive(ts_rs::TS))]
#[cfg_attr(test, ts(export, export_to = "Games.ts"))]
struct TagDto {
    id: String,
    label: String,
}

pub fn router() -> Router<Arc<AppState>> {
    Router::new().route("/games", get(list_games))
}

async fn list_games(State(state): State<Arc<AppState>>, headers: HeaderMap) -> impl IntoResponse {
    let locale = preferred_locale(&headers, |locale| {
        state
            .data
            .overlays_for(locale)
            .iter()
            .any(|overlays| !overlays.games.is_empty())
    });
    let (game_overlays, tag_overlays) = collect_overlays(&state.data, locale.as_deref());
    let games = state
        .data
        .games
        .iter()
        .map(|(id, entry)| build_game(id, &entry.item, &state.data, &game_overlays, &tag_overlays))
        .collect::<Vec<_>>();
    ([(header::VARY, "Accept-Language")], Json(games))
}

/// All overlays relevant to the games list, ordered by fallback chain.
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

    let mut tag_ids: BTreeSet<&str> = BTreeSet::new();
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
        .collect::<BTreeSet<_>>()
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

/// First overlay to define the field wins; canonical fallback.
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

fn entry_tag_ids(mode: &GameMode, questions: &Registry<Question>) -> BTreeSet<String> {
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

fn category_tag_ids(cat: &BoardCategory, questions: &Registry<Question>) -> BTreeSet<String> {
    let mut set: BTreeSet<String> = BTreeSet::new();
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

fn linear_tag_ids(g: &crate::data::LinearGame, questions: &Registry<Question>) -> BTreeSet<String> {
    let mut set: BTreeSet<String> = BTreeSet::new();
    if let LinearSource::Questions { question_ids } = &g.questions {
        for qid in question_ids {
            if let Some(entry) = questions.get(qid) {
                set.extend(entry.item.tags().iter().cloned());
            }
        }
    }
    set
}

fn filter_tag_ids(filter: &PackFilter) -> BTreeSet<String> {
    let mut set: BTreeSet<String> = BTreeSet::new();
    if let Some(tags) = &filter.tags_all {
        set.extend(tags.iter().cloned());
    }
    if let Some(tags) = &filter.tags_any {
        set.extend(tags.iter().cloned());
    }
    set
}

/// `points × categories` for grid; explicit-list length for linear `Questions`;
/// `None` for linear `Pack`/`Filter` (would need pack/filter resolution).
fn entry_question_count(game: &crate::data::Game) -> Option<u32> {
    match &game.mode {
        GameMode::GridQuiz(g) => {
            let n = (g.board.points.len() as u32).saturating_mul(g.board.categories.len() as u32);
            Some(n)
        }
        GameMode::Linear(g) => match &g.questions {
            LinearSource::Questions { question_ids } => Some(question_ids.len() as u32),
            // TODO: pack and filter should have a limit/count
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    use crate::data::{Dataset, load_dataset, run_cross_file_checks};

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

    fn load(files: &[(&str, &str)]) -> Dataset {
        let mut ds = load_dataset(fixture(files).path()).expect("load_dataset");
        ds.issues.extend(run_cross_file_checks(&ds));
        ds
    }

    fn valid_tags() -> Vec<(&'static str, &'static str)> {
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

    fn with_registries<'a>(extra: &'a [(&'a str, &'a str)]) -> Vec<(&'a str, &'a str)> {
        let mut files = valid_tags();
        files.extend_from_slice(extra);
        files
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

    #[test]
    fn build_game_unions_modes_subjects_and_sums_counts() {
        let ds = load(&with_registries(&[
            ("questions/one.yaml", Q_ONE),
            ("questions/two.yaml", Q_TWO),
            ("games/g.yaml", GRID_4_QUESTIONS_2_TAGS),
        ]));
        let gc = ds.games.get("game_g4").unwrap();
        let g = build_game("game_g4", &gc.item, &ds, &[], &[]);

        assert_eq!(g.id, "game_g4");
        assert!(!g.auto_advance);
        assert_eq!(g.modes, vec!["grid_quiz".to_string()]);
        // 2 points × 2 categories = 4 cells.
        assert_eq!(g.question_count, Some(4));
        assert_eq!(g.entries.len(), 1);
        assert_eq!(g.entries[0].question_count, Some(4));
    }

    #[test]
    fn tags_come_from_filter_and_explicit_question_tags() {
        let ds = load(&with_registries(&[
            ("questions/one.yaml", Q_ONE),
            ("questions/two.yaml", Q_TWO),
            ("games/g.yaml", GRID_4_QUESTIONS_2_TAGS),
        ]));
        let gc = ds.games.get("game_g4").unwrap();
        let g = build_game("game_g4", &gc.item, &ds, &[], &[]);

        // Filter tag (geo) + q_one tags (geo, difficulty:general) + q_two tags
        // (history, difficulty:general). Sorted, deduped.
        assert_eq!(
            g.tags,
            vec![
                TagDto {
                    id: "difficulty:general".to_string(),
                    label: "General".to_string(),
                },
                TagDto {
                    id: "subject:geo".to_string(),
                    label: "Geography".to_string(),
                },
                TagDto {
                    id: "subject:history".to_string(),
                    label: "History".to_string(),
                },
            ]
        );
        assert_eq!(g.entries[0].tags, g.tags);
    }

    #[test]
    fn linear_pack_source_yields_none_question_count() {
        let ds = load(&with_registries(&[("games/lin.yaml", LINEAR_PACK)]));
        let gc = ds.games.get("game_lin_pack").unwrap();
        let g = build_game("game_lin_pack", &gc.item, &ds, &[], &[]);
        assert_eq!(g.entries[0].question_count, None);
        assert_eq!(g.question_count, None);
        assert!(g.entries[0].tags.is_empty());
    }

    #[test]
    fn linear_explicit_questions_have_known_count_and_tag_union() {
        let ds = load(&with_registries(&[
            ("questions/one.yaml", Q_ONE),
            ("questions/two.yaml", Q_TWO),
            ("games/lin.yaml", LINEAR_EXPLICIT),
        ]));
        let gc = ds.games.get("game_lin_explicit").unwrap();
        let g = build_game("game_lin_explicit", &gc.item, &ds, &[], &[]);
        assert_eq!(g.entries[0].question_count, Some(2));
        assert_eq!(g.question_count, Some(2));
        assert_eq!(
            g.tags,
            vec![
                TagDto {
                    id: "difficulty:general".to_string(),
                    label: "General".to_string(),
                },
                TagDto {
                    id: "subject:geo".to_string(),
                    label: "Geography".to_string(),
                },
                TagDto {
                    id: "subject:history".to_string(),
                    label: "History".to_string(),
                },
            ]
        );
    }

    #[test]
    fn accept_language_localizes_games_with_fallback_and_quality() {
        let ds = load(&with_registries(&[
            ("questions/one.yaml", Q_ONE),
            ("questions/two.yaml", Q_TWO),
            ("games/g.yaml", GRID_4_QUESTIONS_2_TAGS),
            (
                "i18n/de/games/g.yaml",
                "id: game_g4\ntitle: Deutsch\ndescription: Beschreibung\ngames:\n  - title: Runde 1\n",
            ),
        ]));
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
        let ds = load(&with_registries(&[(
            "games/g.yaml",
            GRID_4_QUESTIONS_2_TAGS,
        )]));
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
}
