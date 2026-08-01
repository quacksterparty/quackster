use std::collections::BTreeSet;
use std::sync::Arc;

use axum::{
    Json, Router,
    extract::State,
    http::{
        HeaderMap, header,
    },
    response::IntoResponse,
    routing::get,
};
use serde::{Deserialize, Serialize};

use crate::data::{
    BoardCategory, Dataset, GameConfig, GameConfigOverlay, GameMode, LinearSource, PackFilter,
    Question, Registry, TagOverlay, normalize_locale,
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

pub fn router() -> Router<Arc<AppState>> {
    Router::new().route("/games", get(list_games))
}

async fn list_games(State(state): State<Arc<AppState>>, headers: HeaderMap) -> impl IntoResponse {
    let locale = preferred_locale(&headers, &state.data);
    let games = state
        .data
        .games
        .iter()
        .map(|(id, entry)| build_game(id, &entry.item, &state.data, locale.as_deref()))
        .collect::<Vec<_>>();
    ([(header::VARY, "Accept-Language")], Json(games))
}

fn preferred_locale(headers: &HeaderMap, data: &Dataset) -> Option<String> {
    let mut ranges = headers
        .get(header::ACCEPT_LANGUAGE)?
        .to_str()
        .ok()?
        .split(',')
        .enumerate()
        .filter_map(|(order, item)| {
            let mut parts = item.trim().split(';');
            let locale = normalize_locale(parts.next()?.trim())?;
            let quality = parts.try_fold(1.0_f32, |_, parameter| {
                let (name, value) = parameter.trim().split_once('=')?;
                name.eq_ignore_ascii_case("q")
                    .then(|| value.parse::<f32>().ok())
                    .flatten()
                    .filter(|q| (0.0..=1.0).contains(q))
            })?;
            (quality > 0.0).then_some((quality, order, locale))
        })
        .collect::<Vec<_>>();
    ranges.sort_by(|a, b| b.0.total_cmp(&a.0).then(a.1.cmp(&b.1)));
    ranges
        .into_iter()
        .map(|(_, _, locale)| locale)
        .find(|locale| {
            data.overlays_for(locale)
                .iter()
                .any(|overlays| !overlays.games.is_empty())
        })
}

fn build_game(id: &str, gc: &GameConfig, data: &Dataset, locale: Option<&str>) -> Game {
    let overlays = locale
        .map(|locale| {
            data.overlays_for(locale)
                .into_iter()
                .filter_map(|overlays| overlays.games.get(id).map(|entry| &entry.item))
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    let tag_overlays: Vec<&TagOverlay> = locale
        .map(|locale| {
            data.overlays_for(locale)
                .into_iter()
                .flat_map(|overlays| overlays.tags.values().map(|entry| &entry.item))
                .collect()
        })
        .unwrap_or_default();
    let entries: Vec<GameEntry> = gc
        .games
        .iter()
        .enumerate()
        .map(|(index, entry)| {
            build_entry(entry, &data, &overlays, &tag_overlays, index)
        })
        .collect();

    let modes: Vec<String> = gc
        .games
        .iter()
        .map(|entry| entry.mode.mode_name().to_owned())
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect();

    let mut tags: Vec<TagDto> = entries
        .iter()
        .flat_map(|e| e.tags.iter().cloned())
        .collect();
    tags.sort_by(|a, b| a.id.cmp(&b.id));
    tags.dedup_by(|a, b| a.id == b.id);

    let question_count = if entries.iter().any(|e| e.question_count.is_none()) {
        None
    } else {
        Some(entries.iter().filter_map(|e| e.question_count).sum())
    };

    Game {
        id: id.to_owned(),
        title: overlays
            .iter()
            .find_map(|overlay| overlay.title.clone())
            .unwrap_or_else(|| gc.title.clone()),
        description: overlays
            .iter()
            .find_map(|overlay| overlay.description.clone())
            .unwrap_or_else(|| gc.description.clone()),
        auto_advance: gc.auto_advance,
        modes,
        tags,
        question_count,
        entries,
    }
}

fn build_entry(
    entry: &crate::data::Game,
    data: &Dataset,
    overlays: &[&GameConfigOverlay],
    tag_overlays: &[&TagOverlay],
    index: usize,
) -> GameEntry {
    let tag_ids = entry_tag_ids(&entry.mode, &data.questions);
    let tags = tag_ids
        .into_iter()
        .map(|id| resolve_tag(&id, data, tag_overlays))
        .collect();
    GameEntry {
        title: overlays
            .iter()
            .find_map(|overlay| overlay.games.get(index)?.title.clone())
            .unwrap_or_else(|| entry.title.clone()),
        mode: entry.mode.mode_name().to_owned(),
        question_count: entry_question_count(entry),
        tags,
    }
}

fn resolve_tag(id: &str, data: &Dataset, tag_overlays: &[&TagOverlay]) -> TagDto {
    let overlay_label = tag_overlays
        .iter()
        .find_map(|overlay| (overlay.id == id).then(|| overlay.label.clone()).flatten());
    let canonical = data
        .tags
        .values()
        .find(|entry| entry.item.id == id)
        .map(|entry| entry.item.label.clone());
    TagDto {
        id: id.to_owned(),
        label: overlay_label.or(canonical).unwrap_or_else(|| id.to_owned()),
    }
}

fn entry_tag_ids(mode: &GameMode, questions: &Registry<Question>) -> Vec<String> {
    match mode {
        GameMode::GridQuiz(g) => {
            let mut set: BTreeSet<String> = BTreeSet::new();
            for cat in &g.board.categories {
                set.extend(category_tag_ids(cat, questions));
            }
            set.into_iter().collect()
        }
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

fn linear_tag_ids(g: &crate::data::LinearGame, questions: &Registry<Question>) -> Vec<String> {
    let mut set: BTreeSet<String> = BTreeSet::new();
    if let LinearSource::Questions { question_ids } = &g.questions {
        for qid in question_ids {
            if let Some(entry) = questions.get(qid) {
                set.extend(entry.item.tags().iter().cloned());
            }
        }
    }
    set.into_iter().collect()
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

/// `points × categories` for grid; explicit-list length for linear with
/// `Questions` source; `None` for linear `Pack`/`Filter` (would need
/// pack/filter resolution — same reason subjects skip them).
// ponytail: linear Pack/Filter counts need a seed to match runtime, defer
// until quackster-26 wires the curation backend.
fn entry_question_count(entry: &crate::data::Game) -> Option<u32> {
    match &entry.mode {
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    use crate::data::{load_dataset, run_cross_file_checks, Dataset};

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
        let g = build_game("game_g4", &gc.item, &ds, None);

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
        let g = build_game("game_g4", &gc.item, &ds, None);

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
        let g = build_game("game_lin_pack", &gc.item, &ds, None);
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
        let g = build_game("game_lin_explicit", &gc.item, &ds, None);
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
        headers.insert(header::ACCEPT_LANGUAGE, "fr;q=0.5, de-CH;q=0.9".parse().unwrap());

        let locale = preferred_locale(&headers, &ds);
        let gc = ds.games.get("game_g4").unwrap();
        let game = build_game("game_g4", &gc.item, &ds, locale.as_deref());

        assert_eq!(locale.as_deref(), Some("de-CH"));
        assert_eq!(game.title, "Deutsch");
        assert_eq!(game.description, "Beschreibung");
        assert_eq!(game.entries[0].title, "Runde 1");
    }

    #[test]
    fn accept_language_uses_canonical_for_unavailable_or_disabled_locales() {
        let ds = load(&with_registries(&[("games/g.yaml", GRID_4_QUESTIONS_2_TAGS)]));
        let mut headers = HeaderMap::new();
        headers.insert(header::ACCEPT_LANGUAGE, "de;q=0, invalid_locale".parse().unwrap());

        assert_eq!(preferred_locale(&headers, &ds), None);
    }
}
