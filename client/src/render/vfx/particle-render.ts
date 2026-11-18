/**
 * GPU drawing of the particle pool (M1-047b, ARCHITECTURE 13.5.3): one billboard quad mesh, one draw call, all live
 * particles as thin instances rewritten from the pool each frame (`ParticlePool.writeInstances`, no allocation).
 * Shared by WebGL2 and WebGPU through the twin shaders in particle-shader.ts. CPU time of `update` is the measured
 * "particle update + upload" cost.
 */
import '@babylonjs/core/Meshes/thinInstanceMesh';
import type { Camera } from '@babylonjs/core/Cameras/camera';
import { ShaderMaterial } from '@babylonjs/core/Materials/shaderMaterial';
import { ShaderLanguage } from '@babylonjs/core/Materials/shaderLanguage';
import { Constants } from '@babylonjs/core/Engines/constants';
import { Vector3 } from '@babylonjs/core/Maths/math.vector';
import { BoundingInfo } from '@babylonjs/core/Culling/boundingInfo';
import { Mesh } from '@babylonjs/core/Meshes/mesh';
import { VertexData } from '@babylonjs/core/Meshes/mesh.vertexData';
import type { Scene } from '@babylonjs/core/scene';
import type { ParticlePool } from './particle-pool';
import { PARTICLE_FRAGMENT_GLSL, PARTICLE_FRAGMENT_WGSL, PARTICLE_VERTEX_GLSL, PARTICLE_VERTEX_WGSL } from './particle-shader';

export class ParticleRenderer {
  readonly mesh: Mesh;
  private readonly material: ShaderMaterial;
  private readonly buf: Float32Array;
  private readonly right = new Vector3();
  private readonly up = new Vector3();
  /** Instances uploaded by the last `update`. */
  drawn = 0;

  constructor(
    scene: Scene,
    private readonly pool: ParticlePool,
    wgsl: boolean,
  ) {
    this.buf = new Float32Array(pool.capacity * 16);
    this.mesh = new Mesh('PH_V-FX-001__particles', scene);
    const vd = new VertexData();
    vd.positions = [-0.5, -0.5, 0, 0.5, -0.5, 0, 0.5, 0.5, 0, -0.5, 0.5, 0];
    vd.indices = [0, 1, 2, 0, 2, 3];
    vd.applyToMesh(this.mesh);
    this.material = new ShaderMaterial(
      'PH_V-FX-001__particles_mat',
      scene,
      wgsl ? { vertexSource: PARTICLE_VERTEX_WGSL, fragmentSource: PARTICLE_FRAGMENT_WGSL } : { vertexSource: PARTICLE_VERTEX_GLSL, fragmentSource: PARTICLE_FRAGMENT_GLSL },
      {
        attributes: ['position'],
        uniforms: ['viewProjection', 'uRight', 'uUp'],
        shaderLanguage: wgsl ? ShaderLanguage.WGSL : ShaderLanguage.GLSL,
      },
    );
    this.material.backFaceCulling = false;
    this.material.alphaMode = Constants.ALPHA_COMBINE;
    this.material.needAlphaBlending = () => true;
    this.material.disableDepthWrite = true;
    this.mesh.material = this.material;
    this.mesh.isPickable = false;
    this.mesh.alwaysSelectAsActiveMesh = true;
    this.mesh.doNotSyncBoundingInfo = true;
    this.mesh.setBoundingInfo(new BoundingInfo(new Vector3(-1e5, -1e5, -1e5), new Vector3(1e5, 1e5, 1e5)));
    this.mesh.thinInstanceSetBuffer('matrix', this.buf, 16, false);
    this.mesh.thinInstanceCount = 0;
    this.mesh.setEnabled(false);
  }

  /** Advance the pool by `dtSec`, rewrite the instance buffer and orient the billboards towards `camera`. */
  update(camera: Camera, dtSec: number): void {
    this.pool.update(dtSec);
    const n = this.pool.writeInstances(this.buf);
    this.drawn = n;
    this.mesh.thinInstanceCount = n;
    this.mesh.setEnabled(n > 0);
    if (n === 0) return;
    this.mesh.thinInstanceBufferUpdated('matrix');
    const m = camera.getViewMatrix(true).m; // row-vector convention: the camera's right / up are the first two columns of the view rotation
    this.right.set(m[0]!, m[4]!, m[8]!);
    this.up.set(m[1]!, m[5]!, m[9]!);
    this.material.setVector3('uRight', this.right);
    this.material.setVector3('uUp', this.up);
  }

  dispose(): void {
    this.mesh.dispose(false, false);
    this.material.dispose();
  }
}
