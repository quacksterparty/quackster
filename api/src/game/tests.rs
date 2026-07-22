//! Game-loop tests: drive `GameState::apply` with `Command`s, assert on
//! observable outcomes (phase, folded score, cells, floor) — not internal
//! representation — so they survive the planned phase-enum refactor.
//!
//! Two tests are `#[ignore]`d: they encode intended behavior that current
//! code gets wrong (PickCell on used cell, Buzz while locked out). Un-ignore
//! them when the phase refactor lands the fixes.

use std::collections::HashSet;

use crate::{
    game::{
        grants::Grant,
        grid_quiz::{Cell, GridQuizPhase, GridQuizState},
        judge::Verdict,
        state::{GameState, ModeState, PlayerSlot, PlayerSlots, Token},
    },
    protocol::Command,
};

const GAME_YAML: &str = r#"
id: game_test
title: Test
description: Test
games:
  - title: Round 1
    rules:
      buzz_policy: open_floor
      scoring_mode: first_correct
      lockout_policy: this_question
      steal_policy: round_limited
      judge: auto
    mode:
      kind: grid_quiz
      board:
        points: [100, 200]
        categories:
          - name: A
            question_ids: { 100: q_a1, 200: q_a2 }
          - name: B
            question_ids: { 100: q_b1, 200: q_b2 }
"#;

fn token(name: &str) -> Token {
    Token(format!("tok_{name}"))
}

/// 2x2 board (points 100/200), one moderator "mod", given names as players.
fn setup(players: &[&str]) -> GameState {
    let mut slots = PlayerSlots::default();
    slots.insert(
        token("mod"),
        PlayerSlot {
            name: "mod".into(),
            connected: true,
            grants: HashSet::from([Grant::Moderate]),
        },
    );
    for p in players {
        slots.insert(
            token(p),
            PlayerSlot {
                name: (*p).into(),
                connected: true,
                grants: HashSet::from([Grant::Play]),
            },
        );
    }

    let cells = vec![
        vec![Cell::Open("q_a1".into()), Cell::Open("q_a2".into())],
        vec![Cell::Open("q_b1".into()), Cell::Open("q_b2".into())],
    ];

    GameState {
        game_config: serde_yaml::from_str(GAME_YAML).expect("fixture parses"),
        current_game_idx: 0,
        player_slots: slots,
        mode: ModeState::GridQuiz(GridQuizState::build(cells, vec![100, 200])),
        judgment_log: Vec::new(),
        seed: 42,
    }
}

fn grid(state: &GameState) -> &GridQuizState {
    match &state.mode {
        ModeState::GridQuiz(g) => g,
        other => panic!("expected grid quiz, got {other:?}"),
    }
}

/// score = fold(judgment_log), skipping superseded entries — same fold as
/// projection.
fn score(state: &GameState, name: &str) -> i32 {
    let superseded: HashSet<usize> = state
        .judgment_log
        .iter()
        .filter_map(|j| j.supersedes)
        .collect();
    state
        .judgment_log
        .iter()
        .enumerate()
        .filter(|(i, j)| !superseded.contains(i) && j.player == token(name))
        .map(|(_, j)| j.points)
        .sum()
}

/// Name of the player whose turn it is to pick.
fn picker(state: &GameState) -> String {
    let t = grid(state).active_picker.clone().expect("a picker");
    state
        .player_slots
        .name_for_token(&t)
        .expect("picker has a slot")
}

fn first_open_cell(state: &GameState) -> (usize, usize) {
    grid(state)
        .cells
        .iter()
        .enumerate()
        .find_map(|(c, column)| {
            column
                .iter()
                .position(|cell| matches!(cell, Cell::Open(_)))
                .map(|p| (c, p))
        })
        .expect("an open cell")
}

/// StartGame + PickCell as the active picker; returns picker name.
fn start_and_pick(state: &mut GameState, category: usize, point: usize) -> String {
    state.apply(token("mod"), Command::StartGame);
    let p = picker(state);
    state.apply(token(&p), Command::PickCell { category, point });
    p
}

// ── lobby / start ──

#[test]
fn start_game_requires_moderate_grant() {
    let mut state = setup(&["alice", "bob"]);
    state.apply(token("alice"), Command::StartGame);
    assert_eq!(grid(&state).phase, GridQuizPhase::Lobby);
}

#[test]
fn start_game_enters_board_select_with_a_playing_picker() {
    let mut state = setup(&["alice", "bob"]);
    state.apply(token("mod"), Command::StartGame);

    let g = grid(&state);
    assert_eq!(g.phase, GridQuizPhase::BoardSelect);
    assert_eq!(g.picker_rotation.len(), 2, "only Play-granted players rotate");
    assert!(!g.picker_rotation.contains(&token("mod")));
    assert_eq!(g.active_picker, g.picker_rotation.front().cloned());
}

#[test]
fn start_game_without_players_stays_in_lobby() {
    let mut state = setup(&[]);
    state.apply(token("mod"), Command::StartGame);
    assert_eq!(grid(&state).phase, GridQuizPhase::Lobby);
}

// ── picking ──

#[test]
fn pick_cell_opens_question_and_floors_the_picker() {
    let mut state = setup(&["alice", "bob"]);
    let p = start_and_pick(&mut state, 0, 0);

    let g = grid(&state);
    assert_eq!(g.phase, GridQuizPhase::QuestionOpen);
    assert_eq!(g.floored_player, Some(token(&p)));
    assert_eq!(g.active_picker, None);
    let current = g.current.as_ref().expect("a current cell");
    assert_eq!(current.question_id, "q_a1");
    assert_eq!(
        g.picker_rotation.front(),
        Some(&token(if p == "alice" { "bob" } else { "alice" })),
        "rotation advances on pick"
    );
}

#[test]
#[ignore = "known bug: pick on used cell still enters QuestionOpen with current=None; fix with phase enum refactor"]
fn pick_used_cell_is_rejected_without_state_change() {
    let mut state = setup(&["alice", "bob"]);
    let p = start_and_pick(&mut state, 0, 0);
    state.apply(token("mod"), Command::Rule { player: p, verdict: Verdict::Correct });
    state.apply(token("mod"), Command::Next);

    let rotation_before = grid(&state).picker_rotation.clone();
    let p2 = picker(&state);
    state.apply(token(&p2), Command::PickCell { category: 0, point: 0 });

    let g = grid(&state);
    assert_eq!(g.phase, GridQuizPhase::BoardSelect, "used cell not pickable");
    assert!(g.current.is_none());
    assert_eq!(g.picker_rotation, rotation_before, "rotation not burned");
}

// ── answering / ruling ──

#[test]
fn floored_player_answer_lands_as_pending_judgment() {
    let mut state = setup(&["alice", "bob"]);
    let p = start_and_pick(&mut state, 0, 0);
    state.apply(token(&p), Command::Answer { text: "42".into() });

    assert_eq!(state.judgment_log.len(), 1);
    let j = &state.judgment_log[0];
    assert_eq!(j.player, token(&p));
    assert_eq!(j.verdict, Verdict::Pending);
    assert_eq!(j.submission.as_deref(), Some("42"));
    assert_eq!(j.points, 0);
}

#[test]
fn non_floored_player_cannot_answer() {
    let mut state = setup(&["alice", "bob"]);
    let p = start_and_pick(&mut state, 0, 0);
    let other = if p == "alice" { "bob" } else { "alice" };

    state.apply(token(other), Command::Answer { text: "sneaky".into() });
    assert!(state.judgment_log.is_empty());
}

#[test]
fn correct_ruling_awards_cell_value_and_reveals() {
    let mut state = setup(&["alice", "bob"]);
    let p = start_and_pick(&mut state, 0, 0);
    state.apply(token(&p), Command::Answer { text: "42".into() });
    state.apply(token("mod"), Command::Rule { player: p.clone(), verdict: Verdict::Correct });

    assert_eq!(score(&state, &p), 100);
    let g = grid(&state);
    assert_eq!(g.phase, GridQuizPhase::Reveal);
    assert_eq!(g.floored_player, None);
    assert_eq!(g.cells[0][0], Cell::Used("q_a1".into()));
    assert_eq!(
        state.judgment_log[1].supersedes,
        Some(0),
        "ruling supersedes the pending submission"
    );
}

#[test]
fn incorrect_ruling_halves_penalty_and_locks_out() {
    let mut state = setup(&["alice", "bob"]);
    let p = start_and_pick(&mut state, 0, 0);
    state.apply(token("mod"), Command::Rule { player: p.clone(), verdict: Verdict::Incorrect });

    assert_eq!(score(&state, &p), -50);
    let g = grid(&state);
    assert_eq!(g.phase, GridQuizPhase::QuestionOpen, "question stays open for steal");
    assert_eq!(g.floored_player, None, "floor reopens");
    assert!(g.locked_out.contains(&token(&p)));
}

#[test]
fn revised_ruling_supersedes_and_refolds_score() {
    let mut state = setup(&["alice", "bob"]);
    let p = start_and_pick(&mut state, 0, 0);
    state.apply(token(&p), Command::Answer { text: "42".into() });
    state.apply(token("mod"), Command::Rule { player: p.clone(), verdict: Verdict::Correct });
    state.apply(token("mod"), Command::Rule { player: p.clone(), verdict: Verdict::Incorrect });

    assert_eq!(score(&state, &p), -50, "latest ruling wins, no double count");
    assert_eq!(state.judgment_log[2].supersedes, Some(1));
}

#[test]
fn all_players_locked_out_closes_the_question() {
    let mut state = setup(&["alice"]);
    let p = start_and_pick(&mut state, 0, 0);
    state.apply(token("mod"), Command::Rule { player: p, verdict: Verdict::Incorrect });

    let g = grid(&state);
    assert_eq!(g.phase, GridQuizPhase::Reveal);
    assert_eq!(g.cells[0][0], Cell::Used("q_a1".into()));
}

// ── buzzing ──

#[test]
fn buzz_takes_the_floor_when_open() {
    let mut state = setup(&["alice", "bob"]);
    let p = start_and_pick(&mut state, 0, 0);
    let other = if p == "alice" { "bob" } else { "alice" };
    state.apply(token("mod"), Command::Rule { player: p, verdict: Verdict::Incorrect });

    state.apply(token(other), Command::Buzz);
    assert_eq!(grid(&state).floored_player, Some(token(other)));
}

#[test]
fn buzz_rejected_while_floor_taken() {
    let mut state = setup(&["alice", "bob"]);
    let p = start_and_pick(&mut state, 0, 0);
    let other = if p == "alice" { "bob" } else { "alice" };

    state.apply(token(other), Command::Buzz);
    assert_eq!(grid(&state).floored_player, Some(token(&p)), "picker keeps floor");
}

#[test]
fn buzz_rejected_outside_question_open() {
    let mut state = setup(&["alice", "bob"]);
    state.apply(token("mod"), Command::StartGame);

    state.apply(token("alice"), Command::Buzz);
    let g = grid(&state);
    assert_eq!(g.phase, GridQuizPhase::BoardSelect);
    assert_eq!(g.floored_player, None);
}

#[test]
#[ignore = "known bug: Buzz ignores locked_out; fix with phase enum refactor"]
fn locked_out_player_cannot_rebuzz() {
    let mut state = setup(&["alice", "bob"]);
    let p = start_and_pick(&mut state, 0, 0);
    state.apply(token("mod"), Command::Rule { player: p.clone(), verdict: Verdict::Incorrect });

    state.apply(token(&p), Command::Buzz);
    assert_eq!(grid(&state).floored_player, None, "locked-out player barred");
}

// ── advancing / game over ──

#[test]
fn next_resets_question_state_and_returns_to_board() {
    let mut state = setup(&["alice", "bob"]);
    let p = start_and_pick(&mut state, 0, 0);
    state.apply(token("mod"), Command::Rule { player: p, verdict: Verdict::Correct });
    state.apply(token("mod"), Command::Next);

    let g = grid(&state);
    assert_eq!(g.phase, GridQuizPhase::BoardSelect);
    assert!(g.current.is_none());
    assert!(g.locked_out.is_empty());
    assert_eq!(g.active_picker, g.picker_rotation.front().cloned());
}

#[test]
fn close_question_marks_cell_used_without_scoring() {
    let mut state = setup(&["alice", "bob"]);
    start_and_pick(&mut state, 0, 0);
    state.apply(token("mod"), Command::CloseQuestion);

    let g = grid(&state);
    assert_eq!(g.phase, GridQuizPhase::Reveal);
    assert_eq!(g.cells[0][0], Cell::Used("q_a1".into()));
    assert!(state.judgment_log.is_empty());
}

#[test]
fn exhausting_the_board_ends_the_game() {
    let mut state = setup(&["alice", "bob"]);
    state.apply(token("mod"), Command::StartGame);

    for _ in 0..4 {
        let p = picker(&state);
        let (category, point) = first_open_cell(&state);
        state.apply(token(&p), Command::PickCell { category, point });
        state.apply(token("mod"), Command::Rule { player: p, verdict: Verdict::Correct });
        if grid(&state).phase == GridQuizPhase::Reveal {
            state.apply(token("mod"), Command::Next);
        }
    }

    let g = grid(&state);
    assert_eq!(g.phase, GridQuizPhase::GameOver);
    assert!(
        g.cells.iter().all(|col| col.iter().all(|c| matches!(c, Cell::Used(_)))),
        "all cells used"
    );
    assert_eq!(
        state.judgment_log.iter().map(|j| j.points).sum::<i32>(),
        600,
        "100+200+100+200 awarded in total"
    );
}
