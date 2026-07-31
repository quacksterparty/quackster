# Persistence and horizontal scaling: durable rooms, room-affinity, lease failover

`architecture.md` v1 keeps live rooms **in-memory only** (actor-per-room tokio
task, dropped on restart). This ADR records the design for two later goals that
share one storage layer: **crash-recovery for self-host**, and a **highly
available multi-pod Kubernetes deployment**. It does not change v1 — it is the
pre-thought-out path so neither is a rewrite.

## Scope split: HA/recovery, not horizontal load

The target is **availability** (survive a pod/node loss, resume the game), not
sharing one live game's load across pods. A quiz workload is tiny; one modest
process serves thousands of players. So a running room stays owned by **exactly
one pod at a time** — the actor model (single owner, `mpsc`-ordered input,
lockless timers, `broadcast` fan-out) is the whole architecture, and
first-buzz-wins / no-desync fall out of it. We keep it. Postgres gives
**durability**, not live coordination: two pods both mutating one room's rows
would need row locks on every buzz plus a cross-pod pub/sub bus to fan
broadcasts — a slower, larger system, and slower buzzes in a buzzer game is a
real UX regression. Rejected.

## Room-affinity: every socket for a room reaches its one owner pod

Because a room's channels/timer live in one pod's task, all sockets for a join
code must land on that pod. Two halves:

- **Assignment** — a **Postgres registry** row decides ownership:
  `rooms(join_code, owner_pod, lease_epoch, lease_expires, snapshot, updated_at)`.
  The creating pod writes the row and claims the lease. This is the in-memory
  `DashMap<JoinCode, RoomHandle>` promoted to shared storage for the cross-pod
  lookup; the DashMap stays as the local handle cache. Chosen over deterministic
  hash (`hash(code) % replicas`) — a hash remaps every code when the replica
  count changes, which in k8s (rolling deploys, scaling) orphans live rooms.
  Ownership must be **data**, not a function of topology.
- **Routing** — a socket may hit any pod; that pod reads `owner_pod` and, if it
  is not the owner, **redirects** the client to the owner's per-pod address
  (`307` at connect, before WS upgrade). Chosen over app-level frame forwarding
  (proxy every frame both ways for the game's life) — a WS lives a whole game, so
  paying one reconnect at join to then get a clean single-hop socket beats
  relaying every buzz. Fewer hops on the hot path = fairness in a buzzer game.
  The reconnect-token flow already in `architecture.md` means the client can
  re-dial a room by URL, so a redirect is just "dial this host instead."

### Kubernetes shape (Traefik + StatefulSet, no volumes)

The game tier runs as a **StatefulSet with no `volumeClaimTemplates`** — state is
in Postgres, so this is "a Deployment whose pods have stable names and DNS,"
nothing heavier. The stable per-pod identity is exactly the redirect target;
plain Deployment pods are unaddressable individually (a Service load-balances
across all of them), so per-pod external addressability is inherently a
stable-identity (StatefulSet + headless Service) feature. Traefik routes host/
path → per-pod Service; the app owns the registry lookup and redirect. **No
edge-native affinity can honor the registry** — session persistence (Gateway API
GEP-1619 / Traefik sticky cookies) pins _one client_ to a pod, not a _room's many
clients_; consistent-hash LB (Traefik `hrw`, hashes client IP) is stateless and
rehashes on scale — both are the deterministic-hash we rejected. The edge is
stateless; ownership is data; the registry-aware component is the app itself.

The Deployment fallback (podIP in the registry + internal pod-to-pod forward) is
possible but reintroduces the proxy hop we rejected, so it is not the path.

## Failover, rolling updates, and split-brain

- **Rolling update = failover fired on purpose.** StatefulSet updates one pod at
  a time, highest ordinal first, waiting for `Ready` between each. A `SIGTERM`
  drain window snapshots each live room, releases its leases, and signals clients
  to reconnect; freed leases let a surviving pod re-claim and resume from
  snapshot instantly. So a planned deploy is invisible — a game hops pods. The
  client reconnect machinery (built for "phone slept mid-quiz") absorbs it: one
  mechanism, three triggers (wifi blip, crash, deploy).
- **Lease-expiry recovery is the crash floor.** A `SIGKILL`/node death does not
  release leases, so leases carry `lease_expires`; the owner renews on a
  heartbeat, and a surviving pod's reconciler reclaims any room whose lease
  expired, rebuilding from the last snapshot. Same reclaim code both paths call —
  graceful just makes it instant instead of TTL-delayed.
- **Split-brain is fenced by a monotonic `lease_epoch`.** A slow-but-alive old
  pod (GC pause, partition) can wrongly believe it still owns a room another pod
  reclaimed. Every claim atomically increments `lease_epoch` via a
  compare-and-swap (`UPDATE … SET owner_pod=me, lease_epoch=lease_epoch+1,
lease_expires=now+ttl WHERE join_code=$c AND (lease_expires < now OR
owner_pod=me)` — one winner). **Every snapshot write is fenced with the same
  `WHERE lease_epoch=$my_epoch`**, so a stale owner's write matches zero rows and
  cannot corrupt state; the first `rows_affected = 0` tells the stale pod it was
  superseded, and it kills its local actor and drops its sockets (which reconnect
  → redirect → the real owner). Snapshot and fence are the same write — no extra
  round-trip. This is the standard fencing-token pattern; it turns split-brain
  from silent corruption into a stale pod harmlessly failing one write.

### Timing budget (probes and lease aligned)

k8s probes and the lease measure the same liveness from two directions and must
agree. Lease TTL sits **just above** the liveness detection window so k8s
usually restarts a zombie before reclaim fires, minimizing the fenced window:

| Knob                            | Value                                                                      |
| ------------------------------- | -------------------------------------------------------------------------- |
| `livenessProbe`                 | period 5s, failureThreshold 3 (~15s) — process health                      |
| `readinessProbe`                | period 5s, failureThreshold 2 (~10s) — **DB reachable + leases renewable** |
| lease heartbeat                 | 5s (same tick as probes)                                                   |
| lease TTL                       | 20s (4 missed beats)                                                       |
| `terminationGracePeriodSeconds` | 45s (> TTL, so drain always finishes before SIGKILL)                       |

Readiness leads liveness so a pod that loses Postgres is pulled from routing
before it is restarted. Readiness **must** check "can I own rooms" (ping DB /
confirm renewals), not a dumb `200`, or the fast gate is lost.

## Storage: `Store` trait — SQLite self-host, Postgres cluster

One `Store` trait, narrow surface (`snapshot_room`, `load_room`,
`list_rooms_for_recovery`, `persist_history`, `sweep_stale_rooms`, plus
Postgres-only lease `claim`/`renew`/`reclaim`). The fork falls exactly on the
line where the _problem_ forks:

- **`SqliteStore` (self-host)** — snapshot/load only. One file, offline-perfect,
  keeps `docs/decisions/0003` honest (no Postgres in the offline path). Single
  owner, no lease methods — the in-memory DashMap already handles affinity,
  persistence is just optional crash-recovery on one box. WAL mode (pragma) so
  many room tasks snapshot to one file concurrently.
- **`PgStore` (cluster)** — snapshot/load **fenced by epoch** + the lease CAS/
  renew/reclaim that only exist with multiple pods.

The game runtime (actor, channels, `select!`) is **identical** on both; it calls
`store.snapshot_room(...)`. Only deployment wiring (is there a lease? a
redirect?) differs, because the problem differs — principled fork, not accidental
complexity. Chosen over Postgres-everywhere (one code path, but a heavier
two-container self-host that breaks the single-binary-plus-a-file instinct) and
over SQLite-shared-via-LiteFS (single-writer; serializes every lease CAS through
one node, reintroducing the SPOF HA removes).

### The room snapshot is an opaque blob, never normalized

A live room is **one JSON blob in one `TEXT` column**, keyed by join code. It is
not shredded into `players`/`judgments`/`cells` tables, and the schema never
grows a column to describe gamestate. `GameState` is a document with a single
owner, no cross-room queries, no joins — SQL is used here for the key-value
store and (in the cluster) the fence CAS, nothing else. **SQL models results,
not gamestate.**

- **Self-contained.** The blob holds the whole `GameState`, resolved
  `GameConfig` (board, questions, correct answers) included. Restore is
  byte-identical to what players saw — no re-resolution from YAML, so a content
  edit mid-game, or two pods running images with different `data/`, cannot swap
  the board under a live room. Costs a few KB per room; buys the removal of an
  entire failure mode.
- **Timer deadlines are wall-clock epoch millis**, never `Instant` — `Instant`
  is neither serializable nor meaningful in another process.

### No versioning, no migrations for `rooms` — strict serde instead

A snapshot's useful life is **one quiz session (~40 min)**. Migration machinery
for data that never outlives an afternoon is museum code, so `load_room` returns
`Option<GameState>`: a decode failure is a **dead room**, not a fatal error — log
`warn!`, `DELETE` the row, return `None`, and the client dialing that code gets
"room gone."

That only stays safe if a schema change can never decode _successfully but
wrongly_. Rename `locked_out` → `lockouts` with a `#[serde(default)]` and the
room resumes with nobody locked out — silent corruption, and a
`schema_version` column would only guard it by human discipline (remember to
bump). Close the class mechanically instead: **`#[serde(deny_unknown_fields)]`
on every persisted type and no `#[serde(default)]` anywhere in them.** Missing
fields already error; `deny_unknown_fields` covers removals. Every shape change —
additive, removal, rename — becomes the loud decode error the drop path already
handles. (Consequence: no `#[serde(flatten)]` in these types, it is incompatible
with `deny_unknown_fields`.)

If a deploy during a live game ever becomes unacceptable (public instance, long
tournaments), the upgrade is `ALTER TABLE rooms ADD COLUMN schema_version
INTEGER NOT NULL DEFAULT 1` plus **migrate-on-read**: `load_room` sees an older
version, applies untyped `serde_json::Value` patches, then decodes. Adding the
column early buys nothing — it is a one-line retrofit.

Explicitly **not** a batch "migrate all rooms before booting the new binary":
ADR-0004's rolling update keeps old pods serving during the upgrade, and the old
pod's next per-transition snapshot is a _validly fenced_ write that clobbers the
migration. Making batch safe requires stop-the-world (`Recreate`, scale to 0),
which discards the HA this ADR exists to buy. Migrate on read, where the room's
single owner is the only writer, or not at all.

## History and stats: the durable dataset, one crossing point

Post-game history and question stats live in the **same DB and pool** as `rooms`
(the engine is already there, so a separate store would be a second thing to
operate for nothing) but under the **opposite policy**:

|               | `rooms` (ephemeral)          | history/stats (durable)           |
| ------------- | ---------------------------- | --------------------------------- |
| shape         | one opaque JSON blob per row | real columns, indexed             |
| lifetime      | ~40 min                      | forever                           |
| schema change | **drop rows**, no migration  | `sqlx::migrate!`, real migrations |
| write rate    | per state transition         | once per finished game            |

Written down because the two policies are contradictory on purpose; a migration
for the `rooms` table is a bug, not thoroughness.

The two datasets meet in **exactly one place**: `persist_history(&GameState)`,
called on game end, one transaction — project `judgment_log` into
`games(id, gamemode, board_id, started_at, ended_at)` /
`game_players(game_id, name, final_score)` /
`answers(game_id, question_id, player, verdict, points)` with superseded entries
folded out (history holds **live** rulings only, matching
`docs/decisions/0002`), then `DELETE FROM rooms`. Chosen over archiving the final
blob ("hardest question" would mean JSON-digging thousands of documents — SQL
bolted on later anyway) and over dual-writing relational rows during play (two
write paths for one fact, and the ephemeral/durable line blurs). The runtime is
unchanged: the room task is still the single writer.

- **Idempotent** — a crash between the insert and the room delete must not
  double-count a game: `games.id` = join code + `started_at`, insert
  `ON CONFLICT DO NOTHING`.
- **Abandoned rooms never reach game end** (host closes the tab), so a sweeper
  is required, not optional: rooms untouched beyond a TTL are
  history-projected-then-deleted, or plain deleted if no question was ever
  answered. Without it `rooms` grows forever _and_ stats silently omit every
  rage-quit game.

Deferred: live cross-game stats (the only goal that would justify dual-writing
during play), and the sweeper's TTL value.

### sqlx: runtime queries, not the checked macros

Use **`sqlx` with the runtime API (`query_as` + a `FromRow` struct), not the
`query!`/`query_as!` macros.** The macros validate the SQL string against **one**
DB at compile time; with two dialects behind a trait (`$1`/`BIGINT`/`now()` vs
`?`/`INTEGER`/`unixepoch()`) that fights every shared query. Type safety is
unchanged — it comes from the `FromRow` struct and from serde on the JSON
snapshot blob (sqlx only ever sees the payload as `TEXT`), and every query rides
`?` with no extra unwraps. The only loss is compile-time SQL-string checking,
replaced by **one integration test per `Store` impl** — which is wanted anyway,
because it verifies the fence CAS (does a stale epoch actually get rejected?), a
behavior the macro could never check. Always go through `query_as` + a struct,
never `query()` + manual `.get()` (which panics on type mismatch).

## Cost

The self-host path (v1 in-memory + optional SQLite snapshot) is untouched;
Postgres/lease/redirect is a **cluster-only** addition layered beside it, built
only when the k8s HA deployment is wanted. Contributors keep two `Store` impls,
but the runtime and the trait never move between phases. Snapshots are
per-transition (lockless — the room task is the single writer), which is the
correctness floor for crash recovery and the natural home of the fence.

Supersedes the "Persistence: none in v1 / add SQLite when crash-recovery
matters" note in `architecture.md` §Persistence for the _design_; v1 code stays
in-memory until the SQLite phase is actually built.
