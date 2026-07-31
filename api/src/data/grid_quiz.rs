//! Board builder — resolve a board definition into a 2D grid of question slots.
//!
//! Explicit IDs win, then pack refs, then filters. Deterministic shuffle via
//! seeded RNG. Variant resolved per slot via `Question::resolve_variant`.

use std::collections::{HashMap, HashSet};

use rand::SeedableRng;
use rand::rngs::StdRng;
use rand::seq::IndexedRandom;

use super::query::{PackCache, query_pool, resolve_pack};
use super::types::*;

/// Resolved board: `grid[category_idx][point_idx] = Some(QuestionSlot) | None`.
pub type BoardGrid = Vec<Vec<Option<QuestionSlot>>>;

fn slot_for(ds: &Dataset, qid: &str, variant_override: Option<VariantName>) -> Option<QuestionSlot> {
    ds.questions
        .get(qid)
        .map(|e| QuestionSlot::resolve(&e.item, variant_override))
}

/// Build a resolved NxM board grid. Unresolvable slots are `None`.
///
/// `allow_youtube = false` (yt-dlp feature off) degrades instead of blocking:
/// youtube-ref questions drop out of candidate pools, explicit youtube
/// question_ids resolve to `None` (an empty cell).
pub fn build_board(ds: &Dataset, board: &Board, seed: u64, allow_youtube: bool) -> BoardGrid {
    let mut rng = StdRng::seed_from_u64(seed);
    let mut used = HashSet::new();
    let mut pack_cache: PackCache = PackCache::new();

    let diff_map: HashMap<&u32, &[String]> = board
        .difficulty_map
        .as_ref()
        .map(|dm| dm.iter().map(|(k, v)| (k, v.as_slice())).collect())
        .unwrap_or_default();

    let mut grid = Vec::new();

    for cat in &board.categories {
        let mut row = Vec::new();
        for point in &board.points {
            // 1. Explicit question_ids override
            if let Some(cell) = cat.question_ids.as_ref().and_then(|m| m.get(point)) {
                if !allow_youtube && has_youtube_media(ds, cell.id()) {
                    tracing::warn!(
                        question_id = %cell.id(),
                        "explicit board cell needs yt-dlp (disabled); leaving cell empty"
                    );
                    row.push(None);
                    continue;
                }
                let qid = cell.id().to_owned();
                used.insert(qid.clone());
                row.push(slot_for(ds, &qid, cell.variant));
                continue;
            }

            // 2. Build candidates from pack_ref + filter + difficulty_map
            let mut candidates = build_candidates(cat, point, ds, &mut pack_cache, &diff_map);
            if !allow_youtube {
                candidates.retain(|qid| !has_youtube_media(ds, qid));
            }
            let unused: Vec<&String> = candidates.iter().filter(|id| !used.contains(*id)).collect();
            let pool: Vec<&String> = if unused.is_empty() {
                candidates.iter().collect()
            } else {
                unused
            };

            if let Some(&picked) = pool.choose(&mut rng) {
                used.insert(picked.clone());
                row.push(slot_for(ds, picked, None));
            } else {
                row.push(None);
            }
        }
        grid.push(row);
    }

    grid
}

fn build_candidates(
    cat: &BoardCategory,
    point: &u32,
    ds: &Dataset,
    pack_cache: &mut PackCache,
    diff_map: &HashMap<&u32, &[String]>,
) -> Vec<String> {
    let mut candidates = Vec::new();

    if let Some(ref pack_id) = cat.pack_ref {
        candidates = resolve_pack(ds, pack_cache, pack_id);
    }

    if let Some(ref filter) = cat.filter {
        let filtered: HashSet<String> = query_pool(ds, filter).into_iter().collect();
        if candidates.is_empty() {
            candidates = filtered.into_iter().collect();
        } else {
            candidates.retain(|id| filtered.contains(id));
        }
    }

    if let Some(diff_tags) = diff_map.get(point)
        && !diff_tags.is_empty() {
            candidates.retain(|qid| {
                ds.questions
                    .get(qid)
                    .map(|e| {
                        e.item
                            .tags()
                            .iter()
                            .any(|qtag| diff_tags.iter().any(|dtag| dtag == qtag))
                    })
                    .unwrap_or(false)
            });
        }

    candidates
}

fn has_youtube_media(ds: &Dataset, qid: &str) -> bool {
    ds.questions
        .get(qid)
        .and_then(|entry| entry.item.prompt().media.as_deref())
        .is_some_and(|media| media.iter().any(|m| m.media_ref.starts_with("youtube:")))
}

/// Resolve a `LinearSource` into a slot list (variant resolved per
/// question). Pack/Filter results shuffle with the seed for reproducibility.
/// Unknown ids silently drop — caller decides whether to error.
#[allow(dead_code)] // TODO: consumed once the linear runtime lands
pub fn resolve_linear(
    ds: &Dataset,
    source: &LinearSource,
    seed: u64,
    allow_youtube: bool,
) -> Vec<QuestionSlot> {
    use rand::seq::SliceRandom;
    let mut rng = StdRng::seed_from_u64(seed);
    let mut ids: Vec<String> = match source {
        LinearSource::Questions { question_ids } => question_ids.clone(),
        LinearSource::Pack { pack_id } => {
            let mut cache = PackCache::new();
            resolve_pack(ds, &mut cache, pack_id)
        }
        LinearSource::Filter { filter } => query_pool(ds, filter),
    };
    if !allow_youtube {
        ids.retain(|qid| !has_youtube_media(ds, qid));
    }
    ids.shuffle(&mut rng);
    ids.iter().filter_map(|qid| slot_for(ds, qid, None)).collect()
}
