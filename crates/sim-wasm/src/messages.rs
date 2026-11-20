//! Typed message encoders/decoders for C0 control and C1 input (M0-005 types).

use arrayvec::ArrayVec;
use frontline_protocol::bitio::{BitReader, BitWriter};
use frontline_protocol::channel::c0_type;
use frontline_protocol::codec::Codec;
use frontline_protocol::msg::input::{
    decode_input_bundle, encode_input_bundle, FireCmd, InputBundle, InputCmd, MAX_BUNDLE_CMDS,
    MAX_BUNDLE_FIRE,
};
use frontline_protocol::msg::{
    decode_c0_server, decode_team_state_with_markers, encode_c0_client, Chat, ChatChannel,
    ClientMsgC0, Disconnect, ForgivePunish, Hello, MapVoteCast, Ping, PingKind, PingPlace,
    ServerMsgC0, TeamSwitchRequest, VoteCast, VoteKind, VoteStart,
};
use wasm_bindgen::prelude::*;

use crate::js_err;
use crate::predictor::{parse_cmd, INPUT_CMD_BYTES};

pub fn layout_table() -> &'static [(&'static str, u32)] {
    &[
        ("C0_HELLO", c0_type::HELLO as u32),
        ("C0_WELCOME", c0_type::WELCOME as u32),
        ("C0_REJECT", c0_type::REJECT as u32),
        ("C0_PING", c0_type::PING as u32),
        ("C0_PONG", c0_type::PONG as u32),
        ("C0_DISCONNECT", c0_type::DISCONNECT as u32),
        ("C0_DEPLOY_RESULT", c0_type::DEPLOY_RESULT as u32),
        ("FIRE_CMD_BYTES", FIRE_CMD_BYTES as u32),
        ("MAX_BUNDLE_CMDS", MAX_BUNDLE_CMDS as u32),
        ("MAX_BUNDLE_FIRE", MAX_BUNDLE_FIRE as u32),
    ]
}

/// Bytes of one full-width `FireCmd` (`packFireCmd`): 16+16+8+16+16+3*14+16 = 130 bits.
pub const FIRE_CMD_BYTES: usize = 17;

pub(crate) fn whole<T: Codec>(m: &T) -> Result<Vec<u8>, String> {
    let mut w = BitWriter::new();
    m.encode(&mut w).map_err(|e| e.to_string())?;
    Ok(w.finish())
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn pack_input_cmd_core(
    seq: u16,
    tick: u32,
    move_x: i8,
    move_z: i8,
    yaw: u16,
    pitch: i16,
    buttons: u16,
    weapon_slot: u8,
    aux: u8,
    view_tick: u16,
) -> Result<Vec<u8>, String> {
    let c = InputCmd {
        seq,
        tick,
        move_x,
        move_z,
        yaw,
        pitch,
        buttons,
        weapon_slot,
        aux,
        view_tick,
    };
    let b = whole(&c)?;
    debug_assert_eq!(b.len(), INPUT_CMD_BYTES);
    Ok(b)
}

fn small<T: TryFrom<i64>>(v: i32, what: &str) -> Result<T, String> {
    T::try_from(i64::from(v)).map_err(|_| format!("{what} out of range: {v}"))
}

/// `InputCmd` -> 18 bytes (the argument of `Predictor.step`).
#[wasm_bindgen(js_name = packInputCmd)]
#[allow(clippy::too_many_arguments)]
pub fn pack_input_cmd(
    seq: u32,
    tick: u32,
    move_x: i32,
    move_z: i32,
    yaw: u32,
    pitch: i32,
    buttons: u32,
    weapon_slot: u32,
    aux: u32,
    view_tick: u32,
) -> Result<Vec<u8>, JsError> {
    (|| {
        pack_input_cmd_core(
            small(seq as i32, "seq")?,
            tick,
            small(move_x, "moveX")?,
            small(move_z, "moveZ")?,
            small(yaw as i32, "yaw")?,
            small(pitch, "pitch")?,
            small(buttons as i32, "buttons")?,
            small(weapon_slot as i32, "weaponSlot")?,
            small(aux as i32, "aux")?,
            small(view_tick as i32, "viewTick")?,
        )
    })()
    .map_err(js_err)
}

/// Same encoding as [`pack_input_cmd`] (kept as a separate name for the §8.3 vocabulary).
#[wasm_bindgen(js_name = encodeInputCmd)]
#[allow(clippy::too_many_arguments)]
pub fn encode_input_cmd(
    seq: u32,
    tick: u32,
    move_x: i32,
    move_z: i32,
    yaw: u32,
    pitch: i32,
    buttons: u32,
    weapon_slot: u32,
    aux: u32,
    view_tick: u32,
) -> Result<Vec<u8>, JsError> {
    pack_input_cmd(
        seq,
        tick,
        move_x,
        move_z,
        yaw,
        pitch,
        buttons,
        weapon_slot,
        aux,
        view_tick,
    )
}

/// `FireCmd` -> 17 bytes (`origin_hint` entries are 14-bit codes).
#[wasm_bindgen(js_name = packFireCmd)]
#[allow(clippy::too_many_arguments)]
pub fn pack_fire_cmd(
    seq: u32,
    view_tick: u32,
    weapon: u32,
    dir_yaw: u32,
    dir_pitch: u32,
    origin_x: u32,
    origin_y: u32,
    origin_z: u32,
    spread_seed: u32,
) -> Result<Vec<u8>, JsError> {
    (|| {
        let f = FireCmd {
            seq: small(seq as i32, "seq")?,
            view_tick: small(view_tick as i32, "viewTick")?,
            weapon: small(weapon as i32, "weapon")?,
            dir_yaw: small(dir_yaw as i32, "dirYaw")?,
            dir_pitch: small(dir_pitch as i32, "dirPitch")?,
            origin_hint: [
                small(origin_x as i32, "originX")?,
                small(origin_y as i32, "originY")?,
                small(origin_z as i32, "originZ")?,
            ],
            spread_seed: small(spread_seed as i32, "spreadSeed")?,
        };
        whole(&f)
    })()
    .map_err(js_err)
}

pub(crate) fn bundle_core(
    cmds: &[u8],
    fire: &[u8],
    ack_tick: u32,
    pred_hash: u32,
) -> Result<Vec<u8>, String> {
    if cmds.is_empty() || !cmds.len().is_multiple_of(INPUT_CMD_BYTES) {
        return Err(format!(
            "cmds must be 1..={MAX_BUNDLE_CMDS} x {INPUT_CMD_BYTES} bytes, got {}",
            cmds.len()
        ));
    }
    if !fire.len().is_multiple_of(FIRE_CMD_BYTES) {
        return Err(format!("fire must be a multiple of {FIRE_CMD_BYTES} bytes"));
    }
    let mut list: ArrayVec<InputCmd, MAX_BUNDLE_CMDS> = ArrayVec::new();
    for chunk in cmds.chunks(INPUT_CMD_BYTES) {
        list.try_push(parse_cmd(chunk)?)
            .map_err(|_| format!("at most {MAX_BUNDLE_CMDS} commands per bundle"))?;
    }
    let mut fl: ArrayVec<FireCmd, MAX_BUNDLE_FIRE> = ArrayVec::new();
    for chunk in fire.chunks(FIRE_CMD_BYTES) {
        let mut r = BitReader::new(chunk);
        let f = FireCmd::decode(&mut r).map_err(|e| format!("bad fire command: {e}"))?;
        r.finish().map_err(|e| format!("bad fire command: {e}"))?;
        fl.try_push(f)
            .map_err(|_| format!("at most {MAX_BUNDLE_FIRE} fire commands per bundle"))?;
    }
    let b = InputBundle {
        ack_snapshot_tick: u16::try_from(ack_tick).map_err(|_| "ackTick out of range (u16)")?,
        pred_hash: u8::try_from(pred_hash).map_err(|_| "predHash out of range (u8)")?,
        cmds: list,
        fire: fl,
    };
    encode_input_bundle(&b).map_err(|e| e.to_string())
}

/// `cmds`: 1..=3 consecutive `packInputCmd` outputs back to back; `fire`: 0..=2 `packFireCmd`
/// outputs. `ackTick` is the u16 snapshot-tick delta, `predHash` the 8-bit prediction hash.
#[wasm_bindgen(js_name = encodeInputBundle)]
pub fn encode_input_bundle_js(
    cmds: &[u8],
    ack_tick: u32,
    pred_hash: u32,
    fire: Option<Vec<u8>>,
) -> Result<Vec<u8>, JsError> {
    bundle_core(cmds, fire.as_deref().unwrap_or(&[]), ack_tick, pred_hash).map_err(js_err)
}

/// Server-side decode of a bundle, returning the number of commands (test helper for the
/// round trip; the server proper is native).
#[wasm_bindgen(js_name = inputBundleLen)]
pub fn input_bundle_len(bytes: &[u8]) -> Result<u32, JsError> {
    decode_input_bundle(bytes)
        .map(|b| b.cmds.len() as u32)
        .map_err(js_err)
}

#[wasm_bindgen(js_name = encodeHello)]
pub fn encode_hello(
    protocol_version: u32,
    data_hash: u64,
    ticket: &[u8],
    client_build: u32,
    caps: u32,
) -> Result<Vec<u8>, JsError> {
    (|| {
        let m = ClientMsgC0::Hello(Hello {
            protocol_version: small(protocol_version as i32, "protocolVersion")?,
            data_hash,
            ticket: ticket.to_vec(),
            client_build,
            caps: small(caps as i32, "caps")?,
        });
        encode_c0_client(&m).map_err(|e| e.to_string())
    })()
    .map_err(js_err)
}

#[wasm_bindgen(js_name = encodePing)]
pub fn encode_ping(client_time: u32) -> Result<Vec<u8>, JsError> {
    encode_c0_client(&ClientMsgC0::Ping(Ping { client_time })).map_err(js_err)
}

#[wasm_bindgen(js_name = encodeDisconnect)]
pub fn encode_disconnect(reason: u32) -> Result<Vec<u8>, JsError> {
    let r = u8::try_from(reason).map_err(|_| js_err("reason out of range"))?;
    encode_c0_client(&ClientMsgC0::Disconnect(Disconnect { reason: r })).map_err(js_err)
}

fn rule_int(value: f64, max: u32, field: &str) -> Result<u32, JsError> {
    // Keep the full JS number at the boundary so wasm-bindgen doesn't truncate/wrap it first.
    if (0.0..=f64::from(max)).contains(&value) && value.fract() == 0.0 {
        Ok(value as u32)
    } else {
        Err(js_err(format!("{field} out of range: {value}")))
    }
}

fn rule_byte(value: f64, field: &str) -> Result<u8, JsError> {
    Ok(rule_int(value, u32::from(u8::MAX), field)? as u8)
}

#[wasm_bindgen(js_name = encodeVoteStart)]
pub fn encode_vote_start(kind: f64, target: f64) -> Result<Vec<u8>, JsError> {
    let kind = VoteKind::from_u8(rule_byte(kind, "kind")?)
        .ok_or_else(|| js_err("vote kind out of range"))?;
    encode_c0_client(&ClientMsgC0::VoteStart(VoteStart {
        kind,
        target: rule_byte(target, "target")?,
    }))
    .map_err(js_err)
}

#[wasm_bindgen(js_name = encodeVoteCast)]
pub fn encode_vote_cast(vote_id: f64, yes: bool) -> Result<Vec<u8>, JsError> {
    encode_c0_client(&ClientMsgC0::VoteCast(VoteCast {
        vote_id: rule_int(vote_id, u32::MAX, "voteId")?,
        yes,
    }))
    .map_err(js_err)
}

#[wasm_bindgen(js_name = encodeTeamSwitchRequest)]
pub fn encode_team_switch_request(team: f64, accept_offer: bool) -> Result<Vec<u8>, JsError> {
    encode_c0_client(&ClientMsgC0::TeamSwitchRequest(TeamSwitchRequest {
        team: rule_byte(team, "team")?,
        accept_offer,
    }))
    .map_err(js_err)
}

#[wasm_bindgen(js_name = encodeForgivePunish)]
pub fn encode_forgive_punish(victim_event: f64, punish: bool) -> Result<Vec<u8>, JsError> {
    encode_c0_client(&ClientMsgC0::ForgivePunish(ForgivePunish {
        victim_event: rule_int(victim_event, u32::from(u16::MAX), "victimEvent")? as u16,
        punish,
    }))
    .map_err(js_err)
}

#[wasm_bindgen(js_name = encodeMapVoteCast)]
pub fn encode_map_vote_cast(vote_id: f64, choice: f64) -> Result<Vec<u8>, JsError> {
    encode_c0_client(&ClientMsgC0::MapVoteCast(MapVoteCast {
        vote_id: rule_int(vote_id, u32::MAX, "voteId")?,
        choice: rule_byte(choice, "choice")?,
    }))
    .map_err(js_err)
}

/// C0 Chat. The server sets the sender when it broadcasts the message.
#[wasm_bindgen(js_name = encodeChat)]
pub fn encode_chat(channel: u32, text: String) -> Result<Vec<u8>, JsError> {
    let channel = u8::try_from(channel)
        .ok()
        .and_then(ChatChannel::from_u8)
        .ok_or_else(|| JsError::new("invalid chat channel"))?;
    encode_c0_client(&ClientMsgC0::Chat(Chat {
        channel,
        text,
        sender: 0,
    }))
    .map_err(js_err)
}

/// C0 PingPlace; the target is an advisory hint only.
#[wasm_bindgen(js_name = encodePingPlace)]
pub fn encode_ping_place(kind: u8, x: u16, z: u16, target: u16) -> Result<Vec<u8>, JsError> {
    let kind = PingKind::from_u8(kind).ok_or_else(|| JsError::new("invalid ping kind"))?;
    encode_c0_client(&ClientMsgC0::PingPlace(PingPlace {
        kind,
        pos: [x, z],
        target,
    }))
    .map_err(js_err)
}

/// C3 team state (legacy prefix with optional markers) as `[tick, kind, x, z, ttl, ...]`.
/// Member state is validated by the protocol codec and omitted from this view.
#[wasm_bindgen(js_name = decodeTeamMarkers)]
pub fn decode_team_markers(bytes: &[u8]) -> Result<Vec<u16>, JsError> {
    let state = decode_team_state_with_markers(bytes).map_err(js_err)?;
    let mut out = Vec::with_capacity(1 + state.markers.len() * 4);
    out.push(state.team.tick);
    for marker in state.markers {
        out.extend([
            u16::from(marker.kind as u8),
            marker.x,
            marker.z,
            u16::from(marker.ttl_s),
        ]);
    }
    Ok(out)
}

/// Thin §8.3 wrapper: `[kind, ...payload]` validated as a client C0 message.
#[wasm_bindgen(js_name = encodeC0)]
pub fn encode_c0(kind: u32, payload: &[u8]) -> Result<Vec<u8>, JsError> {
    let mut v = Vec::with_capacity(payload.len() + 1);
    v.push(u8::try_from(kind).map_err(|_| js_err("kind out of range"))?);
    v.extend_from_slice(payload);
    frontline_protocol::msg::decode_c0_client(&v).map_err(js_err)?;
    Ok(v)
}

/// A decoded server C0 message. Fields unused by `kind` retain their defaults.
#[wasm_bindgen]
#[derive(Debug, Clone, Default)]
pub struct ServerC0 {
    /// Server C0 message type.
    pub kind: u8,
    // Welcome
    #[wasm_bindgen(js_name = playerId)]
    pub player_id: u8,
    #[wasm_bindgen(js_name = serverTick)]
    pub server_tick: u32,
    #[wasm_bindgen(js_name = tickHz)]
    pub tick_hz: u8,
    #[wasm_bindgen(js_name = matchSeed)]
    pub match_seed: u64,
    #[wasm_bindgen(js_name = dataHash)]
    pub data_hash: u64,
    #[wasm_bindgen(js_name = mapId)]
    pub map_id: u8,
    pub mode: u8,
    pub team: u8,
    #[wasm_bindgen(js_name = clientTickOffset)]
    pub client_tick_offset: i16,
    // Reject / Disconnect / DeployResult
    pub reason: u8,
    // DeployResult
    pub ok: bool,
    #[wasm_bindgen(js_name = spawnTick)]
    pub spawn_tick: u32,
    // Pong
    #[wasm_bindgen(js_name = clientTime)]
    pub client_time: u32,
    #[wasm_bindgen(js_name = serverTimeMs)]
    pub server_time_ms: u32,
    notice_key: String,
    #[wasm_bindgen(js_name = chatChannel)]
    pub chat_channel: u8,
    #[wasm_bindgen(js_name = chatSender)]
    pub chat_sender: u8,
    chat_text: String,
}

#[wasm_bindgen]
impl ServerC0 {
    #[wasm_bindgen(getter, js_name = noticeKey)]
    pub fn notice_key(&self) -> String {
        self.notice_key.clone()
    }

    #[wasm_bindgen(getter, js_name = chatText)]
    pub fn chat_text(&self) -> String {
        self.chat_text.clone()
    }
}

pub(crate) fn decode_server_core(bytes: &[u8]) -> Result<ServerC0, String> {
    let m = decode_c0_server(bytes).map_err(|e| format!("bad server C0 message: {e}"))?;
    let mut o = ServerC0 {
        kind: m.msg_type(),
        ..Default::default()
    };
    match m {
        ServerMsgC0::Welcome(w) => {
            o.player_id = w.player_id;
            o.server_tick = w.server_tick;
            o.tick_hz = w.tick_hz;
            o.match_seed = w.match_seed;
            o.data_hash = w.data_hash;
            o.map_id = w.map_id;
            o.mode = w.mode;
            o.team = w.team;
            o.client_tick_offset = w.client_tick_offset;
        }
        ServerMsgC0::Reject(r) => o.reason = r.reason as u8,
        ServerMsgC0::Pong(p) => {
            o.client_time = p.client_time;
            o.server_tick = p.server_tick;
            o.server_time_ms = p.server_time_ms;
        }
        ServerMsgC0::Disconnect(d) => o.reason = d.reason,
        ServerMsgC0::DeployResult(d) => {
            o.ok = d.ok;
            o.reason = d.reason;
            o.spawn_tick = d.spawn_tick;
        }
        ServerMsgC0::Notice(n) => o.notice_key = n.key,
        ServerMsgC0::Chat(chat) => {
            o.chat_channel = chat.channel as u8;
            o.chat_sender = chat.sender;
            o.chat_text = chat.text;
        }
        // Placement's signed positions are handled by its dedicated TS C0 adapter.
        ServerMsgC0::PlacementResult(_) => {}
    }
    Ok(o)
}

#[wasm_bindgen(js_name = decodeServerC0)]
pub fn decode_server_c0(bytes: &[u8]) -> Result<ServerC0, JsError> {
    decode_server_core(bytes).map_err(js_err)
}

/// §8.3 name for [`decode_server_c0`].
#[wasm_bindgen(js_name = decodeC0)]
pub fn decode_c0(bytes: &[u8]) -> Result<ServerC0, JsError> {
    decode_server_c0(bytes)
}

/// JSON string literal (quotes, backslash, control characters escaped) for a nickname.
fn json_str(s: &str) -> String {
    let mut o = String::with_capacity(s.len() + 2);
    o.push('"');
    for c in s.chars() {
        match c {
            '"' => o.push_str("\\\""),
            '\\' => o.push_str("\\\\"),
            c if (c as u32) < 0x20 => o.push_str(&format!("\\u{:04x}", c as u32)),
            c => o.push(c),
        }
    }
    o.push('"');
    o
}

/// A C4 event as one JSON object whose `type` and field names are the TS `C4Event` of
/// `client/src/game/state/c4.ts` (M1-040c). The roster entry carries no `seq`: the receiver numbers them.
pub(crate) fn decode_c4_core(bytes: &[u8]) -> Result<String, String> {
    use frontline_protocol::msg::c4::{decode_c4, roster_flags, ServerEventC4 as E};
    let ev = decode_c4(bytes).map_err(|e| format!("bad C4 event: {e}"))?;
    Ok(match ev {
        E::Kill(k) => format!(
            r#"{{"type":"kill","victim":{},"killer":{},"weapon":{},"headshot":{},"distQ":{}}}"#,
            k.victim, k.killer, k.weapon, k.headshot, k.dist_q
        ),
        E::Death(d) => format!(
            r#"{{"type":"death","victim":{},"respawnAfter":{},"canBeRevived":{}}}"#,
            d.victim, d.respawn_after, d.can_be_revived
        ),
        E::Revive(r) => format!(
            r#"{{"type":"revive","medic":{},"target":{}}}"#,
            r.medic, r.target
        ),
        E::ScoreDelta(s) => format!(
            r#"{{"type":"score","player":{},"kind":{},"category":{},"reason":{},"value":{}}}"#,
            s.player,
            s.kind,
            s.category() as u8,
            s.reason(),
            s.value
        ),
        E::ObjectiveChange(o) => format!(
            r#"{{"type":"objective","objective":{},"owner":{},"reason":{}}}"#,
            o.objective, o.owner, o.reason
        ),
        E::SpawnPointState(p) => format!(
            r#"{{"type":"spawnpoint","id":{},"kind":{},"team":{},"pos":[{},{}],"state":{}}}"#,
            p.id, p.kind as u8, p.team, p.pos[0], p.pos[1], p.state
        ),
        E::PlayerRoster(r) => format!(
            r#"{{"type":"roster","player":{},"nick":{},"team":{},"squad":{},"role":{},"flags":{},"left":{}}}"#,
            r.player,
            json_str(&r.nick),
            r.team,
            r.squad,
            r.role,
            r.flags,
            r.flags & roster_flags::LEFT != 0
        ),
        E::Notice(n) => format!(
            r#"{{"type":"notice","code":{},"args":[{},{},{}]}}"#,
            n.code, n.args[0], n.args[1], n.args[2]
        ),
        E::MatchPhase(m) => format!(
            r#"{{"type":"phase","phase":{},"endsAt":{}}}"#,
            m.phase as u8, m.ends_at
        ),
        E::VoteState(v) => format!(
            r#"{{"type":"voteState","voteId":{},"kind":{},"target":{},"yes":{},"no":{},"endsInS":{}}}"#,
            v.vote_id, v.kind as u8, v.target, v.yes, v.no, v.ends_in_s
        ),
        E::VoteResult(v) => format!(
            r#"{{"type":"voteResult","voteId":{},"passed":{}}}"#,
            v.vote_id, v.passed
        ),
        E::TkPrompt(p) => format!(
            r#"{{"type":"tkPrompt","killer":{},"expiresS":{},"victimEvent":{}}}"#,
            p.killer, p.expires_s, p.victim_event
        ),
        E::TeamBalanceNotice(n) => format!(
            r#"{{"type":"teamBalanceNotice","code":{},"args":[{},{}]}}"#,
            n.code as u8, n.args[0], n.args[1]
        ),
        E::MapVoteState(v) => format!(
            r#"{{"type":"mapVoteState","voteId":{},"candidates":{:?},"votes":{:?},"endsInS":{}}}"#,
            v.vote_id, v.candidates, v.votes, v.ends_in_s
        ),
        E::MapVoteResult(v) => format!(
            r#"{{"type":"mapVoteResult","voteId":{},"winner":{}}}"#,
            v.vote_id, v.winner
        ),
    })
}

/// Decodes one C4 payload to JSON (see [`decode_c4_core`]); throws on malformed bytes.
#[wasm_bindgen(js_name = decodeC4)]
pub fn decode_c4_js(bytes: &[u8]) -> Result<String, JsError> {
    decode_c4_core(bytes).map_err(js_err)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn input_cmd_is_18_bytes_and_bundle_roundtrips() {
        let mk =
            |i: u32| pack_input_cmd_core(i as u16, 10 + i, 5, -7, 1000, -300, 1, 0, 0, 0).unwrap();
        assert_eq!(mk(0).len(), INPUT_CMD_BYTES);
        let cmds: Vec<u8> = (0..3).flat_map(mk).collect();
        let b = bundle_core(&cmds, &[], 4, 9).unwrap();
        assert_eq!(decode_input_bundle(&b).unwrap().cmds.len(), 3);
        // 4 commands, wrong lengths, non-consecutive ticks and a bad ack all fail cleanly.
        let four: Vec<u8> = (0..4).flat_map(mk).collect();
        assert!(bundle_core(&four, &[], 0, 0).is_err());
        assert!(bundle_core(&cmds[..20], &[], 0, 0).is_err());
        assert!(bundle_core(&[], &[], 0, 0).is_err());
        assert!(bundle_core(&mk(0), &[], 70_000, 0).is_err());
        let gap: Vec<u8> = [mk(0), mk(2)].concat();
        assert!(bundle_core(&gap, &[], 0, 0).is_err());
    }

    #[test]
    fn c4_score_keeps_packed_kind_and_exposes_category_and_reason() {
        // C4 type, player, Support with reason 63, signed i16 in big-endian order.
        let json = decode_c4_core(&[0x04, 7, 0xff, 0x80, 0x00]).unwrap();
        assert_eq!(
            serde_json::from_str::<serde_json::Value>(&json).unwrap(),
            serde_json::json!({
                "type": "score", "player": 7, "kind": 255,
                "category": 3, "reason": 63, "value": -32768,
            }),
        );
    }

    #[test]
    fn c4_events_decode_to_ts_shaped_json() {
        use frontline_protocol::msg::c4::{
            encode_c4, Kill, Notice, PlayerRoster, ServerEventC4 as E,
        };
        let k = encode_c4(&E::Kill(Kill {
            victim: 3,
            killer: 4,
            weapon: 1,
            headshot: true,
            dist_q: 400,
            assist1: 0,
            assist2: 0,
        }))
        .unwrap();
        assert_eq!(
            decode_c4_core(&k).unwrap(),
            r#"{"type":"kill","victim":3,"killer":4,"weapon":1,"headshot":true,"distQ":400}"#
        );
        let n = encode_c4(&E::Notice(Notice {
            code: 7,
            args: [1, -2, 3],
        }))
        .unwrap();
        assert_eq!(
            decode_c4_core(&n).unwrap(),
            r#"{"type":"notice","code":7,"args":[1,-2,3]}"#
        );
        let r = encode_c4(&E::PlayerRoster(PlayerRoster {
            player: 5,
            nick: "A\"b\\c".into(),
            team: 1,
            squad: 2,
            role: 3,
            flags: 8,
        }))
        .unwrap();
        let j = decode_c4_core(&r).unwrap();
        assert!(
            j.contains(r#""nick":"A\"b\\c""#) && j.ends_with(r#""left":true}"#),
            "{j}"
        );
        assert!(decode_c4_core(&[0xff]).is_err());
        assert!(decode_c4_core(&k[..2]).is_err());
    }

    #[test]
    fn server_c0_decodes() {
        use frontline_protocol::msg::{encode_c0_server, Pong};
        let b = encode_c0_server(&ServerMsgC0::Pong(Pong {
            client_time: 7,
            server_tick: 8,
            server_time_ms: 9,
        }))
        .unwrap();
        let d = decode_server_core(&b).unwrap();
        assert_eq!(
            (d.kind, d.client_time, d.server_tick, d.server_time_ms),
            (c0_type::PONG, 7, 8, 9)
        );
        assert!(decode_server_core(&b[..2]).is_err());
        assert!(decode_server_core(&[0xff]).is_err());
    }

    #[test]
    fn deploy_result_keeps_ok_spawn_tick_and_reason() {
        use frontline_protocol::msg::c0::{encode_c0_server, DeployResult};
        for (ok, spawn_tick, reason) in [(true, 0, 0), (true, 0xf1234567, 0), (false, 0, 21)] {
            let bytes = encode_c0_server(&ServerMsgC0::DeployResult(DeployResult {
                ok,
                spawn_tick,
                reason,
            }))
            .unwrap();
            let d = decode_server_core(&bytes).unwrap();
            assert_eq!(d.kind, c0_type::DEPLOY_RESULT);
            assert_eq!((d.ok, d.spawn_tick, d.reason), (ok, spawn_tick, reason));
        }
    }
}
