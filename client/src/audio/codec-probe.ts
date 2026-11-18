/** Codec selection (ARCHITECTURE §12.7): try Opus first, then AAC; if neither decodes, run silent (never throw). */
export type AudioPack = 'opus' | 'aac' | 'none';
export type Decoder = (probe: ArrayBuffer) => Promise<unknown>;

let cached: AudioPack | undefined;

export async function probeCodec(decode: Decoder, probes: { opus: ArrayBuffer; aac: ArrayBuffer }, force = false): Promise<AudioPack> {
  if (cached !== undefined && !force) return cached;
  const tryDecode = async (b: ArrayBuffer): Promise<boolean> => {
    try {
      await decode(b.slice(0)); // decodeAudioData detaches its input
      return true;
    } catch {
      return false;
    }
  };
  cached = (await tryDecode(probes.opus)) ? 'opus' : (await tryDecode(probes.aac)) ? 'aac' : 'none';
  return cached;
}
export const resetCodecProbe = (): void => {
  cached = undefined;
};
