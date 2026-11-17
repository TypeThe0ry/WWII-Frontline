/**
 * "Opaque to generic tools" checks (M1-003 acceptance 5): mesh / texture entry payloads must not be recognised as glTF, GLB,
 * KTX2, or by `file(1)`. This documents the non-standalone property; it says nothing about secrecy (see format.ts).
 */
import { spawnSync } from 'node:child_process';
import { NodeIO } from '@gltf-transform/core';
import { read as readKtx2 } from 'ktx-parse';

export interface OpaqueCheck {
  tool: string;
  recognized: boolean;
  detail: string;
}

const KTX2_ID = Uint8Array.of(0xab, 0x4b, 0x54, 0x58, 0x20, 0x32, 0x30, 0xbb, 0x0d, 0x0a, 0x1a, 0x0a);

export async function checkOpaque(payload: Uint8Array): Promise<OpaqueCheck[]> {
  const out: OpaqueCheck[] = [];
  const head = Buffer.from(payload.subarray(0, 12));
  out.push({ tool: 'magic: glTF / KTX2 identifier', recognized: head.subarray(0, 4).toString('latin1') === 'glTF' || Buffer.compare(head, Buffer.from(KTX2_ID)) === 0, detail: head.toString('hex') });
  try {
    await new NodeIO().readBinary(payload);
    out.push({ tool: 'gltf-transform readBinary (GLB)', recognized: true, detail: 'parsed' });
  } catch (e) {
    out.push({ tool: 'gltf-transform readBinary (GLB)', recognized: false, detail: (e as Error).message.slice(0, 80) });
  }
  try {
    new NodeIO().readJSON({ json: JSON.parse(Buffer.from(payload).toString('utf8')) as never, resources: {} });
    out.push({ tool: 'glTF JSON', recognized: true, detail: 'parsed' });
  } catch {
    out.push({ tool: 'glTF JSON', recognized: false, detail: 'not JSON' });
  }
  try {
    readKtx2(payload);
    out.push({ tool: 'ktx-parse (ktx info equivalent)', recognized: true, detail: 'parsed' });
  } catch (e) {
    out.push({ tool: 'ktx-parse (ktx info equivalent)', recognized: false, detail: (e as Error).message.slice(0, 80) });
  }
  const f = spawnSync('file', ['-b', '--mime-type', '-'], { input: Buffer.from(payload) });
  if (f.status === 0) {
    const mime = f.stdout.toString().trim();
    out.push({ tool: 'file(1)', recognized: mime !== 'application/octet-stream' && mime !== 'application/x-empty', detail: mime });
  } else {
    out.push({ tool: 'file(1)', recognized: false, detail: 'file not installed; skipped' });
  }
  return out;
}

export async function assertOpaque(payload: Uint8Array, what = 'entry'): Promise<OpaqueCheck[]> {
  const checks = await checkOpaque(payload);
  const bad = checks.filter((c) => c.recognized);
  if (bad.length) throw new Error(`${what} is recognised by: ${bad.map((b) => b.tool).join(', ')}`);
  return checks;
}
