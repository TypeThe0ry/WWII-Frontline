# frontline-protocol changelog

`PROTOCOL_VERSION` must match exactly at handshake. Fields are only appended at the end of a
message; any other change bumps the version.

## 2 — M1-010 (phase A: C0 + C4 + extra events)

Breaking: version bumped 1 -> 2 (new `msg_type`s and event types; `hello.hex` golden changed).

### C0 (reliable, both directions share one `msg_type` numbering)

Strings are `len:8 | UTF-8 bytes`; limits are in **bytes**; invalid UTF-8 is `InvalidValue`,
over-limit is `TooLong`.

| type | message | direction | fields (bits) |
|---|---|---|---|
| 0x10 | `DeployRequest` | C->S | spawn_point 16, role 8, loadout 8, squad 8 |
| 0x11 | `DeployResult` | S->C | ok 1, spawn_tick 32, reason 8 |
| 0x12 | `SquadOp` | C->S | op 8 (1 create, 2 join, 3 leave, 4 kick, 5 promote, 6 rename, **7 lock, 8 unlock, 9 invite** — M1 additions, GD §4.3), squad 8, target 8, name_key str(<=32) |
| 0x13 | `Chat` | both | channel 8 (0 all, 1 team, 2 squad, 3 command), text str(<=200), **sender 8** (S->C: originating player; ignored C->S) |
| 0x14 | `MarkerPlace` | C->S | kind 8, pos 2x16, ttl 8 |
| 0x15 | `Settings` | C->S | view_dist_m 16, flags 8 (bit0 mute proximity voice) |
| 0x16 | `Report` | C->S | target 8, reason 8, note str(<=120) |

`Disconnect` (0x06) already existed. `CommanderOrder`, `VoiceState` stay for M2/M3.

### C4 (reliable server events; `msg_type:8` then fields)

| type | event | fields (bits) |
|---|---|---|
| 0x01 | `Kill` | victim 8, killer 8 (0xFF env), weapon 8, headshot 1, dist_q 12 (0.25 m), assist1 8, assist2 8 |
| 0x02 | `Death` | victim 8, respawn_after 8 (s), can_be_revived 1 |
| 0x03 | `Revive` | medic 8, target 8 |
| 0x04 | `ScoreDelta` | player 8, kind 8, value i16 |
| 0x05 | `ObjectiveChange` | objective 8, owner 8, reason 8 |
| 0x06 | `SpawnPointState` | id 16, kind 8 (0 garrison, 1 outpost, 2 airhead, 3 op, 4 vehicle, 5 boat), team 8, pos 2x16, state 8 |
| 0x07 | `PlayerRoster` | player 8, nick str(<=24), team 8, squad 8, role 8, flags 8 (1 downed, 2 commander, 4 squad leader, 8 left, 16 bot) |
| 0x08 | `Notice` | code 16, args 3x i32 |
| 0x09 | `MatchPhase` | phase 8 (0 warmup, 1 live, 2 overtime, 3 ended), ends_at 32 |

### C2 event segment (`type:5`)

| type | event | payload (bits) |
|---|---|---|
| 5 | `Explosion` | pos 3x14, kind 8, radius_q 8 |
| 6 | `Suppress` | source_dir 8, intensity 8 |
| 7 | `Marker` | kind 8, pos 2x14 (x,z), ttl 8 (server quantises enemy marks to 5 m, <=1 Hz) |

## 1 — M0
Initial: Hello/Welcome/Reject/Ping/Pong/Disconnect, InputBundle/FireCmd, snapshots with events 1–4.
