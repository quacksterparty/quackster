#!/usr/bin/env -S uv run --script --with websockets
# /// script
# requires-python = ">=3.11"
# dependencies = ["websockets>=12"]
# ///
"""
Quackster backend harness — drive the live game over WebSocket from the
command line. Two scenarios:

  smoke  create room, two bots join, print state, exit
  play   drive a full game: StartGame → PickCell → Answer → Rule → Next loop
         until GameOver. Alternates correct/incorrect to exercise scoring,
         lockout, and the auto-close on all-locked branches.

  both   smoke then play

Use `play` to find bugs in the game runtime — every command sent and every
phase transition is logged. Intentionally low-level: stays close to the wire
shape so renaming/protocol bugs in api/src/protocol.rs surface instead of
being hidden by a friendlier wrapper.
"""

import argparse
import asyncio
import json
import os
import sys
import urllib.request
from typing import Any, Optional

import websockets

BASE = "http://localhost:3000"
WS_BASE = "ws://localhost:3000"
DEBUG = os.environ.get("QUACKSTER_DEBUG") == "1"


def http_post(path: str, body: dict) -> Any:
    req = urllib.request.Request(
        f"{BASE}{path}",
        data=json.dumps(body).encode(),
        headers={"Content-Type": "application/json"},
        method="POST",
    )
    with urllib.request.urlopen(req) as r:
        return json.loads(r.read())


def http_get(path: str) -> Any:
    with urllib.request.urlopen(f"{BASE}{path}") as r:
        return json.loads(r.read())


class View:
    """Per-snapshot projection. Bare-bones — protocol renames show up here."""

    def __init__(self, raw: dict):
        self.raw = raw
        # `stage` is `GamemodeView` (internally tagged `#[serde(tag = "kind")]`):
        # variant data is flattened next to `kind`, NOT nested under a variant
        # key. So `stage.kind == "GridQuiz"` and `stage.phase` lives there.
        stage = raw.get("stage", {})
        self.mode: str = stage.get("kind", "?")
        self.phase: str = stage.get("phase", "?")
        self.active_picker: Optional[str] = stage.get("active_picker")
        self.floored: Optional[str] = stage.get("floored")
        self.current_category: Optional[str] = stage.get("current_category")
        self.current_points: Optional[int] = stage.get("current_points")
        self.locked_out: list[str] = stage.get("locked_out", [])
        self.categories: list[str] = stage.get("categories", [])
        self.points: list[int] = stage.get("points", [])
        self.used: list[list[bool]] = stage.get("used", [])
        self.players: dict[str, dict] = raw.get("players", {})
        self.question: Optional[dict] = raw.get("question")
        self.judgment_log: list[dict] = raw.get("judgment_log", [])

    def my_grants(self, name: str) -> list[str]:
        return list(self.players.get(name, {}).get("grants", []))

    def first_open_cell(self) -> tuple[int, int]:
        for c in range(len(self.categories)):
            for p in range(len(self.points)):
                if not self.used[c][p]:
                    return c, p
        raise RuntimeError("no open cells")


class Bot:
    def __init__(self, name: str):
        self.name = name
        self.ws: Optional[websockets.WebSocketClientProtocol] = None
        self.token: Optional[str] = None
        self.view: Optional[View] = None
        self.grants: list[str] = []
        self._buf: list[dict] = []

    async def connect(self, join_code: str, *, locale: str = "en"):
        self.ws = await websockets.connect(f"{WS_BASE}/ws/{join_code}")
        await self.ws.send(
            json.dumps({"kind": "Join", "name": self.name, "locale": locale})
        )
        # server emits Joined + initial Snapshot
        self._buf.append(json.loads(await self.ws.recv()))
        self._buf.append(json.loads(await self.ws.recv()))
        self._consume()
        if not self.token:
            raise RuntimeError(f"{self.name}: no Joined in handshake")
        if not self.view:
            raise RuntimeError(f"{self.name}: no Snapshot in handshake")

    def _consume(self):
        for msg in self._buf:
            k = msg.get("kind")
            if k == "Joined":
                self.token = msg["token"]
            elif k == "Snapshot":
                # tuple variant: ClientView fields are flattened next to `kind`
                self.view = View(msg)
                self.grants = self.view.my_grants(self.name)
            elif k == "Error":
                raise RuntimeError(f"{self.name} server error: {msg.get('message')}")
        self._buf.clear()

    async def _drain_to_snapshot(self, timeout: float) -> View:
        deadline = asyncio.get_event_loop().time() + timeout
        while True:
            for m in self._buf:
                if m.get("kind") == "Error":
                    self._consume()
                    raise RuntimeError(f"{self.name} server error: {m.get('message')}")
            if any(m.get("kind") == "Snapshot" for m in self._buf):
                self._consume()
                return self.view

            remaining = deadline - asyncio.get_event_loop().time()
            if remaining <= 0:
                raise TimeoutError(
                    f"{self.name}: no snapshot within {timeout}s "
                    f"(buf={len(self._buf)}, phase={self.view.phase if self.view else '?'})"
                )
            raw = await asyncio.wait_for(self.ws.recv(), timeout=remaining)
            msg = json.loads(raw)
            if DEBUG:
                print(f"  [{self.name}] ← {msg}", file=sys.stderr)
            self._buf.append(msg)

    async def cmd(self, **kwargs) -> None:
        if not self.token:
            raise RuntimeError(f"{self.name}: not joined")
        await self.ws.send(
            json.dumps({"kind": "Authed", "token": self.token, "cmd": kwargs})
        )

    async def wait_phase(self, phase: str, *, timeout: float = 5.0) -> View:
        while True:
            v = await self._drain_to_snapshot(timeout)
            if v.phase == phase:
                return v

    async def drain(self, *, timeout: float = 5.0) -> View:
        return await self._drain_to_snapshot(timeout)

    async def close(self):
        if self.ws:
            await self.ws.close()
            self.ws = None


def picker_of(view: View, bots: list[Bot]) -> Bot:
    by_name = {b.name: b for b in bots}
    try:
        return by_name[view.active_picker]
    except KeyError:
        raise RuntimeError(f"unknown active_picker {view.active_picker!r}")


async def join_all(code: str, bots: list[Bot]) -> None:
    """Connect each bot in order; host drains after each non-host join."""
    host = bots[0]
    await host.connect(code)
    for other in bots[1:]:
        await other.connect(code)
        await host.drain()


async def scenario_smoke(game_id: str, players: int):
    games = http_get("/api/games")
    print(f"[smoke] games: {[g['id'] for g in games]}")
    room = http_post("/api/rooms", {"game_id": game_id})
    code = room["join_code"]
    print(f"[smoke] created room {code} (players={players})")

    bots = [Bot(f"player{i}") for i in range(players)]
    await join_all(code, bots)

    host, *rest = bots
    print(
        f"[smoke] {host.name} joined: grants={host.grants} "
        f"phase={host.view.phase} players={list(host.view.players)}"
    )
    for r in rest:
        print(f"[smoke] {r.name} joined: grants={r.grants}")

    assert "Moderate" in host.grants, f"{host.name} expected Moderate, got {host.grants}"
    for r in rest:
        assert "Moderate" not in r.grants, f"{r.name} should not be Moderate, got {r.grants}"

    for b in bots:
        await b.close()
    print("[smoke] OK")


async def scenario_join(code: str, players: int):
    """Join an existing room you control elsewhere. Bots answer when picker;
    you drive every host action (StartGame, PickCell, Rule, Next) from your UI."""
    bots = [Bot(f"player{i}") for i in range(players)]
    await join_all(code, bots)
    for b in bots:
        print(f"[join] {b.name} joined: grants={b.grants}")

    last_phase: Optional[str] = None
    try:
        while True:
            await asyncio.gather(*(b.drain(timeout=60.0) for b in bots))
            phase = bots[0].view.phase
            if phase != last_phase:
                print(f"[join] phase={phase} players={list(bots[0].view.players)}")
                last_phase = phase
            if phase == "game_over":
                break
            # Only the picker answers in grid_quiz; others just observe.
            for b in bots:
                if b.view.phase == "question_open":
                    await b.cmd(kind="Answer", text="harness_answer")
                    break
    except (KeyboardInterrupt, asyncio.CancelledError):
        print("[join] interrupted")

    print(f"[join] final players={bots[0].view.players}")
    for b in bots:
        await b.close()


async def scenario_play(game_id: str, players: int):
    room = http_post("/api/rooms", {"game_id": game_id})
    code = room["join_code"]
    print(f"[play] room={code} game={game_id} players={players}")

    bots = [Bot(f"player{i}") for i in range(players)]
    host = bots[0]
    await join_all(code, bots)

    print(
        f"[play] lobby: phase={host.view.phase} "
        f"categories={host.view.categories} points={host.view.points}"
    )
    for b in bots:
        print(f"[play] {b.name} grants={b.grants}" + (" (host)" if b is host else ""))

    await host.cmd(kind="StartGame")
    await asyncio.gather(*(b.wait_phase("board_select") for b in bots))
    print(f"[play] → StartGame; active_picker={host.view.active_picker}")

    cells_played = 0
    rule_correct = True
    errors: list[str] = []

    while host.view.phase != "game_over":
        c, p = host.view.first_open_cell()
        cat = host.view.categories[c]
        pts = host.view.points[p]
        picker_bot = picker_of(host.view, bots)
        print(
            f"[play] → PickCell ({cat}, {pts}) by {picker_bot.name} "
            f"[{c},{p}]"
        )
        await picker_bot.cmd(kind="PickCell", category=c, point=p)
        await picker_bot.wait_phase("question_open")
        await asyncio.gather(*(b.drain() for b in bots if b is not picker_bot))

        q = picker_bot.view.question
        prompt_text = (q or {}).get("prompt", {}).get("text", "")
        print(f"[play]   Q: {prompt_text[:100]!r}")
        await picker_bot.cmd(kind="Answer", text="harness_answer")
        await asyncio.gather(*(b.drain() for b in bots))

        verdict = "correct" if rule_correct else "incorrect"
        rule_correct = not rule_correct
        print(f"[play] → Rule {verdict} (host)")
        await host.cmd(kind="Rule", verdict=verdict)
        await asyncio.gather(*(b.wait_phase("reveal") for b in bots))
        cells_played += 1

        log_tail = host.view.judgment_log[-1] if host.view.judgment_log else None
        print(f"[play]   judgment_log[-1]={log_tail}")

        print("[play] → Next (host)")
        await host.cmd(kind="Next")
        try:
            await asyncio.gather(*(b.wait_phase("board_select", timeout=2.0) for b in bots))
        except TimeoutError:
            # Could be game_over (board exhausted). wait_phase already
            # consumed the latest snapshot into self.view — check it
            # directly instead of calling drain() (which would re-block).
            pass
        if host.view.phase == "game_over":
            break
        if host.view.phase == "board_select":
            continue
        errors.append(f"stuck after Next; phase={host.view.phase}")
        break

    print(f"[play] game_over after {cells_played} cells")
    print(f"[play] final players={host.view.players}")
    print(f"[play] final judgment_log={host.view.judgment_log}")

    # Bug probe: Command::EndGame is declared in protocol.rs but never matched
    # in grid_quiz::apply — falls through to `todo!()`, panicking the room
    # task. Confirms by attempting it and observing the connection die.
    print("[play] → EndGame (expect todo!() panic — known unimplemented bug)")
    try:
        await host.cmd(kind="EndGame")
        try:
            await host.drain(timeout=2.0)
            print("[play] EndGame: no panic observed — bug may be fixed?")
        except TimeoutError:
            print("[play] EndGame: no response (room likely panicked) — bug confirmed")
        except websockets.ConnectionClosed:
            print("[play] EndGame: socket closed by server — bug confirmed")
    except websockets.ConnectionClosed as e:
        print(f"[play] EndGame: send failed, socket already closed ({e}) — bug confirmed")

    for b in bots:
        await b.close()

    if errors:
        print(f"[play] ERRORS: {errors}")
        sys.exit(1)


async def main():
    ap = argparse.ArgumentParser(description=__doc__.splitlines()[1].strip())
    ap.add_argument("scenario", nargs="?", default="play", choices=["play", "smoke", "both", "join"])
    ap.add_argument("--game", default="game_school_quiz")
    ap.add_argument("--players", type=int, default=2, help="number of bots to join")
    ap.add_argument("--code", help="join existing room code (for `join` scenario)")
    args = ap.parse_args()
    if args.players < 1:
        sys.exit("--players must be >= 1")

    if args.scenario == "join":
        if not args.code:
            sys.exit("--code required for join scenario")
        await scenario_join(args.code, args.players)
    else:
        if args.scenario in ("smoke", "both"):
            await scenario_smoke(args.game, args.players)
        if args.scenario in ("play", "both"):
            await scenario_play(args.game, args.players)


if __name__ == "__main__":
    asyncio.run(main())