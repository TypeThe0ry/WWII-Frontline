/**
 * `.fpak` container constants and small helpers (SPEC.md is the byte-level reference).
 *
 * Honest statement (D-PAK / R11): the container exists so that processed assets are not shipped as standalone
 * .glb/.ktx2/.ogg files. It is NOT encryption, obfuscation or DRM and must not be described as such.
 */
export const FPAK_VERSION = 1;
export const MAGIC = new Uint8Array([0x46, 0x50, 0x41, 0x4b]); // "FPAK"
export const MAGIC_END = new Uint8Array([0x4b, 0x41, 0x50, 0x46]); // "KAPF"
export const HEADER_BYTES = 32;
export const FOOTER_BYTES = 32;
export const ALIGN = 16;
/** Fixed part of an index entry, before the logical id bytes. */
export const INDEX_FIXED_BYTES = 2 + 1 + 1 + 2 + 8 + 4 + 8; // id_len, kind, reserved, codec, offset, bytes, sha8
export const MAX_ENTRY_BYTES = 0xffff_ffff;

export const Kind = { mesh: 0, texture: 1, audio: 2, meta: 3 } as const;
export type KindName = keyof typeof Kind;
export const KIND_NAMES: readonly KindName[] = ['mesh', 'texture', 'audio', 'meta'];

/** Codec numbers (a codec names the entry payload layout, not a general-purpose compressor). */
export const Codec = {
  raw: 0,
  /** entry-mesh.ts layout v1 (meshopt-compressed vertex / index buffers + own header). */
  meshV1: 1,
  /** entry-texture.ts layout v1 (KTX2 payload re-ordered, own header). */
  textureV1: 2,
  /** Reserved for M1-004 (audio). */
  audio: 3,
  /** UTF-8 JSON. */
  json: 4,
} as const;

/** Logical id: lower-case snake_case with an optional `@variant` part, e.g. `wpn_us_m1_garand@lod0_fp`. No paths, no vendors. */
export const LOGICAL_ID_RE = /^[a-z][a-z0-9_]*(@[a-z0-9_]+)?$/;
export const MAX_LOGICAL_ID_BYTES = 255;

let table: Uint32Array | undefined;
export function crc32(data: Uint8Array, seed = 0): number {
  if (!table) {
    table = new Uint32Array(256);
    for (let n = 0; n < 256; n++) {
      let c = n;
      for (let k = 0; k < 8; k++) c = c & 1 ? 0xedb88320 ^ (c >>> 1) : c >>> 1;
      table[n] = c >>> 0;
    }
  }
  let c = ~seed >>> 0;
  for (let i = 0; i < data.length; i++) c = (table[(c ^ (data[i] as number)) & 0xff] as number) ^ (c >>> 8);
  return ~c >>> 0;
}

export function alignUp(n: number, a = ALIGN): number {
  return Math.ceil(n / a) * a;
}

export function bytesEqual(a: Uint8Array, b: Uint8Array): boolean {
  if (a.length !== b.length) return false;
  for (let i = 0; i < a.length; i++) if (a[i] !== b[i]) return false;
  return true;
}

export function hex(b: Uint8Array): string {
  return Array.from(b, (x) => x.toString(16).padStart(2, '0')).join('');
}

export class FpakFormatError extends Error {
  constructor(message: string) {
    super(message);
    this.name = 'FpakFormatError';
  }
}

/** A payload or index failed its checksum. */
export class IntegrityError extends Error {
  readonly entry: string | undefined;
  constructor(message: string, entry?: string) {
    super(message);
    this.name = 'IntegrityError';
    this.entry = entry;
  }
}
