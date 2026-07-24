use std::collections::HashSet;

use crate::game::{
    grid_quiz::{CurrentCell, GridQuizPhase},
    state::Token,
};

#[derive(Default)]
pub enum Phase {
    #[default]
    Poisoned,
    Lobby(Lobby),
    BoardSelect(BoardSelect),
    QuestionOpen(QuestionOpen),
    Reveal(Reveal),
    GameOver(GameOver),
}

impl Phase {
    pub fn kind(&self) -> GridQuizPhase {
        match self {
            Phase::Lobby(_) => GridQuizPhase::Lobby,
            Phase::BoardSelect(_) => GridQuizPhase::BoardSelect,
            Phase::Reveal(_) => GridQuizPhase::Reveal,
            Phase::QuestionOpen(_) => GridQuizPhase::QuestionOpen,
            Phase::GameOver(_) => GridQuizPhase::GameOver,
            Phase::Poisoned => GridQuizPhase::GameOver,
        }
    }
}

pub struct Lobby;

impl Lobby {
    pub fn start(self, picker: Token) -> BoardSelect {
        BoardSelect {
            active_player: picker,
        }
    }
}

pub struct BoardSelect {
    active_player: Token,
}

impl BoardSelect {
    pub fn active_player(&self) -> &Token {
        &self.active_player
    }

    pub fn pick(self, cell: CurrentCell) -> QuestionOpen {
        QuestionOpen {
            current: cell,
            // TODO: when flooring strategies land this needs to be changed
            floored_player: Some(self.active_player),
            locked_out: HashSet::new(),
        }
    }
}

pub struct QuestionOpen {
    current: CurrentCell,
    floored_player: Option<Token>,
    locked_out: HashSet<Token>,
}

impl QuestionOpen {
    pub fn buzz(&mut self, player: Token) -> BuzzOutcome {
        if self.floored_player.is_some() {
            return BuzzOutcome::OtherPlayerFloored;
        }
        if self.locked_out.contains(&player) {
            return BuzzOutcome::LockedOut;
        }

        self.floored_player = Some(player);
        BuzzOutcome::Success
    }

    pub fn clear_floor(&mut self) {
        self.floored_player = None;
    }

    pub fn lock_out(&mut self) {
        if let Some(player) = self.floored_player.take() {
            self.locked_out.insert(player);
        }
    }

    pub fn close(self) -> Reveal {
        Reveal {
            current: self.current,
        }
    }

    pub fn current(&self) -> &CurrentCell {
        &self.current
    }

    pub fn floored_player(&self) -> Option<&Token> {
        self.floored_player.as_ref()
    }

    pub fn is_locked_out(&self, player: &Token) -> bool {
        self.locked_out.contains(player)
    }
}

pub enum BuzzOutcome {
    Success,
    OtherPlayerFloored,
    LockedOut,
}

pub struct Reveal {
    current: CurrentCell,
}

impl Reveal {
    pub fn next(self, board_status: BoardStatus) -> Resolution {
        match board_status {
            BoardStatus::OpenCellsRemain { next_picker } => Resolution::BoardSelect(BoardSelect {
                active_player: next_picker,
            }),
            BoardStatus::Exhausted => Resolution::GameOver(GameOver),
        }
    }
}

pub struct GameOver;

pub enum BoardStatus {
    OpenCellsRemain { next_picker: Token },
    Exhausted,
}

pub enum Resolution {
    BoardSelect(BoardSelect),
    GameOver(GameOver),
}
