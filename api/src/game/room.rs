//! The room actor — one owning tokio task per live game.
//!
//! `spawn_room()` creates the channels and spawns the task; returns a
//! `RoomHandle { cmd_tx, state_tx }` for the registry. The task loop is a
//! `tokio::select!` racing: next `Command` (mpsc), the timer deadline
//! (`sleep_until`), and shutdown — mutate state, broadcast a snapshot, loop.
//! Sole owner of `GameState`, so mutation needs no lock; first-buzz-wins falls
//! out of mpsc ordering. On exit, removes its own entry from the registry.
//!
//! v1: grid_quiz buzz/lockout/timer/scoring policy lives directly in this loop.
//!
//! TODO: RoomHandle, spawn_room, the select! loop.

use std::{
    collections::{BTreeMap, HashSet},
    sync::Arc,
};

use rand::RngExt;
use tokio::sync::{broadcast, mpsc};

use crate::{
    data::{Dataset, GameConfig, GameMode, build_board, normalize_locale},
    game::{
        grants::Grant::{self},
        grid_quiz::{Cell, GridQuizState},
        state::{GameState, LinearState, ModeState, PlayerSlot, PlayerSlots, Token},
    },
    media::MediaFetcher,
    protocol::{Command, ConnectionError, MediaFetchStatus, RoomMessage},
};

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct JoinCode(pub String);

const ALPHABET: &[u8] = b"ABCDEFGHJKMNPQRSTUVWXYZ23456789"; // no 0 O 1 I L
const LEN: usize = 6;

impl JoinCode {
    pub fn generate() -> Self {
        let mut rng = rand::rng();
        JoinCode(
            (0..LEN)
                .map(|_| {
                    let i = rng.random_range(0..ALPHABET.len());
                    ALPHABET[i] as char
                })
                .collect(),
        )
    }
}

#[derive(Clone)]
pub struct RoomHandle {
    pub command_tx: mpsc::Sender<RoomMessage>,
    pub state_tx: broadcast::Sender<Arc<GameState>>,
}

pub fn spawn_room(
    code: JoinCode,
    game_config: GameConfig,
    data: Arc<Dataset>,
    media_fetcher: Arc<MediaFetcher>,
) -> RoomHandle {
    let (room_msg_tx, mut room_msg_rx) = mpsc::channel::<RoomMessage>(64);
    let (state_tx, _) = broadcast::channel::<Arc<GameState>>(16);

    let state_tx_loop = state_tx.clone();
    let room_msg_tx_for_task = room_msg_tx.clone();
    tokio::spawn(async move {
        let seed = rand::rng().random::<u64>();
        let mode = match &game_config
            .games
            .first()
            .expect("a room without a game")
            .mode
        {
            GameMode::GridQuiz(game) => {
                let cells: Vec<Vec<Cell>> =
                    build_board(&data, &game.board, seed, media_fetcher.enabled())
                        .into_iter()
                        .map(|row| row.into_iter().map(Cell::from).collect())
                        .collect();
                ModeState::GridQuiz(Box::new(GridQuizState::build(
                    cells,
                    game.board.points.clone(),
                )))
            }
            GameMode::Linear(_) => ModeState::Linear(Box::new(LinearState)),
        };

        let media_status = match &mode {
            ModeState::GridQuiz(grid) => collect_youtube_refs(&data, &grid.cells),
            // TODO: walk resolved linear question list once Linear runtime lands.
            ModeState::Linear(_) => BTreeMap::new(),
        };

        let mut state = GameState {
            game_config,
            current_game_idx: 0,
            player_slots: PlayerSlots::default(),
            mode,
            judgment_log: Vec::new(),
            seed,
            media_status,
        };

        // Kick prefetch after the map is populated so the actor sees
        // `Pending` first, then `Downloading`/`Ready`/`Failed` as they land.
        if let ModeState::GridQuiz(grid) = &state.mode {
            let cells: Vec<Vec<Cell>> = grid.cells.clone();
            prefetch_board_media(&cells, &data, &media_fetcher, room_msg_tx_for_task.clone());
        }

        while let Some(room_msg) = room_msg_rx.recv().await {
            tracing::info!("room msg {:?}", &room_msg);
            match room_msg {
                RoomMessage::Join {
                    name,
                    locale,
                    reply,
                } => {
                    let Some(locale) = normalize_locale(&locale) else {
                        let _ = reply.send(Err(ConnectionError::InvalidLocale));
                        continue;
                    };
                    let taken = state.player_slots.values().any(|slot| slot.name == name);
                    if taken {
                        let _ = reply.send(Err(ConnectionError::NameTaken));
                        continue;
                    }

                    let grants = if state.player_slots.is_empty() {
                        HashSet::from([Grant::Moderate])
                    } else {
                        HashSet::from([Grant::Play])
                    };

                    let token = Token::generate();
                    let media_locale = locale.clone();
                    state.player_slots.insert(
                        token.clone(),
                        PlayerSlot {
                            name,
                            locale,
                            connected: true,
                            grants,
                        },
                    );
                    prefetch_locale_media(
                        &mut state,
                        &data,
                        &media_locale,
                        &media_fetcher,
                        room_msg_tx_for_task.clone(),
                    );

                    let _ = reply.send(Ok(token));
                }
                RoomMessage::Reconnect {
                    token,
                    locale,
                    reply,
                } => {
                    let Some(locale) = normalize_locale(&locale) else {
                        let _ = reply.send(Err(ConnectionError::InvalidLocale));
                        continue;
                    };
                    let Some(slot) = state.player_slots.get_mut(&token) else {
                        let _ = reply.send(Err(ConnectionError::SlotGone));
                        continue;
                    };

                    slot.locale = locale.clone();
                    slot.connected = true;

                    prefetch_locale_media(
                        &mut state,
                        &data,
                        &locale,
                        &media_fetcher,
                        room_msg_tx_for_task.clone(),
                    );
                    let _ = reply.send(Ok(token));
                }
                RoomMessage::Disconnect { token } => {
                    if let Some(slot) = state.player_slots.get_mut(&token) {
                        slot.connected = false;
                    }
                }
                RoomMessage::MediaStatus { media_ref, status } => {
                    state.media_status.insert(media_ref, status);
                }
                RoomMessage::Client { token, cmd } => {
                    tracing::info!("command {:?} received in room {}", cmd, &code.0);

                    match &cmd {
                        Command::RetryMediaFetch => {
                            retry_failed(
                                &mut state,
                                &data,
                                &media_fetcher,
                                room_msg_tx_for_task.clone(),
                            );
                        }
                        Command::SetLocale { locale } => {
                            if let Some(normalized) = state.try_set_locale(&token, locale) {
                                prefetch_locale_media(
                                    &mut state,
                                    &data,
                                    &normalized,
                                    &media_fetcher,
                                    room_msg_tx_for_task.clone(),
                                );
                            }
                        }
                        _ => state.apply(token, cmd),
                    }
                }
            }

            let _ = state_tx_loop.send(Arc::new(state.clone()));
        }
    });

    RoomHandle {
        command_tx: room_msg_tx.clone(),
        state_tx,
    }
}

/// Init map to `Pending` for every `youtube:` ref on the board.
fn collect_youtube_refs(data: &Dataset, cells: &[Vec<Cell>]) -> BTreeMap<String, MediaFetchStatus> {
    youtube_refs_for_questions(data, &grid_question_ids(cells))
        .into_iter()
        .map(|media| (media.media_ref, MediaFetchStatus::Pending))
        .collect()
}

fn youtube_refs_for_questions(data: &Dataset, question_ids: &[&str]) -> Vec<crate::data::Media> {
    question_ids
        .iter()
        .filter_map(|id| data.questions.get(*id))
        .filter_map(|entry| entry.item.prompt().media.as_deref())
        .flatten()
        .filter(|media| media.media_ref.starts_with("youtube:"))
        .cloned()
        .collect()
}

fn prefetch_board_media(
    cells: &[Vec<Cell>],
    data: &Dataset,
    media_fetcher: &Arc<MediaFetcher>,
    command_tx: mpsc::Sender<RoomMessage>,
) {
    let to_fetch = youtube_refs_for_questions(data, &grid_question_ids(cells));
    media_fetcher.prefetch(to_fetch, command_tx);
}

fn prefetch_locale_media(
    state: &mut GameState,
    data: &Dataset,
    locale: &str,
    media_fetcher: &Arc<MediaFetcher>,
    command_tx: mpsc::Sender<RoomMessage>,
) {
    let cells = match &state.mode {
        ModeState::GridQuiz(grid) => &grid.cells,
        ModeState::Linear(_) => return,
    };

    let media = data.localized_youtube_media(locale, &grid_question_ids(cells));
    for item in &media {
        state
            .media_status
            .entry(item.media_ref.clone())
            .or_insert(MediaFetchStatus::Pending);
    }
    media_fetcher.prefetch(media, command_tx);
}

/// Re-kick downloads for every currently-failed ref. Flips them to `Pending`
/// so the lobby updates immediately, then re-jobs the fetcher.
fn retry_failed(
    state: &mut GameState,
    data: &Dataset,
    media_fetcher: &Arc<MediaFetcher>,
    command_tx: mpsc::Sender<RoomMessage>,
) {
    let cells = match &state.mode {
        ModeState::GridQuiz(grid) => &grid.cells,
        ModeState::Linear(_) => return,
    };
    let question_ids = grid_question_ids(cells);
    let to_fetch: Vec<_> = youtube_refs_for_questions(data, &question_ids)
        .into_iter()
        .filter(|media| {
            matches!(
                state.media_status.get(&media.media_ref),
                Some(MediaFetchStatus::Failed { .. })
            )
        })
        .collect();
    for media in &to_fetch {
        state
            .media_status
            .insert(media.media_ref.clone(), MediaFetchStatus::Pending);
    }
    media_fetcher.prefetch(to_fetch, command_tx);
}

fn grid_question_ids(cells: &[Vec<Cell>]) -> Vec<&str> {
    cells
        .iter()
        .flatten()
        .filter_map(|cell| match cell {
            Cell::Open(slot) | Cell::Used(slot) => Some(slot.question_id.as_str()),
            Cell::Empty => None,
        })
        .collect()
}
