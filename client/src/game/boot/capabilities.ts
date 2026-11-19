/** Capability probe + quality-tier recommendation (UI_SPEC §4.1, M1 placeholder: Low/Medium/High). Local only, nothing uploaded. */
export type Backend = 'webgpu' | 'webgl2' | 'none';
export type Tier = 'low' | 'medium' | 'high';

export interface Capabilities {
  backend: Backend;
  gpuName?: string;
  deviceMemoryGb?: number;
  cores: number;
  storageQuotaMb?: number;
  recommendedTier: Tier;
}

/** Minimal structural view of `navigator`, so tests can mock it. */
export interface NavLike {
  gpu?: { requestAdapter(): Promise<{ info?: { description?: string; device?: string; vendor?: string } } | null | undefined> };
  deviceMemory?: number;
  hardwareConcurrency?: number;
  storage?: { estimate(): Promise<{ quota?: number }> };
}

export function recommendTier(backend: Backend, cores: number, memGb?: number): Tier {
  if (backend === 'none') return 'low';
  const mem = memGb ?? 4; // `deviceMemory` is capped at 8 and absent in some browsers
  if (backend === 'webgl2') return cores >= 8 && mem >= 8 ? 'medium' : 'low';
  if (cores >= 8 && mem >= 8) return 'high';
  if (cores >= 4 && mem >= 4) return 'medium';
  return 'low';
}

/** Features disabled on the WebGL2 degraded path (acceptance: ≥ 3 listed). Keys are i18n keys. */
export const DEGRADED_FEATURES: readonly string[] = ['ui.boot.degraded.volumetricFog', 'ui.boot.degraded.taau', 'ui.boot.degraded.ssr', 'ui.boot.degraded.gtao'];

export async function probeCapabilities(nav: NavLike, hasWebgl2: () => boolean): Promise<Capabilities> {
  let backend: Backend = 'none';
  let gpuName: string | undefined;
  if (nav.gpu) {
    try {
      const ad = await nav.gpu.requestAdapter();
      if (ad) {
        backend = 'webgpu';
        gpuName = ad.info?.description || ad.info?.device || ad.info?.vendor || undefined;
      }
    } catch {
      /* fall through to WebGL2 */
    }
  }
  if (backend === 'none' && hasWebgl2()) backend = 'webgl2';
  const cores = nav.hardwareConcurrency ?? 4;
  let storageQuotaMb: number | undefined;
  try {
    const q = (await nav.storage?.estimate())?.quota;
    if (q !== undefined) storageQuotaMb = Math.round(q / 1048576);
  } catch {
    /* ignore */
  }
  return { backend, gpuName, deviceMemoryGb: nav.deviceMemory, cores, storageQuotaMb, recommendedTier: recommendTier(backend, cores, nav.deviceMemory) };
}
