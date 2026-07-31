//! `GameState` — the full in-memory truth for one room: players, scores
//! (folded from the judgment log, never a mutated counter), current question,
//! timer DEADLINE timestamps (not countdowns), phase. Must be `Clone` (each
//! broadcast subscriber gets a copy).
//!
//! `snapshot()` produces the full-truth value the room broadcasts; per-role
//! stripping happens later in `project`, not here.
//!
//! TODO: GameState, apply(Command), on_timeout, snapshot, score = fold(log).

use std::{
    collections::{BTreeMap, HashMap, HashSet},
    ops::{Deref, DerefMut},
};

use uuid::Uuid;

use crate::{
    data::{GameConfig, normalize_locale},
    game::{
        grants::{Grant, GrantSet},
        grid_quiz::GridQuizState,
        judge::Verdict,
    },
    protocol::{Command, GridQuizPhase, MediaFetchStatus},
};

#[derive(Debug, thiserror::Error)]
pub enum CommandError {
    #[error("no players in game")]
    NoPlayers,
    #[error("player not floored {0}")]
    PlayerNotFloored(String),
    #[error("point out of range")]
    PointOutOfRange,
    #[error("buzzed while floored player")]
    BuzzWhileFlooredPlayer,
    #[error("buzzed while locked out")]
    BuzzWhileLockedOut,
    #[error("picked cell while not being the picker {0}")]
    NotYourTurn(String),
    #[error("question not open (phase {0:?})")]
    WrongPhase(GridQuizPhase),
    #[error("the cell type isn't open")]
    WrongCellType,
}

#[derive(Clone, Debug)]
pub struct GameState {
    pub game_config: GameConfig,
    /// Which entry in the game chain (`game.games[..]`) is currently live.
    pub current_game_idx: usize,
    pub player_slots: PlayerSlots,
    pub mode: ModeState,
    /// Global, append-only, spans all rounds. `score = fold(judgment_log)`.
    pub judgment_log: Vec<Judgment>,
    pub seed: u64,
    pub media_status: BTreeMap<String, MediaFetchStatus>,
}

impl GameState {
    pub fn apply(&mut self, token: Token, cmd: Command) {
        if let Some(needed) = cmd.required_grant() {
            let ok = self
                .player_slots
                .grants_for(&token)
                .is_some_and(|grants| grants.contains(&needed));
            if !ok {
                tracing::info!(?token, ?needed, "command without required grant");
                return;
            }
        }

        match cmd {
            Command::SetLocale { locale } => {
                if self.try_set_locale(&token, &locale).is_none() {
                    tracing::warn!(?token, ?locale, "invalid locale change rejected");
                }
            }
            Command::Kick { player } => {
                self.player_slots.retain(|_, v| v.name != player);
            }
            Command::Grant { player, grants } => {
                let Some(token) = self.player_slots.token_for_name(&player) else {
                    tracing::info!(?player, "grant by unknown player");
                    return;
                };

                self.player_slots
                    .entry(token)
                    .and_modify(|slot| slot.grants = grants);
            }
            other => match self
                .mode
                .apply(&self.player_slots, token.clone(), other, self.seed)
            {
                Ok(effects) => effects
                    .into_iter()
                    .for_each(|effect| self.run_effect(effect)),
                Err(err) => tracing::warn!(?token, %err, "command rejected"),
            },
        }
    }

    /// Validate + apply a locale change. Returns the normalized locale on success;
    /// `None` for malformed input or an unknown token. Caller decides whether to
    /// follow up (e.g. prefetch locale-specific media).
    pub fn try_set_locale(&mut self, token: &Token, raw: &str) -> Option<String> {
        let locale = normalize_locale(raw)?;
        let slot = self.player_slots.get_mut(token)?;
        slot.locale = locale.clone();
        Some(locale)
    }

    fn run_effect(&mut self, effect: Effect) {
        match effect {
            Effect::Submit {
                player,
                question_id,
                text,
            } => {
                if let Some(idx) = self.live_judgment(&player, &question_id)
                    && self.judgment_log[idx].verdict == Verdict::Pending
                {
                    // TODO: maybe we want to allow updating the answer before judgment
                    tracing::warn!(?player, "answer while pending judgment");
                    return;
                }

                let locale = self
                    .player_slots
                    .get(&player)
                    .expect("Submit effect fires only for existing slots")
                    .locale
                    .clone();
                self.judgment_log.push(Judgment {
                    game_idx: self.current_game_idx,
                    player,
                    locale,
                    points: 0,
                    verdict: Verdict::Pending,
                    question_id,
                    submission: Some(text),
                    supersedes: None,
                });
            }
            Effect::Rule {
                target,
                question_id,
                verdict,
                points,
            } => {
                let pending_idx = self.live_judgment(&target, &question_id);
                let locale = pending_idx
                    .and_then(|idx| self.judgment_log.get(idx))
                    .map(|judgment| judgment.locale.clone())
                    .unwrap_or_else(|| {
                        self.player_slots
                            .get(&target)
                            .expect("Rule effect fires only for existing targets")
                            .locale
                            .clone()
                    });
                self.judgment_log.push(Judgment {
                    player: target,
                    locale,
                    question_id,
                    verdict,
                    points,
                    game_idx: self.current_game_idx,
                    submission: None,
                    supersedes: pending_idx,
                });
            }
        }
    }

    // TODO: rebuilds the superseded set on every call, O(n) per lookup. fine at
    // quiz-room scale. if the log grows large, cache it or store a superseded flag
    // on each entry.
    fn live_judgment(&self, player: &Token, question_id: &str) -> Option<usize> {
        let superseded: HashSet<usize> = self
            .judgment_log
            .iter()
            .filter_map(|j| j.supersedes)
            .collect();
        self.judgment_log
            .iter()
            .enumerate()
            .rev()
            .find_map(|(i, j)| {
                (j.player == *player && j.question_id == question_id && !superseded.contains(&i))
                    .then_some(i)
            })
    }
}

#[derive(Clone, Debug)]
pub struct PlayerSlots(HashMap<Token, PlayerSlot>);

impl PlayerSlots {
    pub(crate) fn new() -> Self {
        Self(HashMap::new())
    }

    pub(crate) fn grants_for(&self, token: &Token) -> Option<&GrantSet> {
        match self.get(token) {
            Some(slot) => Some(&slot.grants),
            None => None,
        }
    }

    pub(crate) fn name_for_token(&self, token: &Token) -> Option<String> {
        self.get(token).map(|slot| slot.name.clone())
    }

    pub(crate) fn token_for_name(&self, name: &str) -> Option<Token> {
        self.iter()
            .find(|(_, slot)| slot.name == name)
            .map(|(token, _)| token.clone())
    }

    pub(crate) fn is_grant(&self, token: &Token, grant: &Grant) -> bool {
        self.grants_for(token)
            .is_some_and(|grants| grants.contains(grant))
    }
}

impl Default for PlayerSlots {
    fn default() -> Self {
        Self::new()
    }
}

impl Deref for PlayerSlots {
    type Target = HashMap<Token, PlayerSlot>;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}
impl DerefMut for PlayerSlots {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct Token(pub String);

impl Token {
    pub fn generate() -> Self {
        Self(Uuid::new_v4().into())
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PlayerSlot {
    pub name: String,
    pub locale: String,
    pub connected: bool,
    pub grants: GrantSet,
}

pub(crate) enum Effect {
    Submit {
        player: Token,
        question_id: String,
        text: String,
    },
    Rule {
        target: Token,
        question_id: String,
        verdict: Verdict,
        points: i32,
    },
}

#[derive(Clone, Debug)]
pub enum ModeState {
    GridQuiz(Box<GridQuizState>),
    Linear(Box<LinearState>),
}

impl ModeState {
    fn apply(
        &mut self,
        player_slots: &PlayerSlots,
        token: Token,
        cmd: Command,
        seed: u64,
    ) -> Result<Vec<Effect>, CommandError> {
        match self {
            ModeState::GridQuiz(modestate) => modestate.apply(player_slots, token, cmd, seed),
            ModeState::Linear(modestate) => {
                let _ = modestate;
                todo!("Linear not implemented yet")
            }
        }
    }
}

/// One entry in the append-only judgment log. Revising a ruling = append a new
/// entry that supersedes the old, refold, rebroadcast.
#[derive(Clone, Debug, PartialEq)]
pub struct Judgment {
    /// Index into `game.games` — which chain entry (GameEntry) this belongs to.
    /// Equals `current_game_idx` at append time.
    pub game_idx: usize,
    pub player: Token,
    /// Normalized locale used when submission/ruling was made. Historical
    /// review must not change when player later changes language.
    pub locale: String,
    pub question_id: String,
    /// `None` for spoken answers (moderator verdict stands alone).
    pub submission: Option<String>,
    pub verdict: Verdict,
    /// Resolved award (cell value, half on steal, penalty…). The fold sums
    /// this — steal/half math can't be re-derived from a single entry, so the
    /// outcome is recorded once at append (steal logic lives in one place).
    pub points: i32,
    /// Index of the log entry this supersedes, if revising a prior ruling.
    pub supersedes: Option<usize>,
}

/// linear play state — not yet designed. Stub so `ModeState` carries both
/// variants from the start.
#[derive(Clone, Debug, Default)]
pub struct LinearState;
