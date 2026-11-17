/** Minimal RIFF/WAVE helpers (PCM s16 / f32): enough to read durations and to build test fixtures. */
export interface WavInfo {
  channels: number;
  sampleRate: number;
  bitsPerSample: number;
  format: number;
  frames: number;
  /** Chunk ids found, in order (to see bext / iXML / LIST). */
  chunks: string[];
  data: Uint8Array;
}

export function parseWav(b: Uint8Array): WavInfo {
  const dv = new DataView(b.buffer, b.byteOffset, b.byteLength);
  const tag = (o: number) => String.fromCharCode(b[o] as number, b[o + 1] as number, b[o + 2] as number, b[o + 3] as number);
  if (tag(0) !== 'RIFF' || tag(8) !== 'WAVE') throw new Error('not a RIFF/WAVE file');
  let p = 12;
  let fmt: Pick<WavInfo, 'channels' | 'sampleRate' | 'bitsPerSample' | 'format'> | undefined;
  let data: Uint8Array | undefined;
  const chunks: string[] = [];
  while (p + 8 <= b.byteLength) {
    const id = tag(p);
    let len = dv.getUint32(p + 4, true);
    chunks.push(id);
    if (id === 'fmt ') {
      fmt = { format: dv.getUint16(p + 8, true), channels: dv.getUint16(p + 10, true), sampleRate: dv.getUint32(p + 12, true), bitsPerSample: dv.getUint16(p + 22, true) };
    } else if (id === 'data') {
      if (p + 8 + len > b.byteLength) len = b.byteLength - p - 8; // streamed WAV with 0xffffffff size
      data = b.subarray(p + 8, p + 8 + len);
    }
    p += 8 + len + (len & 1);
  }
  if (!fmt || !data) throw new Error('WAV without fmt/data chunk');
  return { ...fmt, frames: data.byteLength / ((fmt.bitsPerSample / 8) * fmt.channels), chunks, data };
}

/** Samples of channel 0 as floats in [-1, 1]. */
export function samples(w: WavInfo): Float32Array {
  const bytes = w.bitsPerSample / 8;
  const out = new Float32Array(w.frames);
  const dv = new DataView(w.data.buffer, w.data.byteOffset, w.data.byteLength);
  for (let i = 0; i < w.frames; i++) {
    const o = i * w.channels * bytes;
    out[i] = w.format === 3 ? dv.getFloat32(o, true) : w.bitsPerSample === 16 ? dv.getInt16(o, true) / 32768 : dv.getInt32(o, true) / 2147483648;
  }
  return out;
}

export function writeWav(opts: { sampleRate: number; channels?: number; samples: Float32Array; extraChunks?: { id: string; data: Uint8Array }[] }): Uint8Array {
  const ch = opts.channels ?? 1;
  const pcm = new Uint8Array(opts.samples.length * 2 * ch);
  const dv = new DataView(pcm.buffer);
  opts.samples.forEach((s, i) => {
    const v = Math.max(-1, Math.min(1, s)) * 32767;
    for (let c = 0; c < ch; c++) dv.setInt16((i * ch + c) * 2, Math.round(v), true);
  });
  const fmt = new Uint8Array(16);
  const f = new DataView(fmt.buffer);
  f.setUint16(0, 1, true);
  f.setUint16(2, ch, true);
  f.setUint32(4, opts.sampleRate, true);
  f.setUint32(8, opts.sampleRate * 2 * ch, true);
  f.setUint16(12, 2 * ch, true);
  f.setUint16(14, 16, true);
  const chunk = (id: string, d: Uint8Array) => {
    const o = new Uint8Array(8 + d.length + (d.length & 1));
    o.set(Buffer.from(id, 'latin1'), 0);
    new DataView(o.buffer).setUint32(4, d.length, true);
    o.set(d, 8);
    return o;
  };
  const parts = [chunk('fmt ', fmt), ...(opts.extraChunks ?? []).map((c) => chunk(c.id, c.data)), chunk('data', pcm)];
  const body = parts.reduce((s, p) => s + p.length, 4);
  const out = new Uint8Array(8 + body);
  out.set(Buffer.from('RIFF'), 0);
  new DataView(out.buffer).setUint32(4, body, true);
  out.set(Buffer.from('WAVE'), 8);
  let p = 12;
  for (const part of parts) {
    out.set(part, p);
    p += part.length;
  }
  return out;
}
