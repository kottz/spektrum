# Spektrum state machines

This is the single reference for every explicit and implicit state machine in Spektrum. For each one it gives the states, the invariants that must hold, and the transition table. It also records:

- every invalid state or transition the code could actually reach, with evidence and its status;
- the compensating code that exists only to patch over a design flaw;
- redesigns under which that code would never have been written.

Code is cited as `file::function` rather than by line number, so references survive edits. When code changes, keep the tables and the status column in sync. Every _Fixed_ entry names the regression test that guards it.

Sections: [1. Server](#1-server) · [2. Frontend](#2-frontend) · [3. Admin panel](#3-admin-panel) · [4. Issue log](#4-issue-log) · [5. Compensating code](#5-compensating-code) · [6. Redesigns](#6-redesigns-from-first-principles)

---

## 1. Server

### 1.1 Game phase (`server/src/game.rs`, `GameEngine`)

**States:** `Lobby`, `Score`, `Question`, `GameOver`, `GameClosed`. `GameClosed` is terminal.

The round being played is one value, `round: Option<Round>`, where `Round` is `{question, correct_answers, alternatives, started_at}`. Also tracked: `current_question_index`, and per player `has_answered`, `answer`, `round_score`.

The round's deadline is `started_at + round_duration`. When `StartRound` creates a round, the server spawns a timer (`server.rs::spawn_round_timer`) that calls `end_round_if_expired` at the deadline. That call checks the engine's own deadline, so a timer that outlives its round (the admin ended it early and started another) does nothing.

**Invariants**

- I1: `GameClosed` is terminal. No event changes any state afterwards, and a sender is told the game is closed.
- I2: `phase == Question` ⇔ `round.is_some()`. Every other phase has it cleared (`clear_round`).
- I3: `shuffled_question_indices` is a permutation of _this lobby's_ question pool: the chosen set, or all questions. It holds for the lobby's whole lifetime, restarts included.
- I4: "Questions remaining" has one source: `prepare_next_round()` returns `None` exactly when `current_question_index >= shuffled_question_indices.len()`.
- I5: Player state is changed only by an action that succeeds. A rejected action mutates nothing.

| From                         | Event                                          | Guard                                                                               | To           | Effects                                                                                                                                                                           |
| ---------------------------- | ---------------------------------------------- | ----------------------------------------------------------------------------------- | ------------ | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| any non-closed               | `Connect`                                      | known player or admin                                                               | same         | `Connected` and a full `StateDelta` snapshot to the sender (with `admin_extra` for the admin); `AdminInfo` if the admin connects mid-question                                     |
| `GameClosed`                 | any event                                      | —                                                                                   | `GameClosed` | sender gets `GameClosed{"This game has ended"}` (I1)                                                                                                                              |
| `Lobby`, `GameOver`          | `StartGame`                                    | admin                                                                               | `Score`      | from `GameOver`: `reset_for_new_game` reshuffles the **same pool** and zeroes scores and misses; `StateDelta` to all; `AdminNextQuestions` to the admin (always, even when empty) |
| `Score`                      | `StartRound`                                   | admin, `prepare_next_round()` is `Some`                                             | `Question`   | players reset only after a question is secured (I5); round fields set; `StateDelta` to all; `AdminInfo` to the admin                                                              |
| `Score`                      | `StartRound`                                   | no questions left                                                                   | `Score`      | `Error{no_more_questions}` to the sender; nothing else changes                                                                                                                    |
| `Question`                   | `EndRound`, or the round timer at the deadline | admin (`EndRound`); `now >= deadline` (timer)                                       | `Score`      | misses updated, `clear_round`, index+1, `StateDelta` to all, `AdminNextQuestions`                                                                                                 |
| `Score`                      | `SkipQuestion`                                 | admin, questions remain                                                             | `Score`      | index+1, `AdminNextQuestions`                                                                                                                                                     |
| `Lobby`, `Score`, `Question` | `EndGame`                                      | admin                                                                               | `GameOver`   | `clear_round`; `GameOver{final_scores}` to all (points are awarded on answer, so they already include an unfinished round)                                                        |
| `GameOver`                   | `EndGame`                                      | —                                                                                   | `GameOver`   | `Error{invalid_phase}`                                                                                                                                                            |
| any non-closed               | `CloseGame`, admin `Leave`, 1 h inactivity     | —                                                                                   | `GameClosed` | single `close(reason)` transition: `clear_round`, `GameClosed` to all                                                                                                             |
| `Question`                   | `Answer`                                       | player, not yet answered, `elapsed <= round_duration` (exact `Duration` comparison) | `Question`   | score, `Answered` to all                                                                                                                                                          |
| any                          | `KickPlayer`                                   | admin, name exists                                                                  | same         | `PlayerKicked` to the target, then remove; `PlayerLeft` and a `StateDelta` with scoreboard, **round scores** and misses to all                                                    |
| any                          | player `Leave`                                 | —                                                                                   | same         | remove; `PlayerLeft` to all                                                                                                                                                       |
| (HTTP)                       | join                                           | lobby not closed, not full, not locked                                              | same         | player added; `StateDelta{scoreboard}` to all (any phase)                                                                                                                         |

### 1.2 Client connection (`game.rs::ClientConnection`, `server.rs::handle_socket`)

Each player and the admin hold `connection: Option<ClientConnection>`. A `ClientConnection` is `{id, tx, close: Arc<Notify>}`.

**States:** _unbound_ (`None`) and _bound_ (`Some`). This replaces the old pair `tx: Option` + `connection_id: Option`, which allowed a fourth, undocumented state: "send failed, `tx` dropped, `connection_id` kept, socket still open".

**Invariants**

- C1: A player has at most one live connection (one session per window).
- C2: A bound connection either receives every update for its player, or is closed. Delivery never silently stops while the socket stays open.
- C3: A socket binds to at most one session.

| From        | Event                                        | To        | Effects                                                                                                                                                                           |
| ----------- | -------------------------------------------- | --------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| socket open | `Connect{token}`                             | bound     | `update_player_connection`; a previous connection is sent `Error{session_superseded}` and closed (C1)                                                                             |
| bound       | `Connect` again on the same socket           | bound     | rejected with `Error{invalid_message}` (C3)                                                                                                                                       |
| bound       | `push_update` fails (channel full or closed) | unbound   | `close.notify_one()`, so the socket task ends and the client reconnects and receives a full snapshot (C2)                                                                         |
| bound       | socket closes, read error, or rate limit     | unbound   | `handle_disconnect` clears the slot only if the connection id matches; the sender task drains queued messages, sends a Close frame, and is aborted after `SHUTDOWN_FLUSH_TIMEOUT` |
| any         | invalid or multibyte UUID in the token       | unchanged | `Error{session_invalid}` (no panic)                                                                                                                                               |

### 1.3 Lobby registry (`server.rs::AppState`, `cleanup_lobbies`)

A lobby is created by unauthenticated `POST /api/create-lobby`, with `round_duration` limited to `10..=600` s. A sweep runs every 60 s: it closes lobbies idle for more than 1 h, then removes closed ones. Joins are refused for closed, full or locked lobbies.

### 1.4 Question store (`question.rs::QuestionStore`)

**States:** a live `ArcSwap<QuestionSnapshot>`. Running lobbies hold their own `Arc` of questions and are isolated from reloads.

**Invariants**

- Q1: Storage never holds data the server cannot start from.
- Q2: Updates are serialised.
- Q3: Readers never see a half-written file.

`replace(data)` runs these steps in order:

1. `build_snapshot(data)`: validate and require at least one playable question, using pure code.
2. Take the `update_lock`.
3. Back up the current file.
4. Write atomically (temp file + rename).
5. Swap in the snapshot that was already built.

Validation failures map to `400 {error, details}`.

---

## 2. Frontend

### 2.1 WebSocket supervisor (`frontend/src/lib/stores/websocket.svelte.ts`)

**States:** `INITIAL`, `CONNECTING`, `CONNECTED`, `RECONNECTING`, `SUSPENDED` (tab hidden), `OFFLINE`, `DISCONNECTED`, `ERROR`. The hidden variables are `desiredConnection`, `sessionToken`, `generationId` (events from older sockets are dropped), `isVisible`, `isOnline` and `pendingConnect`.

**Invariants**

- W1: `connect()` resolves only when the server accepts the session (`Connected`), and rejects on `session_invalid`.
- W2: A dead socket is detected within `HEARTBEAT_INTERVAL + HEARTBEAT_TIMEOUT` (≈7 s) while visible.
- W3: `ERROR` persists, together with its Reconnect action, until the user acts. Visibility and network events do not hide it.
- W4: The UI only commits to an action whose message was actually sent.

| From         | Event                                                                           | Guard                   | To             | Effects                                                                                        |
| ------------ | ------------------------------------------------------------------------------- | ----------------------- | -------------- | ---------------------------------------------------------------------------------------------- |
| any          | `CONNECT_REQUEST`                                                               | hidden                  | `SUSPENDED`    | reject the pending connect                                                                     |
| any          | `CONNECT_REQUEST`                                                               | otherwise               | `CONNECTING`   | open a socket (gen+1) with a 5 s connect timeout                                               |
| `CONNECTING` | `OPEN`                                                                          | —                       | `CONNECTED`    | send `Connect`; start heartbeat and liveness                                                   |
| `CONNECTED`  | `ACCEPTED` (`Connected` message)                                                | —                       | `CONNECTED`    | resolve the pending connect (W1)                                                               |
| any          | `Error{session_invalid}` or `PlayerKicked`                                      | —                       | `DISCONNECTED` | forget the token, reject the pending connect, close; the game store cleans up and returns home |
| any          | `Error{session_superseded}`                                                     | —                       | `ERROR`        | close, no auto-retry; manual Reconnect reclaims the session                                    |
| any          | `GameClosed`                                                                    | —                       | `DISCONNECTED` | forget the token                                                                               |
| live         | `CLOSE`, `ERROR`, `CONNECTION_TIMEOUT`, `HEARTBEAT_TIMEOUT`, `LIVENESS_TIMEOUT` | wanted, visible, online | `RECONNECTING` | backoff of 1–10 s; after 5 attempts → `ERROR`                                                  |
| `ERROR`      | `VISIBILITY_VISIBLE` or `ONLINE`                                                | —                       | `ERROR`        | unchanged (W3)                                                                                 |
| any          | `MANUAL_RECONNECT`                                                              | token present           | `CONNECTING`   | reset the attempt counter                                                                      |
| —            | unparseable message                                                             | —                       | unchanged      | dropped with a warning                                                                         |

Heartbeat: a `0x42` byte is sent every 3 s. The 4 s timeout is armed **only when no echo is outstanding**, and an echo disarms it (W2).

### 2.2 Client game state (`game.svelte.ts`, `player-delta.ts`)

**Phases:** these mirror the server's phases. Phase side effects are applied in `processServerMessage`.

**Invariants**

- G1: Deltas are partial. An absent field means _unchanged_, never zero. A present `scoreboard` is the complete player list (`applyPlayerDelta`).
- G2: Player objects are replaced, never mutated in place.
- G3: `GameOver.final_scores` is authoritative.
- G4: Leaving a game in any way (kicked, closed, invalid session, leave) runs `cleanup`, which also resets the timer.

| Message                                                | Effect                                                                                                    |
| ------------------------------------------------------ | --------------------------------------------------------------------------------------------------------- |
| `Connected`                                            | save the session; set id, name, round duration                                                            |
| `StateDelta`                                           | phase; `applyPlayerDelta`; question; timer start/stop on phase change; `admin_extra` → upcoming questions |
| `Answered`                                             | append to `currentAnswers` (dedup by name)                                                                |
| `PlayerLeft`                                           | remove the player                                                                                         |
| `GameOver`                                             | apply final scores; clear the question and answers; stop the timer                                        |
| `GameClosed`, `PlayerKicked`, `Error{session_invalid}` | `cleanup` → home                                                                                          |
| `AdminInfo`                                            | cue the current video                                                                                     |
| `AdminNextQuestions`                                   | upcoming list (an empty list means out of questions); cue the next video when not mid-question            |

### 2.3 Other client machines

- **Session persistence** (`spektrumSession` in localStorage): saved on `Connected`, removed by `cleanup` or a failed session check. Reconnecting is user-initiated through `ReconnectCard`.
- **Timers** (`timer.svelte.ts`): a game timer that tracks an answer snapshot, and a stream timer. They start on Question and stop on Score or GameOver; `cleanup` resets them.
- **YouTube player** (`youtube-store.svelte.ts`): states are not loaded → not ready (with a pending video) → ready.
- **Stream window** (`broadcast.service.ts`, `stream.store.svelte.ts`): the stream sends `STREAM_READY` every second until the admin replies `INITIAL_STATE`. After that the admin forwards raw server messages. See the open issues F6 and F7.

---

## 3. Admin panel

### 3.1 Data manager (`admin_panel/src/lib/stores/data-manager.svelte.ts`)

**State:** five entity arrays, `loaded`, `snapshots[]`, `currentIndex`, `isBatching` and `preBatchState`.

**Invariants**

- D1: When not batching, the live arrays equal `snapshots[currentIndex].state`.
- D2: Ids are allocated by `addEntity` at insertion time, never pre-computed by callers.
- D3: `setData` starts a fresh history; `loaded` means logged in.
- D4: Entities are replaced, never mutated, so snapshots can share them.

| From     | Event                                       | To           | Effects                                                |
| -------- | ------------------------------------------- | ------------ | ------------------------------------------------------ |
| —        | `setData(data)`                             | idle, loaded | history = [Initial state]                              |
| idle     | `addEntity`, `modifyEntity`, `deleteEntity` | idle         | truncate redo history; push a snapshot (capped at 100) |
| idle     | `startBatch`                                | batching     | remember `preBatchState`                               |
| batching | mutation                                    | batching     | no snapshot                                            |
| batching | `commitBatch`                               | idle         | one snapshot                                           |
| batching | `cancelBatch`                               | idle         | restore `preBatchState`                                |
| idle     | `undo` / `redo`                             | idle         | `currentIndex ∓ 1`; restore arrays                     |

### 3.2 Login (`admin-editor.svelte`, `auth-component.svelte`)

With no `baseVersion` the login form shows; a successful `POST /api/questions` calls `setData`, which stores the data and the version it is at. Errors show the server's reason, and 401 shows "Wrong password". Dev auto-login uses the build-time `PUBLIC_DEV_ADMIN_PASSWORD`, which is stripped from production bundles.

### 3.3 Inline edit (`table/editable-input.svelte`)

The input element holds the draft. Enter blurs, blur commits the value if it changed, and Escape restores the prop value and blurs. There are no parent-side draft maps.

### 3.4 Upload dialog, per file (`character-upload-dialog.svelte`)

`pending → added | invalid(reason) | conflict`, and `conflict → (Overwrite) → added`. A file is `invalid` when it is not AVIF, not 300×300, not loadable, or its name violates the server rule `[A-Za-z0-9_-]+`.

### 3.5 Apply changes (`changes-review.svelte`)

`editing → uploading images (parallel) → POST update-questions{base_version} → success: setData(response with the new version) | error: local state kept, server reason shown`. The server refuses (409) a save whose `base_version` no longer matches the stored data, so a stale editor cannot overwrite newer changes. See open issue A9 for partial failure.

---

## 4. Issue log

Issues are grouped by root cause and ordered by impact. **Status** is _Fixed_ (with its regression test) or _Open_ (needs a decision, listed for review).

### S1 · Connections silently dead while looking alive (critical)

| #   | Sequence                                                                                                                                                                | Consequence                                                           | Status / test                                                                                                  |
| --- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------- | --------------------------------------------------------------------- | -------------------------------------------------------------------------------------------------------------- |
| 1   | A slow or backgrounded client fills its 128-slot channel. The engine dropped `tx` but the socket stayed open, and heartbeats (on a separate channel) kept being echoed. | The player sees a frozen game while the socket looks healthy.         | Fixed: `undeliverable_update_closes_the_connection`                                                            |
| 2   | The same session is opened in two tabs.                                                                                                                                 | The old tab receives nothing, can still act, and is never told.       | Fixed: `newer_connection_supersedes_and_closes_the_older_one`, FSM `superseded session …`, e2e protocol script |
| 3   | `Connect` with another token on a bound socket.                                                                                                                         | One socket bound to two lobbies.                                      | Fixed: rejected (e2e protocol script)                                                                          |
| 4   | Client heartbeat re-armed its 4 s timeout every 3 s.                                                                                                                    | It never fired, so dead sockets were only found after 20–25 s.        | Fixed: FSM `heartbeat timeout fires when echoes stop arriving`                                                 |
| 5   | `ERROR`, then the tab becomes visible or the network returns.                                                                                                           | `DISCONNECTED` hid the Reconnect overlay, leaving a dead game screen. | Fixed: FSM `an error state survives …`                                                                         |
| 6   | A malformed message set `ERROR` directly while timers kept running.                                                                                                     | Inconsistent state.                                                   | Fixed: FSM `an unparseable message is ignored …`                                                               |
| 7   | The rate-limit error was queued, then the sender task was aborted.                                                                                                      | The message was never delivered.                                      | Fixed by graceful drain (no unit test; see `handle_socket`)                                                    |

### S2 · Session invalidity never detected (high)

| #   | Sequence                                                                                                                                  | Consequence                                       | Status / test                                                                                                                                                                      |
| --- | ----------------------------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| 1   | Reconnect after the lobby was removed, or after being kicked while offline. The client matched `ErrorCode` strings the server never sent. | Stuck on a dead game screen showing only a toast. | Fixed: typed `ErrorCode` on `GameUpdate::Error`; FSM `stops reconnecting when the server reports the session invalid`, `connect resolves only once the server accepts the session` |
| 2   | `connect()` resolved on transport open.                                                                                                   | The UI committed before the server accepted.      | Fixed (same tests)                                                                                                                                                                 |

### S3 · Game phase machine holes (high)

| #   | Sequence                                                                                        | Consequence                                                                      | Status / test                                                                                                                                            |
| --- | ----------------------------------------------------------------------------------------------- | -------------------------------------------------------------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------- |
| 1   | `CloseGame` → `EndGame` → `StartGame`.                                                          | A closed lobby was revived, and joins were accepted into closed lobbies.         | Fixed: `closed_game_is_terminal`, `connecting_to_a_closed_game_reports_it_closed`, `test_join_closed_lobby_is_rejected`                                  |
| 2   | `EndGame` from `GameOver`, or mid-question.                                                     | Repeated GameOver; a stale question in the reconnect snapshot.                   | Fixed: `end_game_is_rejected_after_the_game_ended`, `ending_mid_round_clears_the_round`                                                                  |
| 3   | Restarting a set lobby.                                                                         | The second game used every question.                                             | Fixed: `restarting_a_set_lobby_keeps_the_set`                                                                                                            |
| 4   | `StartRound` on an exhausted set.                                                               | The wrong guard passed, round scores were wiped, then the call failed.           | Fixed: `exhausted_set_rejects_start_round_without_touching_round_scores`                                                                                 |
| 5   | `SkipQuestion` past the end.                                                                    | The index grew without bound.                                                    | Fixed: `skip_is_rejected_when_no_questions_remain`                                                                                                       |
| 6   | Last question played.                                                                           | `AdminNextQuestions` was skipped when empty, so "out of questions" never showed. | Fixed: `admin_is_told_when_questions_run_out`                                                                                                            |
| 7   | A kick sent `round_scores: None`; the client read absent as 0.                                  | Everyone's "+N" was wiped.                                                       | Fixed: `kick_keeps_round_scores_in_the_update` + `applyPlayerDelta` tests                                                                                |
| 8   | `GameOver` ignored `final_scores`.                                                              | Wrong winner after ending mid-round.                                             | Fixed: `GameOver › applies the final scores …`                                                                                                           |
| 9   | Mid-game joiners were broadcast only in Lobby, and every reconnect re-broadcast the scoreboard. | Others never saw late joiners; O(N²) messages on mass reconnect.                 | Fixed: broadcast on join; `players_learn_about_mid_game_joiners`                                                                                         |
| 10  | Scoreboard merge `new Map([...old, ...new])`.                                                   | Ghost players were kept forever.                                                 | Fixed: `applyPlayerDelta › treats a scoreboard as the complete player list`                                                                              |
| 11  | `elapsed.as_secs() > duration` truncated.                                                       | Up to 1 s of grace.                                                              | Fixed: `answer_just_past_the_deadline_is_rejected`                                                                                                       |
| 12  | No server round timer.                                                                          | If the admin disconnects mid-question, the round never ends.                     | Fixed: `round_ends_by_itself_when_time_is_up`, `a_timer_that_outlived_its_round_cannot_end_the_next_one`, `round_ends_at_its_deadline_without_the_admin` |

### S4 · Untrusted input crashes or overflows (high)

| #   | Sequence                                                                                                                           | Consequence                                      | Status / test                                                                                 |
| --- | ---------------------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------ | --------------------------------------------------------------------------------------------- |
| 1   | A token with a multibyte character straddling a hex pair.                                                                          | Panic in `Uuid::from_str`, a leaked sender task. | Fixed: `test_from_str_multibyte_input_is_an_error_not_a_panic`, e2e                           |
| 2   | `round_duration` huge.                                                                                                             | Overflow in `* 1000` (wraps in release).         | Fixed: `test_create_lobby_rejects_excessive_duration`                                         |
| 3   | 64 MiB WebSocket messages accepted by default.                                                                                     | Memory pressure.                                 | Fixed: 16 KiB cap                                                                             |
| 4   | No HTTP rate limit (removed in 4ae2bc7); join codes can be enumerated; unauthenticated lobby creation; admin password brute force. | Abuse.                                           | Kept by decision: IP limits caused widespread disconnects at venues where players share an IP |
| 5   | `verify_admin_password` leaks the length through timing.                                                                           | Minor.                                           | **Open**                                                                                      |

### S5 · Admin save pipeline (high)

| #   | Sequence                                  | Consequence                                                               | Status / test                                                                                                                                                  |
| --- | ----------------------------------------- | ------------------------------------------------------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| 1   | Save data with no active question.        | It was written, then the reload failed, so **the server would not boot**. | Fixed: `test_update_that_would_not_load_is_rejected_and_not_written`                                                                                           |
| 2   | Any validation error.                     | Opaque 500.                                                               | Fixed: 400 with details; `test_invalid_update_reports_the_reason`; the admin shows the reason (smoke)                                                          |
| 3   | A concurrent update or a crash mid-write. | Interleaved backup/write; truncated file.                                 | Fixed: mutex + atomic rename; `test_valid_update_is_written_atomically_and_swapped_in`                                                                         |
| 4   | Two admins edit stale copies.             | Last write wins silently.                                                 | Fixed: content-hash `DataVersion`, 409 on a stale base; `save_based_on_an_older_version_is_refused_and_not_written`, `saves_can_chain_on_the_returned_version` |

### A · Admin panel state (medium–high)

| #   | Sequence                                                                                                                | Consequence                                              | Status / test                                                                                                                                                                                                 |
| --- | ----------------------------------------------------------------------------------------------------------------------- | -------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| 1   | Upload a wrong-size or non-AVIF image.                                                                                  | `valid` was ignored, so it was accepted ✓.               | Fixed: explicit per-file machine (§3.4)                                                                                                                                                                       |
| 2   | Empty database, or delete the last media.                                                                               | "Logged in" was derived from `media.length`; locked out. | Fixed: `loaded` flag; `treats loaded data as a fresh start …`; e2e                                                                                                                                            |
| 3   | Re-login.                                                                                                               | A second "Initial state" stacked on the old history.     | Fixed (same test)                                                                                                                                                                                             |
| 4   | Add, undo, open the form, redo, save.                                                                                   | Duplicate id, rejected by the server.                    | Fixed: `allocates ids at insertion …`                                                                                                                                                                         |
| 5   | Escape, then undo.                                                                                                      | The non-reactive draft Map showed a stale value.         | Fixed: drafts live in the input; smoke `inline edit … undo reverts it on screen`                                                                                                                              |
| 6   | CDN configured.                                                                                                         | Pending previews used `${CDN}/data:...`.                 | Fixed: `media.ts::characterImageSrc`                                                                                                                                                                          |
| 7   | The "used" cache never invalidated.                                                                                     | A character became undeletable.                          | Fixed: `$derived` `usedNames`                                                                                                                                                                                 |
| 8   | Bulk import of N lines.                                                                                                 | N history entries, pushing out "Initial state".          | Fixed: one batch; `records a batch as a single undoable step`                                                                                                                                                 |
| 9   | Apply: images uploaded, then the JSON rejected.                                                                         | Orphan images, or live overwrites without JSON.          | **Open** (see R4)                                                                                                                                                                                             |
| 10  | Text "Add option" inserts `''`; Year questions have no option editor; questions can be saved with zero correct options. | Rejected later, or unplayable.                           | Fixed: `question-rules.ts` blocks the save and flags existing rows (`question-rules.test.ts`); a year editor; the server skips unplayable questions (`game_questions_skip_questions_without_a_usable_answer`) |
| 11  | Undo while an input is focused commits the edit first.                                                                  | Undo undoes the wrong thing.                             | **Open** (UX decision)                                                                                                                                                                                        |
| 12  | `max+1` ids reuse the ids of deleted entities.                                                                          | Ids are aliased in external references.                  | **Open**                                                                                                                                                                                                      |

### F · Frontend (medium)

| #   | Sequence                                                        | Consequence                                       | Status / test                                               |
| --- | --------------------------------------------------------------- | ------------------------------------------------- | ----------------------------------------------------------- |
| 1   | An answer is tapped while reconnecting.                         | `send` dropped it but the buttons locked.         | Fixed: `send` returns boolean; FSM `send reports failure …` |
| 2   | Admin reload.                                                   | `admin_extra.upcoming_questions` was ignored.     | Fixed (`setUpcomingQuestions`)                              |
| 3   | A 7-digit join code.                                            | `/join/1234567` was rejected.                     | Fixed: `params.test.ts`                                     |
| 4   | Reconnect overlay countdown.                                    | `Date.now()` in `$derived` never ticked.          | Fixed (ticking `$effect`)                                   |
| 5   | Overlay "Reconnect".                                            | Used `joinGame`, so attempts were not reset.      | Fixed (`manualReconnect`)                                   |
| 6   | Stream window opened mid-question.                              | Full timer, red "wrong" for players who answered. | **Open**                                                    |
| 7   | Admin reload with a stream open; two stream windows.            | The stream froze; one disconnect muted both.      | **Open**                                                    |
| 8   | `Leave` while reconnecting.                                     | The server keeps the player.                      | **Open**                                                    |
| 9   | Join and create show the game screen before `connect` resolves. | Flash of the game screen, then the overlay.       | **Open**                                                    |

### H · Tooling and docs

| #   | Item                                                                                                                                                                                                        | Status                                                                                                                                           |
| --- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------ |
| 1   | Gameplay stress test answered after rounds ended and counted failures as completed.                                                                                                                         | Fixed                                                                                                                                            |
| 2   | The stress throughput mode trips the 30 msg/s limiter, so it measures errors.                                                                                                                               | Fixed: paced by `--rate` (1–30, default 20)                                                                                                      |
| 3   | `converter.py` drops `characters` and targets SQLite; `server/data/schema.sql` is unused.                                                                                                                   | Fixed: both deleted                                                                                                                              |
| 4   | CLAUDE.md: SQLite, rate limit, `just`, pnpm vs npm, `.env` not read by the server.                                                                                                                          | Fixed; README's server setup also told you to create a `.env` the server never read                                                              |
| 6   | No CI and no task runner.                                                                                                                                                                                   | Fixed: root `justfile` (`just check`), `.github/workflows/ci.yml` runs the same recipes plus e2e                                                 |
| 7   | Admin favicon 404; frontend icons and Android icons unreferenced; unused `aioconsole` and `websockets` Python deps.                                                                                         | Fixed: admin favicon, icon links and `site.webmanifest`, deps removed                                                                            |
| 8   | `http.ts`, `utils.ts` and the light switch are duplicated between the two web apps.                                                                                                                         | Kept: about 40 lines, and the `ui/` components are vendored per app by shadcn's design; a shared workspace package would cost more than it saves |
| 5   | README listed 2 of 4 question types; Dockerfile `EXPOSE 8080` vs 8765; dead `ConnectInfo`, `Recipients::{Multiple,_AllExcept}`, `Uuid::{as_bytes,_NIL}`, unused crate features, stale `.gitignore` entries. | Fixed                                                                                                                                            |

---

## 5. Compensating code

These are the places where code existed only to patch over a design flaw. Removed items are gone; remaining items point to the redesign that would remove them.

| Location                                                                       | Compensated for                                        | Root flaw               | Status                                                                          |
| ------------------------------------------------------------------------------ | ------------------------------------------------------ | ----------------------- | ------------------------------------------------------------------------------- |
| Client liveness monitor; forced reconnect after more than 5 s hidden           | the heartbeat timeout never firing                     | broken heartbeat arming | heartbeat fixed; the monitor is now a second line of defence (keep or drop, R3) |
| `isSessionInvalidMessage` prose matching                                       | no machine-readable errors                             | protocol                | **removed** (ErrorCode)                                                         |
| `sessionStatus`                                                                | nothing (always paired with `desiredConnection=false`) | redundant state         | **removed**                                                                     |
| `canReconnect`, `GAME_ENDED`, `lastConnectedAt`, `clearError`                  | —                                                      | dead                    | **removed**                                                                     |
| `endRound`/`endGame`/`closeGame` stopping the timer before the server confirms | timer not tied to server state                         | —                       | **removed** (server phase drives the timer)                                     |
| `lockLobby` "already X" guard                                                  | the server ignores repeats                             | —                       | **removed**                                                                     |
| Kick "should theoretically not happen" branch                                  | find-then-get double lookup                            | —                       | **removed**                                                                     |
| `setup_round` "question not set" re-check                                      | `setup_round` mutated state and could fail midway      | —                       | **removed** (`prepare_next_round` is pure)                                      |
| Two exhaustion checks (one wrong)                                              | no single "remaining" source                           | —                       | **removed** (I4)                                                                |
| Three close paths                                                              | no `close(reason)` transition                          | —                       | **removed**                                                                     |
| `QuestionStore::reload` re-reading storage after a write                       | the snapshot was not built from validated data         | —                       | **removed** (`replace`)                                                         |
| `QuestionError` wrapping `DbError` + duplicate `NoQuestions`                   | —                                                      | —                       | **removed**                                                                     |
| Admin `cloneCharacter` copying `File`s per snapshot                            | fear of shared references                              | `File` is immutable     | **removed**                                                                     |
| Admin `editingValues` maps, `getEditingValue`, `commitChanges`                 | a parent-held draft duplicating the input              | —                       | **removed**                                                                     |
| Admin hover-gated `cachedUsage`                                                | an O(n) scan on hover                                  | no derived usage        | **removed**                                                                     |
| Upload `error.includes('exists')` string state                                 | the status union was not used                          | —                       | **removed**                                                                     |
| `_pendingImage` stripped by hand in two places                                 | client-only data inside domain entities                | —                       | centralised in `exportData`; full fix R5                                        |
| `Math.max(answered, total)` denominators                                       | the client player list was not authoritative           | —                       | root fixed (G1); the denominators are now merely defensive (**Open**: simplify) |
| `currentQuestionKey` in the question view; `Answered` dedup                    | no round id or sequence numbers in the protocol        | —                       | **Open** (R1)                                                                   |
| `STREAM_READY` resent every second; polling `window.closed`                    | no stream handshake or ownership                       | —                       | **Open** (R6)                                                                   |
| YouTube play retry ×3                                                          | autoplay races                                         | —                       | **Open**                                                                        |
| `check-sessions` scanning every lobby per session                              | the request carries only `player_id`                   | send the full token     | **Open**                                                                        |
| Admin `media` delete blocked while the store cascades deletes                  | two referential-integrity policies                     | —                       | **Open** (R5)                                                                   |

---

## 6. Redesigns from first principles

Each redesign states the principle, the code it would delete, and its cost.

- **R1 · Versioned snapshots instead of hand-applied deltas.** The server keeps a monotonically increasing `version` per lobby and sends `{version, state}` snapshots, or deltas tagged with `base_version`. A client that sees a gap asks for a snapshot.
  - Would delete: `applyPlayerDelta`'s merge rules, `currentQuestionKey`, `Answered` dedup, the "absent means unchanged" convention, and the whole class of bugs fixed in S3.7, S3.8 and S3.10.
  - Cost: slightly larger messages, and a protocol change on both sides.

- **R2 · Server-owned round timer. Done.** The round's data is one `Option<Round>` (I2), and a timer task per round ends it at the deadline.
  - Deleted: the "no start time → half duration" fallback, `PreparedRound`, the four parallel round fields, and stuck rounds when the admin disconnects (S3.12).
  - Not done: moving `Round` into a `GamePhase::Question(Round)` variant. `GamePhase` doubles as the wire enum, so that needs a split between the engine's phase and the protocol's.
  - Cost: a timer task per lobby, plus a decision on whether the admin or the clock ends rounds.

- **R3 · One liveness mechanism.** Protocol-level ping/pong with a server read timeout, plus a client heartbeat whose timeout is the only detector.
  - Would delete: the liveness monitor and the forced reconnect after 5 s hidden, which also triggered the O(N²) reconnect storms.

- **R4 · Content-addressed, transactional publishing.**
  - Images are uploaded to `img/<sha256>.avif` (immutable, cache-forever) and the JSON references hashes. Publishing becomes one request carrying `base_version`, and the server rejects stale bases (409).
  - Would delete: orphaned or overwritten live images (A9) and cache busting (2.16). The `base_version` half is done (S5.4).
  - Cost: a storage layout migration.

- **R5 · Separate draft state from domain state in the admin.**
  - Pending uploads live in a `Map<characterName, File>` beside the entities, and referential integrity (cascade or block) is one policy in the store. Options reference characters by id rather than by `option_text`.
  - Would delete: `_pendingImage` stripping, the two delete policies, name-based cascades, and the rename impossibility.

- **R6 · Stream window as a server client.** The stream connects with a read-only "spectator" token, instead of relaying through BroadcastChannel from the admin tab.
  - Would delete: `STREAM_READY` polling, `hasActiveStreams`, snapshot synthesis (F6), and admin-reload freezes (F7).
  - Cost: a new token type on the server.

- **R7 · Generated protocol types. Done.** The wire types derive `ts_rs::TS`, and `server/src/bindings.rs` checks the generated files into `frontend/src/lib/generated` and `admin_panel/src/lib/generated`. `cargo test` fails while they are stale and rewrites them, so committing the result fixes it.
  - Deleted: the hand-mirrored protocol enums and unions in `types/game.ts`, the admin's mirrored entity types, `GameQuestion::get_question_type` (StateDelta now carries `QuestionType`, not a string), the `question_type ?? ''` fallbacks, and every untyped `response.json()`. HTTP calls go through typed `api.ts` modules in both clients.
  - Left: client-only state types, and the admin's `QUESTION_TYPES`/`COLORS` lists, which need runtime values. A `satisfies Record<…>` keeps those lists exhaustive.
