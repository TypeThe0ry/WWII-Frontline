/**
 * WebGPU compute smoke + feasibility trials (M0-020, E0-6, ARCHITECTURE §20.1-4).
 * Run with `?smoke=compute`. Result is published as `window.__smoke`.
 * Each WGSL block is < 60 lines. Image quality is irrelevant; only data round trips are checked.
 */
import { ComputeShader } from '@babylonjs/core/Compute/computeShader';
import '@babylonjs/core/Engines/WebGPU/Extensions/engine.computeShader';
import '@babylonjs/core/Engines/WebGPU/Extensions/engine.readTexture';
import { StorageBuffer } from '@babylonjs/core/Buffers/storageBuffer';
import { RawTexture } from '@babylonjs/core/Materials/Textures/rawTexture';
import { RawTexture3D } from '@babylonjs/core/Materials/Textures/rawTexture3D';
import { Constants } from '@babylonjs/core/Engines/constants';
import type { BaseTexture } from '@babylonjs/core/Materials/Textures/baseTexture';
import type { WebGPUEngine } from '@babylonjs/core/Engines/webgpuEngine';
import type { RenderContext } from './engine';
import { errMsg } from './capabilities';

export interface TrialResult {
  ok: boolean;
  mismatches: number;
  ms: number;
  detail: string;
}

export interface SmokeResult {
  ok: boolean;
  mismatches: number;
  ms: number;
  status: string;
  buffer?: TrialResult;
  texture?: TrialResult;
  hiz?: TrialResult;
  froxel?: TrialResult;
  taau?: TrialResult;
}

// ---- WGSL ----------------------------------------------------------------------------------

const WGSL_BUFFER_PATTERN = /* wgsl */ `
@group(0) @binding(0) var<storage, read_write> outBuf : array<u32>;
@compute @workgroup_size(8, 8, 1)
fn main(@builtin(global_invocation_id) id : vec3<u32>) {
  if (id.x >= 64u || id.y >= 64u) { return; }
  outBuf[id.y * 64u + id.x] = id.x * 64u + id.y;
}`;

const WGSL_TEXTURE_PATTERN = /* wgsl */ `
@group(0) @binding(0) var outTex : texture_storage_2d<r32float, write>;
@compute @workgroup_size(8, 8, 1)
fn main(@builtin(global_invocation_id) id : vec3<u32>) {
  if (id.x >= 64u || id.y >= 64u) { return; }
  textureStore(outTex, vec2<i32>(id.xy), vec4<f32>(f32(id.x * 64u + id.y), 0.0, 0.0, 0.0));
}`;

// Hi-Z: level 0 is generated on the GPU; each further level is a 2x2 max-reduction of the previous.
const WGSL_HIZ_INIT = /* wgsl */ `
@group(0) @binding(0) var dst : texture_storage_2d<r32float, write>;
@compute @workgroup_size(8, 8, 1)
fn main(@builtin(global_invocation_id) id : vec3<u32>) {
  if (id.x >= 64u || id.y >= 64u) { return; }
  let d = f32((id.x * 7u + id.y * 13u) % 64u) / 64.0;
  textureStore(dst, vec2<i32>(id.xy), vec4<f32>(d, 0.0, 0.0, 0.0));
}`;

const WGSL_HIZ_DOWN = /* wgsl */ `
@group(0) @binding(0) var src : texture_2d<f32>;
@group(0) @binding(1) var dst : texture_storage_2d<r32float, write>;
@compute @workgroup_size(8, 8, 1)
fn main(@builtin(global_invocation_id) id : vec3<u32>) {
  let size = textureDimensions(dst);
  if (id.x >= size.x || id.y >= size.y) { return; }
  let b = vec2<i32>(id.xy) * 2;
  let m = max(max(textureLoad(src, b, 0).x, textureLoad(src, b + vec2<i32>(1, 0), 0).x),
              max(textureLoad(src, b + vec2<i32>(0, 1), 0).x, textureLoad(src, b + vec2<i32>(1, 1), 0).x));
  textureStore(dst, vec2<i32>(id.xy), vec4<f32>(m, 0.0, 0.0, 0.0));
}`;

// Froxel injection: write a constant into a 160x90x64 3D storage texture, then sum it on the GPU.
const WGSL_FROXEL_WRITE = /* wgsl */ `
@group(0) @binding(0) var vol : texture_storage_3d<r32float, write>;
@compute @workgroup_size(4, 4, 4)
fn main(@builtin(global_invocation_id) id : vec3<u32>) {
  if (id.x >= 160u || id.y >= 90u || id.z >= 64u) { return; }
  textureStore(vol, vec3<i32>(id), vec4<f32>(1.0, 0.0, 0.0, 0.0));
}`;

const WGSL_FROXEL_SUM = /* wgsl */ `
@group(0) @binding(0) var vol : texture_3d<f32>;
@group(0) @binding(1) var<storage, read_write> total : array<atomic<u32>>;
@compute @workgroup_size(4, 4, 4)
fn main(@builtin(global_invocation_id) id : vec3<u32>) {
  if (id.x >= 160u || id.y >= 90u || id.z >= 64u) { return; }
  atomicAdd(&total[0], u32(textureLoad(vol, vec3<i32>(id), 0).x));
}`;

// TAAU history blend placeholder: out = mix(history(x - 1, y), current(x, y), 0.25), constant "motion" of 1 px.
const WGSL_TAAU_INIT = /* wgsl */ `
@group(0) @binding(0) var cur : texture_storage_2d<rgba8unorm, write>;
@group(0) @binding(1) var hist : texture_storage_2d<rgba8unorm, write>;
@compute @workgroup_size(8, 8, 1)
fn main(@builtin(global_invocation_id) id : vec3<u32>) {
  if (id.x >= 64u || id.y >= 64u) { return; }
  let p = vec2<i32>(id.xy);
  textureStore(cur, p, vec4<f32>(f32(id.x) / 63.0, f32(id.y) / 63.0, 0.5, 1.0));
  textureStore(hist, p, vec4<f32>(f32(id.x) / 63.0 * 0.5, 0.2, 0.8, 1.0));
}`;

const WGSL_TAAU_RESOLVE = /* wgsl */ `
@group(0) @binding(0) var curT : texture_2d<f32>;
@group(0) @binding(1) var histT : texture_2d<f32>;
@group(0) @binding(2) var outT : texture_storage_2d<rgba8unorm, write>;
@compute @workgroup_size(8, 8, 1)
fn main(@builtin(global_invocation_id) id : vec3<u32>) {
  if (id.x >= 64u || id.y >= 64u) { return; }
  let p = vec2<i32>(id.xy);
  let hp = clamp(p - vec2<i32>(1, 0), vec2<i32>(0), vec2<i32>(63));
  textureStore(outT, p, mix(textureLoad(histT, hp, 0), textureLoad(curT, p, 0), 0.25));
}`;

// ---- helpers -------------------------------------------------------------------------------

type Mapping = Record<string, { group: number; binding: number }>;

async function timed(f: () => Promise<Omit<TrialResult, 'ms'>>): Promise<TrialResult> {
  const t0 = performance.now();
  try {
    const r = await f();
    return { ...r, ms: performance.now() - t0 };
  } catch (e) {
    return { ok: false, mismatches: -1, ms: performance.now() - t0, detail: `exception: ${errMsg(e)}` };
  }
}

function shader(ctx: RenderContext, name: string, source: string, mapping: Mapping): ComputeShader {
  const cs = new ComputeShader(name, ctx.engine, { computeSource: source }, { bindingsMapping: mapping });
  cs.onError = (_e, err) => console.error(`[compute-smoke] ${name}: ${err}`);
  return cs;
}

async function readF32(tex: BaseTexture): Promise<Float32Array> {
  const data = await tex.readPixels();
  if (!data) throw new Error('readPixels returned null');
  return new Float32Array(data.buffer, data.byteOffset, data.byteLength / 4);
}

// ---- trials --------------------------------------------------------------------------------

async function trialBuffer(ctx: RenderContext): Promise<TrialResult> {
  return timed(async () => {
    const sb = new StorageBuffer(ctx.engine as WebGPUEngine, 64 * 64 * 4);
    const cs = shader(ctx, 'smokeBuf', WGSL_BUFFER_PATTERN, { outBuf: { group: 0, binding: 0 } });
    cs.setStorageBuffer('outBuf', sb);
    await cs.dispatchWhenReady(8, 8, 1);
    const out = new Uint32Array((await sb.read()).buffer.slice(0));
    let bad = 0;
    for (let y = 0; y < 64; y++) for (let x = 0; x < 64; x++) if (out[y * 64 + x] !== x * 64 + y) bad++;
    sb.dispose();
    return { ok: bad === 0 && out.length >= 4096, mismatches: bad, detail: `storage buffer 64x64 u32 (${out.length} elements)` };
  });
}

async function trialTexture(ctx: RenderContext): Promise<TrialResult> {
  return timed(async () => {
    const tex = RawTexture.CreateRStorageTexture(null, 64, 64, ctx.scene, false, false, Constants.TEXTURE_NEAREST_SAMPLINGMODE, Constants.TEXTURETYPE_FLOAT);
    const cs = shader(ctx, 'smokeTex', WGSL_TEXTURE_PATTERN, { outTex: { group: 0, binding: 0 } });
    cs.setStorageTexture('outTex', tex);
    await cs.dispatchWhenReady(8, 8, 1);
    const out = await readF32(tex);
    let bad = 0;
    for (let y = 0; y < 64; y++) for (let x = 0; x < 64; x++) if (out[y * 64 + x] !== x * 64 + y) bad++;
    tex.dispose();
    return { ok: bad === 0, mismatches: bad, detail: `r32float storage texture 64x64 (${out.length} floats read back)` };
  });
}

async function trialHiZ(ctx: RenderContext): Promise<TrialResult> {
  return timed(async () => {
    const mk = (n: number) => RawTexture.CreateRStorageTexture(null, n, n, ctx.scene, false, false, Constants.TEXTURE_NEAREST_SAMPLINGMODE, Constants.TEXTURETYPE_FLOAT);
    const levels = [mk(64), mk(32), mk(16), mk(8), mk(4)];
    const init = shader(ctx, 'hizInit', WGSL_HIZ_INIT, { dst: { group: 0, binding: 0 } });
    init.setStorageTexture('dst', levels[0]!);
    await init.dispatchWhenReady(8, 8, 1);
    for (let l = 1; l < levels.length; l++) {
      const cs = shader(ctx, `hizDown${l}`, WGSL_HIZ_DOWN, { src: { group: 0, binding: 0 }, dst: { group: 0, binding: 1 } });
      cs.setTexture('src', levels[l - 1]!, false);
      cs.setStorageTexture('dst', levels[l]!);
      const n = 64 >> l;
      await cs.dispatchWhenReady(Math.ceil(n / 8), Math.ceil(n / 8), 1);
    }
    // CPU reference
    let ref = new Float32Array(64 * 64);
    for (let y = 0; y < 64; y++) for (let x = 0; x < 64; x++) ref[y * 64 + x] = ((x * 7 + y * 13) % 64) / 64;
    let bad = 0;
    for (let l = 1; l < levels.length; l++) {
      const n = 64 >> l;
      const next = new Float32Array(n * n);
      for (let y = 0; y < n; y++)
        for (let x = 0; x < n; x++) {
          const w = n * 2;
          next[y * n + x] = Math.max(ref[2 * y * w + 2 * x]!, ref[2 * y * w + 2 * x + 1]!, ref[(2 * y + 1) * w + 2 * x]!, ref[(2 * y + 1) * w + 2 * x + 1]!);
        }
      ref = next;
      const gpu = await readF32(levels[l]!);
      for (let i = 0; i < n * n; i++) if (Math.abs(gpu[i]! - ref[i]!) > 1e-6) bad++;
    }
    levels.forEach((t) => t.dispose());
    return { ok: bad === 0, mismatches: bad, detail: 'compute mip chain 64->4 (max-reduce) over R32F storage textures; separate texture per level (no per-mip storage view in Babylon API)' };
  });
}

async function trialFroxel(ctx: RenderContext): Promise<TrialResult> {
  return timed(async () => {
    const vol = new RawTexture3D(null, 160, 90, 64, Constants.TEXTUREFORMAT_RED, ctx.scene, false, false, Constants.TEXTURE_NEAREST_SAMPLINGMODE, Constants.TEXTURETYPE_FLOAT, Constants.TEXTURE_CREATIONFLAG_STORAGE);
    const sb = new StorageBuffer(ctx.engine as WebGPUEngine, 4);
    sb.update(new Uint32Array([0]));
    const wr = shader(ctx, 'froxelWrite', WGSL_FROXEL_WRITE, { vol: { group: 0, binding: 0 } });
    wr.setStorageTexture('vol', vol);
    await wr.dispatchWhenReady(40, 23, 16);
    const sum = shader(ctx, 'froxelSum', WGSL_FROXEL_SUM, { vol: { group: 0, binding: 0 }, total: { group: 0, binding: 1 } });
    sum.setTexture('vol', vol, false);
    sum.setStorageBuffer('total', sb);
    await sum.dispatchWhenReady(40, 23, 16);
    const total = new Uint32Array((await sb.read()).buffer.slice(0))[0]!;
    const expected = 160 * 90 * 64;
    vol.dispose();
    sb.dispose();
    return { ok: total === expected, mismatches: total === expected ? 0 : Math.abs(expected - total), detail: `3D r32float storage 160x90x64 write + GPU sum = ${total} (expected ${expected})` };
  });
}

async function trialTAAU(ctx: RenderContext): Promise<TrialResult> {
  return timed(async () => {
    const mk = () => RawTexture.CreateRGBAStorageTexture(null, 64, 64, ctx.scene, false, false, Constants.TEXTURE_NEAREST_SAMPLINGMODE, Constants.TEXTURETYPE_UNSIGNED_BYTE);
    const cur = mk();
    const hist = mk();
    const out = mk();
    const init = shader(ctx, 'taauInit', WGSL_TAAU_INIT, { cur: { group: 0, binding: 0 }, hist: { group: 0, binding: 1 } });
    init.setStorageTexture('cur', cur);
    init.setStorageTexture('hist', hist);
    await init.dispatchWhenReady(8, 8, 1);
    const res = shader(ctx, 'taauResolve', WGSL_TAAU_RESOLVE, { curT: { group: 0, binding: 0 }, histT: { group: 0, binding: 1 }, outT: { group: 0, binding: 2 } });
    res.setTexture('curT', cur, false);
    res.setTexture('histT', hist, false);
    res.setStorageTexture('outT', out);
    await res.dispatchWhenReady(8, 8, 1);
    const px = new Uint8Array((await out.readPixels())!.buffer.slice(0));
    const q = (v: number) => Math.round(v * 255) / 255; // rgba8unorm quantisation of the inputs
    let bad = 0;
    for (let y = 0; y < 64; y++)
      for (let x = 0; x < 64; x++) {
        const hx = Math.max(x - 1, 0);
        const c = [q(x / 63), q(y / 63), q(0.5)];
        const h = [q((hx / 63) * 0.5), q(0.2), q(0.8)];
        for (let k = 0; k < 3; k++) {
          const expect = (h[k]! + (c[k]! - h[k]!) * 0.25) * 255;
          if (Math.abs(px[(y * 64 + x) * 4 + k]! - expect) > 1.5) bad++;
        }
      }
    [cur, hist, out].forEach((t) => t.dispose());
    return { ok: bad === 0, mismatches: bad, detail: 'two RGBA8 RTs: history reprojected by a constant 1 px offset and blended (alpha 0.25); channels compared within +-1.5/255' };
  });
}

// ---- entry ---------------------------------------------------------------------------------

export async function runComputeSmoke(ctx: RenderContext): Promise<SmokeResult> {
  if (ctx.kind !== 'webgpu') {
    return { ok: false, mismatches: 0, ms: 0, status: 'unsupported (expected)' };
  }
  const t0 = performance.now();
  const buffer = await trialBuffer(ctx);
  const texture = await trialTexture(ctx);
  const hiz = await trialHiZ(ctx);
  const froxel = await trialFroxel(ctx);
  const taau = await trialTAAU(ctx);
  const mismatches = [buffer, texture].reduce((n, t) => n + Math.max(t.mismatches, 0), 0) + (buffer.mismatches < 0 || texture.mismatches < 0 ? 1 : 0);
  return {
    ok: buffer.ok && texture.ok,
    mismatches,
    ms: performance.now() - t0,
    status: 'ran',
    buffer,
    texture,
    hiz,
    froxel,
    taau,
  };
}
