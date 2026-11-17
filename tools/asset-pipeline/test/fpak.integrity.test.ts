import { describe, expect, it } from 'vitest';
import { BufferSource, FpakFormatError, FpakReader, FpakWriter, IntegrityError, FOOTER_BYTES } from '../src/fpak/index.ts';

function build() {
  const w = new FpakWriter();
  w.add('a_x@lod0', 'mesh', 1, Uint8Array.from({ length: 100 }, (_, i) => i));
  w.add('b_x@lod0', 'texture', 2, Uint8Array.from({ length: 100 }, (_, i) => 255 - i));
  return w.finish();
}

describe('integrity (acceptance 3) and versioning', () => {
  it('flipping one byte of any entry makes read() throw IntegrityError', async () => {
    const b = build();
    for (const ix of b.index) {
      for (const k of [0, 50, 99]) {
        const bytes = b.bytes.slice();
        bytes[Number(ix.offset) + k] = (bytes[Number(ix.offset) + k] as number) ^ 1;
        const r = await FpakReader.open(new BufferSource(bytes));
        await expect(r.read(ix.logical_id)).rejects.toBeInstanceOf(IntegrityError);
        // the neighbour is unaffected
        const other = b.index.find((e) => e !== ix)!;
        await expect(r.read(other.logical_id)).resolves.toHaveLength(100);
      }
    }
  });

  it('verification can be switched off', async () => {
    const b = build();
    const bytes = b.bytes.slice();
    bytes[Number(b.index[0]!.offset)] = (bytes[Number(b.index[0]!.offset)] as number) ^ 1;
    const r = await FpakReader.open(new BufferSource(bytes), { verify: false });
    await expect(r.read('a_x@lod0')).resolves.toHaveLength(100);
  });

  it('a corrupted index fails to open with IntegrityError', async () => {
    const b = build();
    const bytes = b.bytes.slice();
    bytes[bytes.length - FOOTER_BYTES - 3] = (bytes[bytes.length - FOOTER_BYTES - 3] as number) ^ 0x40;
    await expect(FpakReader.open(new BufferSource(bytes))).rejects.toBeInstanceOf(IntegrityError);
  });

  it('unknown version and foreign files are rejected with clear errors', async () => {
    const b = build();
    const v2 = b.bytes.slice();
    new DataView(v2.buffer).setUint16(v2.length - FOOTER_BYTES + 20, 2, true);
    await expect(FpakReader.open(new BufferSource(v2))).rejects.toThrow(/unsupported .fpak version 2/);
    await expect(FpakReader.open(new BufferSource(new Uint8Array(100)))).rejects.toBeInstanceOf(FpakFormatError);
    await expect(FpakReader.open(new BufferSource(new Uint8Array(5)))).rejects.toBeInstanceOf(FpakFormatError);
  });

  it('unknown entry id', async () => {
    const r = await FpakReader.open(new BufferSource(build().bytes));
    await expect(r.read('nope')).rejects.toBeInstanceOf(FpakFormatError);
    expect(r.has('a_x@lod0')).toBe(true);
  });
});
