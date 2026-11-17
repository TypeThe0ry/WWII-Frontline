# Audio codec probe protocol (M1-004, D-AUD)

Primary: **Opus in WebM**. Fallback: **AAC-LC in MP4**. Never decide by user agent string: Safari support for Opus-in-WebM varies by
version, so the client measures what `decodeAudioData` accepts.

## Entries

Every audio asset has two entries per base logical id (`snd_*`, e.g. `snd_wpn_us_m1_garand_fire_near`):

- `{id}@opus` — audio entry layout v1 (`fpak/entry-audio.ts`), payload = WebM with one Opus stream, 48 kHz, CBR
- `{id}@aac` — audio entry layout v1, payload = MP4 with one AAC-LC stream, 48 kHz

(The task card wrote `#opus` / `#aac`; `.fpak` logical ids only allow `@` as the variant separator, so `@opus` / `@aac` it is — PI-035.)

The entry header carries `container`, `preload` (clip shorter than 3 s: decode once and keep resident; otherwise stream),
`loop`, `channels`, `duration_ms`, `sample_rate`, `loop_start_ms`, `loop_end_ms`. Layout in `entry-audio.ts`.

## Probe

`probe.json` (written next to the packs by `audio-pack`, referenced from the `core` bundle):

```json
{ "schema": "frontline.audio-probe/1", "timeout_ms": 500,
  "opus": { "mime": "audio/webm; codecs=opus", "base64": "...", "bytes": 0 },
  "aac":  { "mime": "audio/mp4; codecs=mp4a.40.2", "base64": "...", "bytes": 0 } }
```

Both samples are 50 ms of silence, below 2 KB each.

## Client procedure (implemented in M1-006 / M1-062)

1. At start-up call `audioContext.decodeAudioData(opusBytes)`; success within `timeout_ms` (500 ms) means `opus` works. A reject or the
   timeout means it does not.
2. If Opus failed, do the same with the AAC sample.
3. `chooseCodec({opus, aac})` (reference implementation: `probe-sample.ts`): `opus` if Opus decodes, else `aac` if AAC decodes,
   else `none` — which is an error the client must surface (no audio).
4. Request `{id}@{codec}` for every sound. Cache the decision for the session.

Changing this protocol (names, timeout, header fields) must be logged in `docs/PLAN_ISSUES.md`.
