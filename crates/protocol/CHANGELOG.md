# frontline-protocol changelog

`PROTOCOL_VERSION` must match exactly at handshake. Fields are only appended at the end of a
message; any other change bumps the version.

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
