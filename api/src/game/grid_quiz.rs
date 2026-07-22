use std::collections::{HashSet, VecDeque};

use serde::{Deserialize, Serialize};

use crate::{
    game::{
        grants::Grant,
        judge::Verdict,
        state::{CommandError, Effect, PlayerSlots, Token},
    },
    protocol::Command,
};

use rand::{SeedableRng, rngs::StdRng, seq::SliceRandom};

#[derive(Clone, Debug, PartialEq)]
pub struct GridQuizState {
    pub phase: GridQuizPhase,
    /// Whose turn to choose a cell. Meaningful only while `phase == BoardSelect`.
    pub active_picker: Option<Token>,
    /// Who may answer right now. `None` = buzz open; `Some` = that player has
    /// the floor. How it relates to `active_picker` depends on the answer
    /// policy (turn-order: floored == picker on pick; open-floor: first buzz).
    // TODO: this will probably something every mode has, so maybe refactor this
    pub floored_player: Option<Token>,
    /// Answered wrong this question — barred from re-buzzing until it resets.
    pub locked_out: HashSet<Token>,
    /// Cell + question currently in play; `None` while on the board.
    pub current: Option<CurrentCell>,
    /// Picking turn order. Shuffled at `StartGame`; advanced per picker policy.
    pub picker_rotation: VecDeque<Token>,
    pub cells: Vec<Vec<Cell>>,
    pub points: Vec<u32>,
}

impl GridQuizState {
    pub(crate) fn apply(
        &mut self,
        player_slots: &PlayerSlots,
        token: Token,
        cmd: Command,
        seed: u64,
    ) -> Result<Vec<Effect>, CommandError> {
        match cmd {
            Command::StartGame => {
                let mut rotation: VecDeque<Token> = player_slots
                    .iter()
                    .filter(|(player_token, player)| {
                        player.connected
                            && player_slots
                                .grants_for(player_token)
                                .is_some_and(|grants| grants.contains(&Grant::Play))
                    })
                    .map(|(token, _)| token.clone())
                    .collect();

                let mut rng = StdRng::seed_from_u64(seed);
                rotation.make_contiguous().shuffle(&mut rng);

                if rotation.is_empty() {
                    return Err(CommandError::NoPlayers);
                }

                self.picker_rotation = rotation;

                self.active_picker = self.picker_rotation.front().cloned();

                self.phase = GridQuizPhase::BoardSelect;
            }
            Command::PickCell { category, point } => {
                self.current = self
                    .cells
                    .get(category)
                    .and_then(|column| column.get(point))
                    .and_then(|cell| match cell {
                        Cell::Open(question) => Some(CurrentCell {
                            category,
                            point,
                            question_id: question.clone(),
                        }),
                        Cell::Used(_) => {
                            tracing::warn!(category, point, "pick on used cell");
                            None
                        }
                        Cell::Empty => {
                            tracing::warn!(category, point, "pick on empty cell");
                            None
                        }
                    });

                // TODO: this should respect different flooring strategies like OpenBuzz or
                // TurnBased etc.
                self.floored_player = self.active_picker.clone();
                self.active_picker = None;
                if let Some(token) = self.picker_rotation.pop_front() {
                    self.picker_rotation.push_back(token);
                }

                self.phase = GridQuizPhase::QuestionOpen;
            }
            Command::Answer { text } => {
                if self.floored_player.as_ref() != Some(&token)
                    && !player_slots.is_grant(&token, &Grant::Moderate)
                {
                    return Err(CommandError::PlayerNotFloored(token.0));
                }

                let Some(current) = self.current.as_ref() else {
                    return Err(CommandError::NoCurrentCell);
                };

                // TODO: we should check for a pending judgement here before pushing, maybe we
                // could even allow or prevent updating your answer
                return Ok(vec![Effect::Submit {
                    player: token,
                    question_id: current.question_id.clone(),
                    text,
                }]);
            }
            Command::Rule { player, verdict } => {
                let Some(current) = self.current.as_ref() else {
                    return Err(CommandError::NoCurrentCell);
                };

                let Some(&value) = self.points.get(current.point) else {
                    return Err(CommandError::PointOutOfRange);
                };
                let Some(target) = player_slots.token_for_name(&player) else {
                    return Err(CommandError::UnknownPlayer(player));
                };

                let points = match verdict {
                    Verdict::Correct => value as i32,
                    Verdict::Incorrect => -(value as i32) / 2,
                    Verdict::Void | Verdict::Pending => 0,
                };

                match verdict {
                    Verdict::Correct | Verdict::Void => {
                        self.floored_player = None;
                        self.cells[current.category][current.point] =
                            Cell::Used(current.question_id.clone());

                        if self
                            .cells
                            .iter()
                            .any(|column| column.iter().any(|cell| matches!(cell, Cell::Open(_))))
                        {
                            self.phase = GridQuizPhase::Reveal;
                        } else {
                            self.phase = GridQuizPhase::GameOver;
                        }
                    }
                    Verdict::Incorrect => {
                        self.floored_player = None;
                        // TODO: check if all players are locked out and close question/reveal
                        // automatically maybe
                        self.locked_out.insert(target.clone());

                        let all_locked = player_slots
                            .iter()
                            .filter(|(player_token, slot)| {
                                slot.connected
                                    && player_slots
                                        .grants_for(&player_token)
                                        .is_some_and(|grants| grants.contains(&Grant::Play))
                            })
                            .all(|(player_token, _)| self.locked_out.contains(player_token));

                        if all_locked {
                            self.floored_player = None;
                            self.cells[current.category][current.point] =
                                Cell::Used(current.question_id.clone());
                            if self.cells.iter().any(|column| {
                                column.iter().any(|cell| matches!(cell, Cell::Open(_)))
                            }) {
                                self.phase = GridQuizPhase::Reveal;
                            } else {
                                self.phase = GridQuizPhase::GameOver;
                            }
                        }
                    }
                    _ => {}
                }

                return Ok(vec![Effect::Rule {
                    target,
                    question_id: current.question_id.clone(),
                    verdict,
                    points,
                }]);
            }
            Command::Buzz => {
                if self.floored_player.is_some() {
                    return Err(CommandError::BuzzWhileFlooredPlayer);
                }

                if &self.phase != &GridQuizPhase::QuestionOpen {
                    return Err(CommandError::WrongPhase(self.phase));
                }

                self.floored_player = Some(token);
            }
            Command::Next => {
                self.locked_out = HashSet::new();
                self.current = None;
                self.active_picker = self.picker_rotation.front().cloned();
                self.phase = GridQuizPhase::BoardSelect;
            }
            Command::CloseQuestion => {
                let Some(current) = self.current.as_ref() else {
                    return Err(CommandError::NoCurrentCell);
                };

                if &self.phase != &GridQuizPhase::QuestionOpen {
                    return Err(CommandError::WrongPhase(self.phase));
                }

                self.floored_player = None;
                self.cells[current.category][current.point] =
                    Cell::Used(current.question_id.clone());

                if self
                    .cells
                    .iter()
                    .any(|column| column.iter().any(|cell| matches!(cell, Cell::Open(_))))
                {
                    self.phase = GridQuizPhase::Reveal;
                } else {
                    self.phase = GridQuizPhase::GameOver;
                }
            }
            _ => todo!("other gridquiz cmds not implemented yet"),
        }
        return Ok(Vec::new());
    }

    pub(crate) fn build(cells: Vec<Vec<Cell>>, points: Vec<u32>) -> Self {
        Self {
            phase: GridQuizPhase::Lobby,
            active_picker: None,
            floored_player: None,
            locked_out: HashSet::new(),
            current: None,
            picker_rotation: VecDeque::new(),
            cells,
            points,
        }
    }

    pub(crate) fn close_current(&mut self) {
        let Some(current) = self.current.as_ref() else {
            tracing::warn!("tried to close question with no current cell");
            return;
        };

        if &self.phase != &GridQuizPhase::QuestionOpen {
            tracing::warn!("tried to close question while no question was open");
            return;
        }

        self.floored_player = None;
        self.cells[current.category][current.point] = Cell::Used(current.question_id.clone());

        if self
            .cells
            .iter()
            .any(|column| column.iter().any(|cell| matches!(cell, Cell::Open(_))))
        {
            self.phase = GridQuizPhase::Reveal;
        } else {
            self.phase = GridQuizPhase::GameOver;
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Cell {
    Open(String),
    Used(String),
    Empty,
}

impl From<Option<String>> for Cell {
    fn from(opt: Option<String>) -> Self {
        match opt {
            Some(id) => Cell::Open(id),
            None => Cell::Empty,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[cfg_attr(test, derive(ts_rs::TS))]
#[cfg_attr(test, ts(export, export_to = "Protocol.ts"))]
pub enum GridQuizPhase {
    /// Pre-`StartGame`; players joining.
    Lobby,
    /// `active_picker` chooses a cell.
    BoardSelect,
    /// Question on screen. `floored_player == None` = buzz open; `Some` =
    /// answering. The re-buzz-after-wrong loop stays in this phase.
    QuestionOpen,
    /// Correct answer + verdict shown. Human-paced beat (discussion); exits on
    /// mod `Next` or an optional auto-advance — not auto-timed by default.
    Reveal,
    /// Terminal: board exhausted or mod ended early.
    GameOver,
}

/// The cell in play + its resolved question id. Question content itself lives
/// in the `Dataset`; projection looks it up by id.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CurrentCell {
    pub category: usize,
    pub point: usize,
    pub question_id: String,
}
