# frontline-protocol changelog

`PROTOCOL_VERSION` must match exactly at handshake. Fields are only appended at the end of a
message; any other change bumps the version.

## 11 — M2-019 comms wire prerequisites

Version 11 includes the M2-018 messages from version 5 with their existing tags and layouts.

* C0 `0x13` `Chat` (both directions) uses `channel:8 | byte_count:10 | UTF-8 text | sender:8`.
  Chat accepts up to 200 Unicode scalar values and 800 UTF-8 bytes.
* C0 `0x1e` `PingPlace` (C→S) adds `kind:8 | x:16 | z:16 | advisory_target:16`.
  Position uses the existing `MarkerPlace` map-normalized x/z encoding.
* C0 `0x1f` `Notice` (S→C) carries `len:8 | UTF-8 i18n key` (1–64 bytes).
* C2 event types 10 and 11 upsert and remove markers by ID. Upserts carry map-relative
  5 m cells, remaining whole seconds (1–254), and an owner. TTL 255 means until revoked
  for command attack/defend. Only Thanks has a target field, which
  can contain a server-verified friendly ID. Enemy marker payloads have no target field.
* Manual marker tags 8–19 extend the existing set. The optional periodic C3 projection
  reuses the existing `TeamStateWithMarkers` layout.

## 5 — M2-018 subset of M2-007 (server rules)

| Channel and type | Message | Fields (bits) |
|---|---|---|
| C0 0x19 | `VoteStart` | kind 8 (0 kick, 1 remove commander, 2 mute), target 8 |
| C0 0x1a | `VoteCast` | vote_id 32, yes 1 |
| C0 0x1b | `TeamSwitchRequest` | team 8 (0 US, 1 GER), accept_offer 1 |
| C0 0x1c | `ForgivePunish` | victim_event 16, punish 1 |
| C0 0x1d | `MapVoteCast` | vote_id 32, choice 8 (candidate index 0..2) |
| C4 0x0a | `VoteState` | vote_id 32, kind 8, target 8, yes 8, no 8, ends_in_s 8 |
| C4 0x0b | `VoteResult` | vote_id 32, passed 1 |
| C4 0x0c | `TkPrompt` | killer 8, expires_s 8, victim_event 16 (echo in `ForgivePunish`) |
| C4 0x0d | `TeamBalanceNotice` | code 8 (`TeamBalanceCode`), args 2x i32 |
| C4 0x0e | `MapVoteState` | vote_id 32, candidates 3x8 (map IDs), votes 3x8, ends_in_s 8 |
| C4 0x0f | `MapVoteResult` | vote_id 32, winner 8 (map ID) |

Vote IDs keep the simulation's 32-bit width so an old vote ID cannot alias after 256 votes.
`Disconnect.reason = 5` is an automatic server-rule removal. Existing message tags and
layouts are unchanged other than the new version value in `Hello`.

## 4 — M1-041d (own vehicle seat)

C2 `SelfState` reports `vehicle_slot` (128..=255, 0 on foot) and `seat` (0 driver, 1 gunner,
2 commander). On-foot bits are unchanged. Seated packets use new position tags and repurpose
the 11-bit deployment/cooldown tail: the deployment state is 0 and only the 5 defined cooldown
bits are sent. Both forms remain 56 bits without and 101 bits with a position correction.
Protocol 3 decoders reject the new tags; peers must upgrade together.

## 3 — M1-010b (vehicles, team channel)

Breaking vs 2: the entity record tag changed (remove is now `id:8 000`, 11 bit) to make room for
vehicle keyframes; `EntityKind::Vehicle`, `QVel`, `VehicleExtra` are now on the wire.

### Snapshot entity records

| tag after `id:8` | record |
|---|---|
| `1` | delta (`field_mask:8`, interpreted by the baseline entity's kind) |
| `01` | player keyframe (unchanged, 100 bit) |
| `001` | **vehicle keyframe**, 180 bit: pos 16/13/16, quat 32 (smallest-three: index 2 + 3x i10), vel 3x i11 (1/64 m/s), turret_yaw 12, gun_pitch 12, track_flags 8, seats 3 (bitmap), damage_flags 8, flags 8, health_q 8 |
| `000` | remove |

Vehicle delta mask: 1 pos, 2 quat, 4 vel, 8 turret (yaw12+pitch12), 16 crew (track_flags 8 + seats 3), 32 health_q, 64 damage_flags, 128 flags. Position delta uses the player size classes. Measured: avg 15.8 B, max 20 B over 10^4 snapshots (budget 24 / 28).
Accuracy: quaternion <= 0.35 deg, turret yaw / gun pitch <= 0.35 deg (12 bit).

### Team channel (C3) and world state (`msg::team`)

* `TeamState`: `tick:16, n:6 (<=49), n x {x:11 z:11 role:4 state:3 facing:3}`; 49 members = 199 B. No ids on the wire: entry i is the i-th team member by ascending player_id (roster from C4); `state == 0` = absent/dead.
* `WorldState` (1 Hz): `timer:16, 5 x {owner:2 capture:7}, manpower 16, munitions 16, fuel 16, front_line_pos 16` = 16 B.

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
