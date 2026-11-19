/**
 * Texture processing (ARCHITECTURE 11.1 step 4, M1-002): ORM packing, NPOT padding, KTX2 (Basis) encoding with mips.
 *
 *  - colour slots (baseColor / emissive): ETC1S for profile `bg`, UASTC for `hero` / `ui`
 *  - normal slot: always UASTC, stored as two channels (RRRG layout: X in R/G/B, Y in A), whatever the asset profile is
 *  - occlusion + metallicRoughness: packed into one ORM image (R = AO, G = roughness, B = metallic), data (linear) encoding
 *
 * The card names `toktx` as the encoder; this pipeline uses the Basis Universal WASM encoder through `ktx2-encoder`
 * (same codecs, same KTX2 container, no native binary needed on the build machine; see PI-033).
 * Images may be PNG (decoded in-process via pngjs) or JPEG (decoded via ffmpeg, matching Poly Haven CC0 glTF textures).
 */
import { type Document, type Material, type Texture } from '@gltf-transform/core';
import { KHRTextureBasisu } from '@gltf-transform/extensions';
import { read as readKtx2 } from 'ktx-parse';
import { encodeToKTX2 } from 'ktx2-encoder';
import { PNG } from 'pngjs';
import { assertAllowed } from './allowed.ts';
import { ffmpeg } from './audio/ffmpeg.ts';
import type { Ledger, LedgerRow } from './ledger.ts';
import { ExitCode, GateError } from './errors.ts';
import type { AssetClass, TextureProfile } from './asset-class.ts';

export interface Raster {
  width: number;
  height: number;
  /** RGBA, 8 bit. */
  data: Uint8Array;
}

export type Ktx2Format = 'ETC1S' | 'UASTC';
export type TextureSlot = 'color' | 'normal' | 'orm' | 'data';

export interface TexResult {
  name: string;
  slot: TextureSlot;
  profile: TextureProfile;
  format: Ktx2Format;
  width: number;
  height: number;
  levels: number;
  /** Data channels stored: 2 for the RRRG normal layout, else 3 (4 with a used alpha). */
  channels: number;
  bytes: number;
  sourceBytes: number;
  padded: boolean;
}

export interface TextureOutcome {
  textures: TexResult[];
  warnings: string[];
}

/** KHR_DF_MODEL_ETC1S / KHR_DF_MODEL_UASTC. */
const MODEL_ETC1S = 163;
const MODEL_UASTC = 166;

export function decodePng(bytes: Uint8Array): Raster {
  const p = PNG.sync.read(Buffer.from(bytes.buffer, bytes.byteOffset, bytes.byteLength));
  return { width: p.width, height: p.height, data: new Uint8Array(p.data.buffer, p.data.byteOffset, p.data.byteLength) };
}

export function encodePng(r: Raster): Uint8Array {
  const p = new PNG({ width: r.width, height: r.height });
  p.data = Buffer.from(r.data);
  return new Uint8Array(PNG.sync.write(p));
}

/** Format policy table: profile -> codec. Normal-map slots override the asset profile (always UASTC). */
export function selectFormat(profile: TextureProfile, slot: TextureSlot = 'color'): Ktx2Format {
  if (slot === 'normal') return 'UASTC';
  return profile === 'bg' ? 'ETC1S' : 'UASTC';
}

/** Profile of one texture: per-texture override, else the asset profile; normal slots are always `normal`. */
export function profileFor(cls: AssetClass, name: string, slot: TextureSlot): TextureProfile {
  if (slot === 'normal') return 'normal';
  return cls.textures?.[name] ?? cls.texture_profile ?? 'bg';
}

/**
 * ORM packing: three single-channel (grey) inputs -> one RGB raster, R = AO, G = roughness, B = metallic.
 * The red channel of each input is used. A missing AO map is filled with 255 (no occlusion).
 */
export function packOrm(inputs: { ao?: Raster | undefined; roughness: Raster; metallic: Raster }): Raster {
  const { roughness: r, metallic: m, ao } = inputs;
  if (r.width !== m.width || r.height !== m.height || (ao && (ao.width !== r.width || ao.height !== r.height))) {
    throw new GateError(ExitCode.Usage, `ORM inputs must have equal size (roughness ${r.width}x${r.height}, metallic ${m.width}x${m.height}${ao ? `, ao ${ao.width}x${ao.height}` : ''})`);
  }
  const n = r.width * r.height;
  const data = new Uint8Array(n * 4);
  for (let i = 0; i < n; i++) {
    data[i * 4] = ao ? (ao.data[i * 4] as number) : 255;
    data[i * 4 + 1] = r.data[i * 4] as number;
    data[i * 4 + 2] = m.data[i * 4] as number;
    data[i * 4 + 3] = 255;
  }
  return { width: r.width, height: r.height, data };
}

/** Pad to a multiple of 4 in both directions (edge replicate). `padded` tells the caller to warn. */
export function padToMultipleOf4(r: Raster): { raster: Raster; padded: boolean } {
  const w = Math.ceil(r.width / 4) * 4;
  const h = Math.ceil(r.height / 4) * 4;
  if (w === r.width && h === r.height) return { raster: r, padded: false };
  const data = new Uint8Array(w * h * 4);
  for (let y = 0; y < h; y++) {
    const sy = Math.min(y, r.height - 1);
    for (let x = 0; x < w; x++) {
      const sx = Math.min(x, r.width - 1);
      const s = (sy * r.width + sx) * 4;
      const d = (y * w + x) * 4;
      for (let k = 0; k < 4; k++) data[d + k] = r.data[s + k] as number;
    }
  }
  return { raster: { width: w, height: h, data }, padded: true };
}

/** Normal map -> two-channel RRRG raster (X in RGB, Y in A), the layout the runtime swizzles back (M1-006). */
export function toRrrg(r: Raster): Raster {
  const data = new Uint8Array(r.data.length);
  for (let i = 0; i < r.width * r.height; i++) {
    const x = r.data[i * 4] as number;
    data[i * 4] = x;
    data[i * 4 + 1] = x;
    data[i * 4 + 2] = x;
    data[i * 4 + 3] = r.data[i * 4 + 1] as number;
  }
  return { width: r.width, height: r.height, data };
}

function hasUsedAlpha(r: Raster): boolean {
  for (let i = 3; i < r.data.length; i += 4) if (r.data[i] !== 255) return true;
  return false;
}

const imageDecoder = async (buffer: Uint8Array) => decodePng(buffer);

export interface EncodeOptions {
  format: Ktx2Format;
  /** sRGB colour data (true) or linear data (normal / ORM). */
  srgb: boolean;
  /** Mark the file as the 2-channel normal layout. */
  normal?: boolean;
  channels?: number;
}

/** Encode one raster as KTX2 (with a full mip chain). */
export async function encodeKtx2(raster: Raster, opts: EncodeOptions): Promise<Uint8Array> {
  const kv: Record<string, string> = { fl_channels: String(opts.channels ?? 3) };
  if (opts.normal) kv.fl_layout = 'RRRG';
  // The Basis WASM module prints progress to stdout through console.log; keep the CLI output clean (encodes run one at a time).
  const log = console.log;
  console.log = () => undefined;
  try {
    return await encode(raster, opts, kv);
  } finally {
    console.log = log;
  }
}

function encode(raster: Raster, opts: EncodeOptions, kv: Record<string, string>): Promise<Uint8Array> {
  return encodeToKTX2(encodePng(raster), {
    isUASTC: opts.format === 'UASTC',
    generateMipmap: true,
    isKTX2File: true,
    isPerceptual: opts.srgb,
    isSetKTX2SRGBTransferFunc: opts.srgb,
    needSupercompression: opts.format === 'UASTC',
    ...(opts.format === 'ETC1S' ? { qualityLevel: 128, compressionLevel: 1 } : { uastcLDRQualityLevel: 1 }),
    kvData: kv,
    imageDecoder,
  });
}

export interface Ktx2Info {
  format: Ktx2Format | 'other';
  width: number;
  height: number;
  levels: number;
  channels: number;
  layout: string | undefined;
  bytes: number;
}

/** Read the KTX2 header + DFD (what `ktx info` would show) and the pipeline's own key/value tags. */
export function inspectKtx2(bytes: Uint8Array): Ktx2Info {
  const c = readKtx2(bytes);
  const model = c.dataFormatDescriptor[0]?.colorModel;
  const fmt: Ktx2Info['format'] = model === MODEL_ETC1S ? 'ETC1S' : model === MODEL_UASTC ? 'UASTC' : 'other';
  const tag = (k: string): string | undefined => {
    const v = c.keyValue[k];
    return v === undefined ? undefined : typeof v === 'string' ? v : new TextDecoder().decode(v).replace(/\0+$/, '');
  };
  return {
    format: fmt,
    width: c.pixelWidth,
    height: c.pixelHeight,
    levels: c.levels.length,
    channels: Number(tag('fl_channels') ?? 0),
    layout: tag('fl_layout'),
    bytes: bytes.byteLength,
  };
}

function slotOf(doc: Document): Map<Texture, Set<'color' | 'normal' | 'data'>> {
  const m = new Map<Texture, Set<'color' | 'normal' | 'data'>>();
  const add = (t: Texture | null, s: 'color' | 'normal' | 'data') => {
    if (!t) return;
    const set = m.get(t) ?? new Set();
    set.add(s);
    m.set(t, set);
  };
  for (const mat of doc.getRoot().listMaterials()) {
    add(mat.getBaseColorTexture(), 'color');
    add(mat.getEmissiveTexture(), 'color');
    add(mat.getNormalTexture(), 'normal');
    add(mat.getOcclusionTexture(), 'data');
    add(mat.getMetallicRoughnessTexture(), 'data');
  }
  return m;
}

function channelRaster(r: Raster, ch: number): Raster {
  const data = new Uint8Array(r.data.length);
  for (let i = 0; i < r.width * r.height; i++) {
    const v = r.data[i * 4 + ch] as number;
    data[i * 4] = v;
    data[i * 4 + 1] = v;
    data[i * 4 + 2] = v;
    data[i * 4 + 3] = 255;
  }
  return { width: r.width, height: r.height, data };
}

function imageOf(t: Texture): { bytes: Uint8Array; mime: string } {
  const img = t.getImage();
  if (!img) throw new GateError(ExitCode.Usage, `texture "${t.getName()}" has no image data`);
  const mime = t.getMimeType();
  if (mime !== 'image/png' && mime !== 'image/jpeg' && mime !== 'image/jpg') {
    throw new GateError(ExitCode.Usage, `texture "${t.getName()}": only PNG and JPEG sources are supported (got ${mime || 'unknown'}); convert upstream`);
  }
  return { bytes: img, mime };
}

/** Decode PNG in-process or JPEG via build-machine ffmpeg into an 8-bit RGBA raster. */
export async function decodeImage(bytes: Uint8Array, mime: string): Promise<Raster> {
  if (mime === 'image/png') return decodePng(bytes);
  if (mime === 'image/jpeg' || mime === 'image/jpg') {
    const { stdout } = await ffmpeg(['-f', 'image2pipe', '-vcodec', 'mjpeg', '-i', 'pipe:0', '-f', 'image2pipe', '-vcodec', 'png', 'pipe:1'], { input: bytes });
    return decodePng(new Uint8Array(stdout.buffer, stdout.byteOffset, stdout.byteLength));
  }
  throw new GateError(ExitCode.Usage, `unsupported image mime "${mime || 'unknown'}"`);
}

/** Pack AO + metallicRoughness of every material into one ORM texture (in place). Returns warnings. */
export async function packMaterialsOrm(doc: Document): Promise<string[]> {
  const warnings: string[] = [];
  const done = new Map<string, Texture>();
  for (const mat of doc.getRoot().listMaterials()) {
    const ao = mat.getOcclusionTexture();
    const mr = mat.getMetallicRoughnessTexture();
    if (!ao || ao === mr) continue;
    const aoImg = imageOf(ao);
    const aoR = await decodeImage(aoImg.bytes, aoImg.mime);
    let rough: Raster;
    let metal: Raster;
    if (mr) {
      const mrImg = imageOf(mr);
      const mrR = await decodeImage(mrImg.bytes, mrImg.mime);
      if (mrR.width !== aoR.width || mrR.height !== aoR.height) {
        warnings.push(`${mat.getName()}: AO ${aoR.width}x${aoR.height} and metallicRoughness ${mrR.width}x${mrR.height} differ in size; not packed`);
        continue;
      }
      rough = channelRaster(mrR, 1);
      metal = channelRaster(mrR, 2);
    } else {
      // No MR texture: glTF falls back to the factors; 255 keeps "factor x 1".
      const white = { width: aoR.width, height: aoR.height, data: new Uint8Array(aoR.width * aoR.height * 4).fill(255) };
      rough = white;
      metal = white;
    }
    const key = `${ao.getName()}|${mr?.getName() ?? ''}|${aoR.width}`;
    let orm = done.get(key);
    if (!orm) {
      orm = doc
        .createTexture(`orm_${mat.getName() || done.size}`)
        .setImage(encodePng(packOrm({ ao: channelRaster(aoR, 0), roughness: rough, metallic: metal })))
        .setMimeType('image/png');
      done.set(key, orm);
    }
    const aoInfo = mat.getOcclusionTextureInfo();
    const mrInfo = mat.getMetallicRoughnessTextureInfo();
    const texCoord = mr ? mrInfo?.getTexCoord() : aoInfo?.getTexCoord();
    mat.setOcclusionTexture(orm).setMetallicRoughnessTexture(orm);
    if (texCoord !== undefined) {
      mat.getOcclusionTextureInfo()?.setTexCoord(texCoord);
      mat.getMetallicRoughnessTextureInfo()?.setTexCoord(texCoord);
    }
  }
  // Drop the replaced source textures.
  for (const t of doc.getRoot().listTextures()) if (t.listParents().every((p) => p.propertyType === 'Root')) t.dispose();
  return warnings;
}

export interface ProcessTexturesOptions {
  cls: AssetClass;
  /** When set, `assertAllowed(assetId, 'TEXCOMP')` runs before the first texture is touched. */
  assetId?: string;
  ledger?: Ledger | readonly LedgerRow[];
}

/**
 * Pack ORM, pad, and encode every texture of `doc` to KTX2 in place (KHR_texture_basisu, required).
 * Mutates `doc`. A document without textures is returned untouched and needs no TEXCOMP permission.
 */
export async function processTextures(doc: Document, o: ProcessTexturesOptions): Promise<TextureOutcome> {
  const warnings: string[] = [];
  if (doc.getRoot().listTextures().length === 0) return { textures: [], warnings };
  if (o.assetId) assertAllowed(o.assetId, 'TEXCOMP', o.ledger);
  warnings.push(...(await packMaterialsOrm(doc)));
  const slots = slotOf(doc);
  const ormTextures = new Set<Texture>();
  for (const mat of doc.getRoot().listMaterials()) {
    const ao = mat.getOcclusionTexture();
    if (ao && ao === mat.getMetallicRoughnessTexture()) ormTextures.add(ao);
  }
  const results: TexResult[] = [];
  for (const tex of doc.getRoot().listTextures()) {
    const s = slots.get(tex) ?? new Set(['color']);
    const slot: TextureSlot = ormTextures.has(tex) ? 'orm' : s.has('normal') ? 'normal' : s.has('color') ? 'color' : 'data';
    const name = tex.getName() || tex.getURI() || `texture_${results.length}`;
    const profile = profileFor(o.cls, name, slot === 'orm' ? 'data' : slot);
    const format = selectFormat(profile, slot);
    const src = imageOf(tex);
    let raster = await decodeImage(src.bytes, src.mime);
    const pad = padToMultipleOf4(raster);
    if (pad.padded) {
      warnings.push(`${name}: ${raster.width}x${raster.height} is not a multiple of 4; padded to ${pad.raster.width}x${pad.raster.height} (UVs are not rescaled)`);
    }
    raster = pad.raster;
    const isNormal = slot === 'normal';
    const channels = isNormal ? 2 : hasUsedAlpha(raster) ? 4 : 3;
    const input = isNormal ? toRrrg(raster) : raster;
    const out = await encodeKtx2(input, { format, srgb: slot === 'color', normal: isNormal, channels });
    const info = inspectKtx2(out);
    tex.setImage(out).setMimeType('image/ktx2');
    results.push({
      name, slot, profile, format: info.format === 'other' ? format : info.format, width: info.width, height: info.height,
      levels: info.levels, channels, bytes: out.byteLength, sourceBytes: src.bytes.byteLength, padded: pad.padded,
    });
  }
  const ext = doc.createExtension(KHRTextureBasisu).setRequired(true);
  void ext;
  return { textures: results, warnings };
}

/** True when the material references a texture (used by callers deciding whether TEXCOMP applies). */
export function usesTextures(mat: Material): boolean {
  return !!(mat.getBaseColorTexture() || mat.getNormalTexture() || mat.getOcclusionTexture() || mat.getMetallicRoughnessTexture() || mat.getEmissiveTexture());
}
