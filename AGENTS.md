# Quackster — Agent Guide

## What this is

Quackster is a **self-hostable, multi-gamemode, open quiz platform** —
think Kahoot/ClassQuiz but with multiple gamemodes (classic, battle royale,
survival, music quiz, quiz duel/jeopardy, who wants to be a millionair, higher or lower, …) sharing a single pool of community-
contributed, translatable questions.

Content (questions, packs, tags, media) lives in the repo as
human-editable YAML and is PR-reviewable. The runtime loads, validates,
merges translation overlays, and serves it to gamemodes that declare what
content they accept.

## Status

> **Backend is Rust + axum**; SvelteKit is a pure static frontend (no server, no
> remote functions). The data layer lives in Rust (`api/src/data/`); `ts-rs`
> exports Rust types to TS. The legacy TS data layer (`src/lib/server/data/`,
> `src/lib/schemas/`) and its valibot/effect schemas have been **removed** — Rust
> reached parity. **`docs/architecture.md` is the canonical runtime reference**
> (backend ownership, transport, concurrency, game-session model);
> `docs/data-model.md` remains canonical for content shape.

Data layer done in Rust: types, garde validation, YAML loader, cross-file validation, pool query engine, board builder, plus an example dataset (~20 questions, German + English overlays, 4 game configs in `data/games/`, `grid_quiz` gamemode). **Game runtime is the current focus** — room actor + state apply + per-role projection + WebSocket edge + REST create/check + score-from-log (ADR 0002) + per-player locale resolution are live; three mechanical pieces remain (see Implementation order). Frontend game shell built over `/room`, `/curate`, `/playground`, `/questions`: AppShell, Navbar, drawers (Players/Mod/Settings), QuestionList, RoomQR, Toaster, plus 8 themes in `src/lib/themes/`. Docker images published to GHCR (`slim` + `full`); `harness/play.py` is the WebSocket driver for runtime testing. JSON Schema export for editor YAML LSP is not yet re-sourced from Rust — tracked as `quackster-3` in `TODO.org`.

## Tech stack

**Backend (Rust):**

- **Rust + axum** (`api/`) — data layer + live game runtime. Serves the static
  frontend and the API (REST for cold content, WebSocket for live game state).
- **`garde`** for validation, **`serde_yaml`** for content parsing.
- **`ts-rs`** exports Rust types to TS — single source of truth for shared types
  (lands in `src/lib/bindings/`).
- **`tokio`** for the per-room actor concurrency model; **`dashmap`** for the
  room registry; **`uuid`** + **`rand`** for join codes and seeded PRNG.
- **`tracing`** + **`tracing-opentelemetry`** for observability (OTLP export
  opt-in — core loop runs offline; ADR 0003).
- **`thiserror`** for fallible path errors.

**Frontend (SvelteKit, static):**

- **SvelteKit 2 + Svelte 5** (runes), **`@sveltejs/adapter-static`** — no server,
  no SSR, built to `build/` and served by the Rust backend.
- **TypeScript** (strict), **Vite** for build/dev.
- **Paraglide JS** (inlang) for UI i18n; messages in `messages/{en,de}.json`.
- **`bits-ui`** for primitives, **`qrcode`** for join QR codes,
  **`@fontsource/*`** bundles per-theme fonts (no runtime font fetch).
- **Vitest** + **Playwright** for tests; **ESLint + Prettier**.
- **pnpm** workspaces.

Runtime concurrency: each game room is an isolated `tokio` task (mpsc in,
broadcast out); state streams to clients over WebSocket as role-specific
projections. See `docs/architecture.md`.

## Layout (current + planned)

```
api/                # Rust backend
  src/
    main.rs         # axum: load data, serve build/ + REST/WS API
    config.rs
    protocol.rs     # wire types — ts-rs exports to src/lib/bindings/
    media.rs        # yt-dlp segment cache + mod-controlled playback
    state.rs        # AppState (DashMap<JoinCode, RoomHandle>)
    data/           # loader, validate, query, board, grid_quiz, error, types/
    game/           # room actor, state apply, judge, project, grants
    http/           # ws.rs (WebSocket edge), rest/ (rooms.rs), auth.rs, locale.rs
src/                # SvelteKit static frontend
  lib/
    paraglide/      # generated UI i18n runtime — do not edit
    bindings/       # ts-rs-generated TS types (Games, Grants, Protocol, Rooms, Verdict)
    themes/         # 8 themes: chalkboard, kawaii, medieval, modern-dark, neon, retro, western, wizard
    components/     # AppShell, Navbar, drawers (Players/Mod/Settings), QuestionList, RoomQR, Toaster, …
    api.ts          # REST client (room create/check, games list)
    room.svelte.ts  # shared reactive room state across components
  routes/
    +page.svelte              # Home / Host
    room/[code]/+page.svelte  # live game (WebSocket owner)
    curate/+page.svelte       # content review
    playground/+page.svelte   # dev sandbox
    questions/+page.svelte    # question browser
  hooks.ts          # paraglide locale reroute
messages/           # paraglide UI strings (en, de)
project.inlang/     # paraglide config
docs/
  architecture.md   # canonical runtime reference
  data-model.md     # canonical content reference
  game-flow.md      # host/player journeys, screens, decisions
  glossary.md       # domain vocabulary
  decisions/        # ADRs (0001 stack, 0002 score-from-log, 0003 offline-capable, 0004 persistence+HA)
data/               # content
  questions/        # YAML, grouped by topic (geography, science, music, …)
  packs/            # curated question lists / filters
  tags/             # registries, one file per category
  i18n/{de,en}/     # translation overlays (questions, tags)
  media/            # local binary assets
  games/            # game configs (rule axes + inline grid board)
harness/
  play.py           # WebSocket driver — `smoke` and `play` scenarios
```

## Architectural rules (from data-model.md)

- **Three layers, never mixed:** Questions (raw facts) · Packs (curated
  lists / filters) · Gamemodes (rules + presentation).
- **Kinds + variants** on questions, not one-type-per-question. Same fact
  plays as MC, T/F, open, numeric_input, range, order, etc.
- **Correctness lives on data** (`correct: true`, `position: N`,
  `answer: <num>`) — no separate answer key, no desync possible.
- **Translation overlays mirror canonical shape** under `data/i18n/<lang>/`.
  Overlays may only touch translatable fields; schema rejects edits to
  `correct`, `position`, numeric `answer`, `tolerance`, `min/max/step`.
- **Tags are `category:slug`** with closed-enum categories
  (`subject`, `difficulty`, `audience`, `region`, `format`, `warning`).
  Registry split one file per category under `data/tags/`.
- **Media refs use `prefix:value`** (`local:`, `url:`, `youtube:`),
  same shape as tags.
- **Licenses are SPDX from an allowlist**, schema-validated.
- **IDs are public API** — `q_<slug>`, `pack_<slug>`, `board_<slug>`, gamemode bare slug.
  No rename/delete without deprecation marker.
- **Validation runs at:** CI (validate step) and runtime (server start, via
  `cargo run` loading `../data`). Editor YAML LSP is pending a Rust-sourced
  JSON Schema export. Same Rust schema definitions everywhere.

When in doubt about content shape, **read `docs/data-model.md`**; for the
runtime, **read `docs/architecture.md`** — they are the spec, this file is just
orientation.

## Scripts

```sh
pnpm dev              # vite dev server
pnpm build            # production build
pnpm preview          # preview built app
pnpm check            # svelte-kit sync + svelte-check
pnpm lint             # prettier --check + eslint
pnpm format           # prettier --write
pnpm test:unit        # vitest
pnpm test:e2e         # playwright (auto-installs browsers)
pnpm test             # unit + e2e
```

Backend (Rust), from `api/`:

```sh
cargo run             # load ../data (validates, logs issues), serve ../build + API
cargo test
```

Planned (not yet implemented): `new-question` scaffolding, Rust JSON Schema
export for editor YAML LSP.

## Conventions

- Svelte 5 runes (`$state`, `$derived`, `$effect`), not legacy reactive `$:`.
- Frontend talks to the backend over REST (cold content) + WebSocket (live game
  state); no SvelteKit server logic (the adapter is static).
- Schema definitions are the source of truth and live in Rust
  (`api/src/data/types/`, validated by `garde`); TS types are generated via
  `ts-rs`, never hand-edited.
- Rust code uses `Result` / `thiserror` idiomatically for fallible paths.
- **Always use descriptive variable names** — no shorthand. Readability over
  brevity. `event_handler` not `eh`, `question_count` not `qc`, `board_config`
  not `bc`. Type narrowing usually makes "shorter" names pointless anyway.
  Applies to Rust, TS, YAML keys, parameter names, locals — everywhere.
- **Run commands inside `nix develop`** (or `nix develop -c <cmd>` for one-offs).
  Don't install tools system-wide. Especially required for frontend tests —
  Playwright needs its bundled browsers and we can't `pnpm exec playwright
install` against the host's missing system deps. `nix develop` already has
  every browser + system lib wired up.
- No comments restating what code does; comment only non-obvious _why_.
- Don't edit `src/lib/paraglide/**` — generated.
- Tasks live in `TODO.org` at the repo root (Org format, `#+TODO: TODO IN_PROGRESS DONE`).
  Read it at session start. Update it as you go: flip a heading to `IN_PROGRESS`
  when you start a task, mark it `DONE` when the work lands, and add new `* TODO`
  entries you discover during the work. When marking DONE, add a `CLOSED: [YYYY-MM-DD Day HH:MM]`
  line directly under the heading (matches existing entries in the file). Priorities
  are Org tags (`p1`/`p2`/`p3`); projects are tags (`runtime`, `frontend`, `tooling`,
  `gamemodes`). Each entry keeps an `:ID: quackster-N` property for the `nb` provenance;
  large items split their work into child `** TODO` / `** DONE` subheadings, completed
  with the parent. The `quackster` notebook in `nb` is read-only history — do not write
  to it from the agent.

## Implementation order (from data-model.md §Implementation order)

Data layer (schemas, loader, validation, query, board, example dataset, first
gamemode) is done and ported to Rust (`api/src/data/`); the legacy TS layer has
been removed. The game runtime is the current focus — most of it is built, but
three mechanical pieces remain. See `docs/data-model.md` and `TODO.org` for the
fine-grained breakdown; high-level status below.

1. ✅ Data layer in Rust: types, loader, cross-file validation, query, board.
2. ✅ Example dataset: ~20 questions, German + English overlays, 4 game configs
   in `data/games/`, `grid_quiz` gamemode.
3. ✅ First gamemode: `grid_quiz` — board logic in `api/src/data/grid_quiz.rs`,
   runtime rules still hardcoded in the room task (see step 4).
4. 🔄 Game runtime (rooms, WebSocket, scoring) — `quackster-2` in `TODO.org`:
   - ✅ Room actor (`spawn_room`, mpsc in / broadcast out, `select!` loop) — `api/src/game/room.rs`
   - ✅ WebSocket edge (`handle_socket`, join / reconnect / authed flow) — `api/src/http/ws.rs`
   - ✅ Registry (`AppState.rooms` DashMap + REST create/check) — `api/src/state.rs`, `api/src/http/rest/rooms.rs`
   - ✅ Token + Grant model (`Play` / `Present` / `Moderate`) — `api/src/game/grants.rs`
   - ✅ Per-role `ClientView` projection + correct-answer strip — `api/src/game/project.rs`
   - ✅ Judgment log + derived score — ADR 0002 (`api/src/game/state.rs`)
   - ✅ Per-player locale resolution + layered overlay merge — `quackster-31`
   - 🔄 `Judge` trait + `Auto` + `Moderator` impls — `api/src/game/judge.rs` (stub)
   - 🔄 Timer `sleep_until(deadline)` arm in the `select!` loop — `api/src/game/room.rs`
   - 🔄 `Gamemode` trait extraction — deferred until step 5 lands
5. ○ Second gamemode to validate gamemode-agnostic claim. **Boards are currently
   inline in `data/games/*.yaml`** rather than split out under `gamemodes/<id>/boards/`
   — separation back into standalone board YAML is revisited if/when a second
   gamemode makes the split pay off.
6. ○ `new-question` scaffolding script.
7. ○ Rust JSON Schema export for editor YAML LSP — `quackster-3` in `TODO.org`
   (schemars derive vs. manual schema builder still TBD).

## Open questions

Tracked in `docs/data-model.md` §Open questions — community pack pipeline,
question `revision` field, RTL support, web editor UI, duplicate-fact
detection, overlay merge semantics, gamemode schema export.
