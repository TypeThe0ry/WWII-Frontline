import { describe, expect, it } from 'vitest';
import { probeCapabilities, probeWebGL2, probeWebGPU, type NavigatorLike } from './capabilities';

describe('probeWebGPU', () => {
  it('reports navigator.gpu undefined', async () => {
    const r = await probeWebGPU({});
    expect(r.hasApi).toBe(false);
    expect(r.adapter).toBe(false);
    expect(r.error).toContain('navigator.gpu is undefined');
  });

  it('reports a null adapter', async () => {
    const r = await probeWebGPU({ gpu: { requestAdapter: async () => null } });
    expect(r.hasApi).toBe(true);
    expect(r.adapter).toBe(false);
    expect(r.error).toContain('null');
  });

  it('reports a thrown requestAdapter', async () => {
    const r = await probeWebGPU({
      gpu: {
        requestAdapter: () => Promise.reject(new Error('boom')),
      },
    });
    expect(r.adapter).toBe(false);
    expect(r.error).toContain('boom');
  });

  it('parses adapter info, limits and features', async () => {
    const r = await probeWebGPU({
      gpu: {
        requestAdapter: async () => ({
          info: { vendor: 'nvidia', architecture: 'ampere', device: '', description: 'RTX 3060' },
          limits: { maxTextureDimension2D: 16384, maxStorageBufferBindingSize: 2147483644 },
          features: new Set(['timestamp-query', 'texture-compression-bc']),
        }),
      },
    });
    expect(r.adapter).toBe(true);
    expect(r.info?.vendor).toBe('nvidia');
    expect(r.limits).toEqual({ maxTextureDimension2D: 16384, maxStorageBufferBindingSize: 2147483644 });
    expect(r.timestampQuery).toBe(true);
    expect(r.features).toContain('texture-compression-bc');
  });

  it('tolerates a missing adapter.info', async () => {
    const r = await probeWebGPU({ gpu: { requestAdapter: async () => ({ features: [] }) } });
    expect(r.adapter).toBe(true);
    expect(r.info).toBeNull();
    expect(r.timestampQuery).toBe(false);
  });
});

describe('probeWebGL2', () => {
  it('handles a missing context', () => {
    expect(probeWebGL2(null)).toEqual({ available: false, maxTextureSize: null, extensions: [], timerQuery: false });
  });

  it('parses MAX_TEXTURE_SIZE and extensions', () => {
    const r = probeWebGL2({
      MAX_TEXTURE_SIZE: 3379,
      getParameter: (p) => (p === 3379 ? 8192 : null),
      getSupportedExtensions: () => ['EXT_disjoint_timer_query_webgl2', 'WEBGL_compressed_texture_s3tc'],
    });
    expect(r.maxTextureSize).toBe(8192);
    expect(r.timerQuery).toBe(true);
  });
});

describe('probeCapabilities', () => {
  it('combines navigator fields; null when unsupported', async () => {
    const nav: NavigatorLike = { deviceMemory: 8, hardwareConcurrency: 12 };
    const r = await probeCapabilities(nav, () => null);
    expect(r.deviceMemory).toBe(8);
    expect(r.hardwareConcurrency).toBe(12);
    expect(r.webgl2.available).toBe(false);
    const r2 = await probeCapabilities({}, () => {
      throw new Error('no gl');
    });
    expect(r2.deviceMemory).toBeNull();
    expect(r2.hardwareConcurrency).toBeNull();
    expect(r2.webgl2.available).toBe(false);
  });
});
