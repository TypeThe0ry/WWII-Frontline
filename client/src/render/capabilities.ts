/**
 * Capability probing (ARCHITECTURE §13.6). The result is used locally to recommend a quality tier
 * and is never uploaded (privacy, §19).
 */
export interface WebGPUCaps {
  /** `navigator.gpu` exists. */
  hasApi: boolean;
  /** requestAdapter() resolved to a non-null adapter. */
  adapter: boolean;
  info: { vendor: string; architecture: string; device: string; description: string } | null;
  limits: { maxTextureDimension2D: number; maxStorageBufferBindingSize: number } | null;
  features: string[];
  timestampQuery: boolean;
  error: string | null;
}

export interface WebGL2Caps {
  available: boolean;
  maxTextureSize: number | null;
  extensions: string[];
  timerQuery: boolean;
}

export interface Capabilities {
  webgpu: WebGPUCaps;
  webgl2: WebGL2Caps;
  /** Chromium reports at most 8. null when unsupported. */
  deviceMemory: number | null;
  hardwareConcurrency: number | null;
}

/** Minimal structural types so tests can mock them without DOM/WebGPU typings. */
export interface GpuAdapterLike {
  info?: { vendor?: string; architecture?: string; device?: string; description?: string };
  limits?: { maxTextureDimension2D?: number; maxStorageBufferBindingSize?: number };
  features?: Iterable<string>;
}
export interface NavigatorLike {
  gpu?: { requestAdapter(): Promise<GpuAdapterLike | null | undefined> };
  deviceMemory?: number;
  hardwareConcurrency?: number;
}
export interface Gl2Like {
  MAX_TEXTURE_SIZE: number;
  getParameter(p: number): unknown;
  getSupportedExtensions(): string[] | null;
}

export async function probeWebGPU(nav: NavigatorLike): Promise<WebGPUCaps> {
  const empty: WebGPUCaps = { hasApi: false, adapter: false, info: null, limits: null, features: [], timestampQuery: false, error: null };
  if (!nav.gpu) return { ...empty, error: 'navigator.gpu is undefined' };
  try {
    const adapter = await nav.gpu.requestAdapter();
    if (!adapter) return { ...empty, hasApi: true, error: 'requestAdapter() returned null' };
    const features = adapter.features ? Array.from(adapter.features) : [];
    const i = adapter.info;
    return {
      hasApi: true,
      adapter: true,
      info: i ? { vendor: i.vendor ?? '', architecture: i.architecture ?? '', device: i.device ?? '', description: i.description ?? '' } : null,
      limits: adapter.limits
        ? {
            maxTextureDimension2D: adapter.limits.maxTextureDimension2D ?? 0,
            maxStorageBufferBindingSize: adapter.limits.maxStorageBufferBindingSize ?? 0,
          }
        : null,
      features,
      timestampQuery: features.includes('timestamp-query'),
      error: null,
    };
  } catch (e) {
    return { ...empty, hasApi: true, error: `requestAdapter() threw: ${errMsg(e)}` };
  }
}

export function probeWebGL2(gl: Gl2Like | null): WebGL2Caps {
  if (!gl) return { available: false, maxTextureSize: null, extensions: [], timerQuery: false };
  const extensions = gl.getSupportedExtensions() ?? [];
  const size = gl.getParameter(gl.MAX_TEXTURE_SIZE);
  return {
    available: true,
    maxTextureSize: typeof size === 'number' ? size : null,
    extensions,
    timerQuery: extensions.includes('EXT_disjoint_timer_query_webgl2'),
  };
}

export async function probeCapabilities(nav: NavigatorLike, createGl2?: () => Gl2Like | null): Promise<Capabilities> {
  const gl = safeCreate(createGl2);
  return {
    webgpu: await probeWebGPU(nav),
    webgl2: probeWebGL2(gl),
    deviceMemory: typeof nav.deviceMemory === 'number' ? nav.deviceMemory : null,
    hardwareConcurrency: typeof nav.hardwareConcurrency === 'number' ? nav.hardwareConcurrency : null,
  };
}

function safeCreate(f?: () => Gl2Like | null): Gl2Like | null {
  try {
    return f ? f() : null;
  } catch {
    return null;
  }
}

export function errMsg(e: unknown): string {
  return e instanceof Error ? e.message : String(e);
}
