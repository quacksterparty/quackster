use std::{
    collections::VecDeque,
    mem,
};

use crate::{
    game::{
        grants::Grant,
        grid_quiz::phase::{BoardStatus, BuzzOutcome, Lobby, Phase, Resolution},
        judge::Verdict,
        state::{CommandError, Effect, PlayerSlots, Token},
    },
    protocol::Command,
};

use rand::{SeedableRng, rngs::StdRng, seq::SliceRandom};

pub mod phase;

#[derive(Debug, Clone)]
pub struct GridQuizState {
    pub phase: Phase,
    /// Picking turn order. Shuffled at `StartGame`; advanced per picker policy.
    // TODO: kicked/disconnected players stay in the rotation and can become
    // active_picker, deadlocking the board once turn enforcement lands. clean
    // up on kick or skip non-eligible players when advancing
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
                let rotation = {
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

                    rotation
                };

                let Some(picker) = rotation.front().cloned() else {
                    return Err(CommandError::NoPlayers);
                };

                match mem::take(&mut self.phase) {
                    Phase::Lobby(lobby) => {
                        self.picker_rotation = rotation;
                        self.phase = Phase::BoardSelect(lobby.start(picker));
                    }
                    other => {
                        let kind = other.kind();
                        self.phase = other;
                        return Err(CommandError::WrongPhase(kind));
                    }
                };
            }
            Command::PickCell { category, point } => {
                let Phase::BoardSelect(board_select) = &self.phase else {
                    return Err(CommandError::WrongPhase(self.phase.kind()));
                };
                if board_select.active_player() != &token
                    && !player_slots.is_grant(&token, &Grant::Moderate)
                {
                    return Err(CommandError::NotYourTurn(token.0));
                }

                let raw_cell = self
                    .cells
                    .get(category)
                    .and_then(|column| column.get(point));

                let Some(Cell::Open(question_id)) = raw_cell else {
                    return Err(CommandError::WrongCellType);
                };

                let cell = CurrentCell {
                    category,
                    point,
                    question_id: question_id.clone(),
                };

                // TODO: this should respect different flooring strategies like OpenBuzz or
                // TurnBased etc.
                match mem::take(&mut self.phase) {
                    Phase::BoardSelect(board_select) => {
                        self.phase = Phase::QuestionOpen(board_select.pick(cell));
                        if let Some(t) = self.picker_rotation.pop_front() {
                            self.picker_rotation.push_back(t);
                        }
                    }
                    other => {
                        let kind = other.kind();
                        self.phase = other;
                        return Err(CommandError::WrongPhase(kind));
                    }
                };
            }
            Command::Answer { text } => {
                let Phase::QuestionOpen(question_open) = &self.phase else {
                    return Err(CommandError::WrongPhase(self.phase.kind()));
                };

                if question_open.floored_player() != Some(&token)
                    && !player_slots.is_grant(&token, &Grant::Moderate)
                {
                    return Err(CommandError::PlayerNotFloored(token.0));
                }

                // TODO: we should check for a pending judgement here before pushing, maybe we
                // could even allow or prevent updating your answer
                return Ok(vec![Effect::Submit {
                    player: token,
                    question_id: question_open.current().question_id.clone(),
                    text,
                }]);
            }
            Command::Rule { verdict } => {
                let Phase::QuestionOpen(question_open) = &mut self.phase else {
                    return Err(CommandError::WrongPhase(self.phase.kind()));
                };

                let question_id = question_open.current().question_id.clone();
                let value = *self
                    .points
                    .get(question_open.current().point)
                    .ok_or(CommandError::PointOutOfRange)?;
                let target = question_open
                    .floored_player()
                    .cloned()
                    .ok_or_else(|| CommandError::PlayerNotFloored(token.0.clone()))?;

                // TODO: if there is someone locked out the points should be halved
                // this could also work if we check if there is judgement for the question id
                let points = match verdict {
                    Verdict::Correct => value as i32,
                    Verdict::Incorrect => -(value as i32) / 2,
                    Verdict::Void | Verdict::Pending => 0,
                };

                match verdict {
                    Verdict::Correct | Verdict::Void => self.close_question()?,
                    Verdict::Incorrect => {
                        question_open.lock_out();

                        // TODO: if every eligible player disconnected this is vacuously
                        // true and closes the question, maybe guard against the empty set
                        let all_locked = player_slots
                            .iter()
                            .filter(|(player_token, slot)| {
                                slot.connected && player_slots.is_grant(&player_token, &Grant::Play)
                            })
                            .all(|(player_token, _)| question_open.is_locked_out(player_token));

                        if all_locked {
                            self.close_question()?;
                        }
                    }
                    Verdict::Pending => {}
                }

                return Ok(vec![Effect::Rule {
                    target,
                    question_id,
                    verdict,
                    points,
                }]);
            }
            Command::Buzz => {
                let Phase::QuestionOpen(question_open) = &mut self.phase else {
                    return Err(CommandError::WrongPhase(self.phase.kind()));
                };

                match question_open.buzz(token) {
                    BuzzOutcome::Success => {}
                    BuzzOutcome::OtherPlayerFloored => {
                        return Err(CommandError::BuzzWhileFlooredPlayer);
                    }
                    BuzzOutcome::LockedOut => return Err(CommandError::BuzzWhileLockedOut),
                }
            }
            Command::Next => {
                let next_picker = self
                    .picker_rotation
                    .front()
                    .cloned()
                    .ok_or(CommandError::NoPlayers)?;

                let board_status = if self.has_open_cells() {
                    BoardStatus::OpenCellsRemain { next_picker }
                } else {
                    BoardStatus::Exhausted
                };

                match mem::take(&mut self.phase) {
                    Phase::Reveal(reveal) => {
                        self.phase = match reveal.next(board_status) {
                            Resolution::BoardSelect(board_select) => {
                                Phase::BoardSelect(board_select)
                            }
                            Resolution::GameOver(game_over) => Phase::GameOver(game_over),
                        }
                    }
                    other => {
                        let kind = other.kind();
                        self.phase = other;
                        return Err(CommandError::WrongPhase(kind));
                    }
                }
            }
            Command::CloseQuestion => self.close_question()?,
            _ => todo!("other gridquiz cmds not implemented yet"),
        }

        Ok(Vec::new())
    }

    pub(crate) fn build(cells: Vec<Vec<Cell>>, points: Vec<u32>) -> Self {
        Self {
            phase: Phase::Lobby(Lobby),
            picker_rotation: VecDeque::new(),
            cells,
            points,
        }
    }

    pub(crate) fn close_question(&mut self) -> Result<(), CommandError> {
        match mem::take(&mut self.phase) {
            Phase::QuestionOpen(question_open) => {
                self.mark_used(question_open.current());
                self.phase = Phase::Reveal(question_open.close());
                Ok(())
            }
            other => {
                let kind = other.kind();
                self.phase = other;
                Err(CommandError::WrongPhase(kind))
            }
        }
    }

    pub(crate) fn mark_used(&mut self, cell: &CurrentCell) {
        self.cells[cell.category][cell.point] = Cell::Used(cell.question_id.clone());
    }

    pub(crate) fn has_open_cells(&self) -> bool {
        self.cells
            .iter()
            .any(|column| column.iter().any(|cell| matches!(cell, Cell::Open(_))))
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

/// The cell in play + its resolved question id. Question content itself lives
/// in the `Dataset`; projection looks it up by id.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CurrentCell {
    pub category: usize,
    pub point: usize,
    pub question_id: String,
}
