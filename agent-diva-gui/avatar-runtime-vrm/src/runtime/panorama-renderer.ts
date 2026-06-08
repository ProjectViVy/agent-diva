import * as THREE from 'three'

/**
 * PanoramaRenderer manages CubeCamera-based 360° equirectangular panorama output.
 *
 * It captures the main Three.js scene with a CubeCamera positioned at the model's
 * head height, then projects the cubemap as a 2:1 equirectangular image onto the
 * canvas — suitable for OBS streaming or 360° previews.
 *
 * The default render mode is "normal" (perspective camera). When panorama mode is
 * enabled via {@link enable}, each frame replaces the standard render call with
 * {@link render}.
 *
 * Reference: super-agent-party/static/js/vrm.js lines 211-268 (setup) and
 * 2219-2243 (render loop).
 */
export class PanoramaRenderer {
  /** Whether panorama mode is currently active. */
  enabled: boolean = false

  /** Cubemap render target (2048×2048 per face). */
  readonly cubeRenderTarget: THREE.WebGLCubeRenderTarget

  /** Captures 6-face cubemap of the main scene. */
  readonly cubeCamera: THREE.CubeCamera

  /** Shader that converts the cubemap texture into an equirectangular projection. */
  readonly panoShaderMaterial: THREE.ShaderMaterial

  /** Full-screen plane that displays the equirectangular result. */
  readonly panoMesh: THREE.Mesh

  /** Dedicated scene containing only {@link panoMesh}. */
  readonly panoScene: THREE.Scene

  /** Orthographic camera rendering the full-screen projection plane. */
  readonly panoCamera: THREE.OrthographicCamera

  constructor(private readonly renderer: THREE.WebGLRenderer) {
    // ── Cubemap render target (2048 px per face) ─────────────────────────
    this.cubeRenderTarget = new THREE.WebGLCubeRenderTarget(2048, {
      format: THREE.RGBAFormat,
      generateMipmaps: true,
      magFilter: THREE.LinearFilter,
    })

    // ── CubeCamera ───────────────────────────────────────────────────────
    this.cubeCamera = new THREE.CubeCamera(0.1, 1000, this.cubeRenderTarget)

    // ── Equirectangular projection shader ────────────────────────────────
    // Converts the 6-face cubemap into a 2:1 equirectangular image.
    this.panoShaderMaterial = new THREE.ShaderMaterial({
      uniforms: {
        tCube: { value: this.cubeRenderTarget.texture },
      },
      vertexShader: /* glsl */ `
        varying vec2 vUv;
        void main() {
          vUv = uv;
          gl_Position = projectionMatrix * modelViewMatrix * vec4(position, 1.0);
        }
      `,
      fragmentShader: /* glsl */ `
        varying vec2 vUv;
        uniform samplerCube tCube;
        #define PI 3.141592653589793
        void main() {
          float longitude = vUv.x * 2.0 * PI;
          float latitude = vUv.y * PI - PI / 2.0;
          vec3 dir;
          dir.x = cos(latitude) * sin(longitude);
          dir.y = sin(latitude);
          dir.z = cos(latitude) * cos(longitude);
          gl_FragColor = textureCube(tCube, dir);
        }
      `,
      side: THREE.DoubleSide,
    })

    // ── Full-screen projection plane ─────────────────────────────────────
    this.panoMesh = new THREE.Mesh(
      new THREE.PlaneGeometry(2, 2),
      this.panoShaderMaterial,
    )

    // ── Dedicated scene for the projection plane ─────────────────────────
    this.panoScene = new THREE.Scene()
    this.panoScene.add(this.panoMesh)

    // ── Orthographic camera for rendering the projection plane ───────────
    this.panoCamera = new THREE.OrthographicCamera(-1, 1, 1, -1, 0, 1)
  }

  /**
   * Enable panorama rendering mode.
   *
   * @param position - World-space position for the CubeCamera (typically
   *   model head height, e.g. `(0, 1.5, 1)`).
   */
  enable(position: THREE.Vector3): void {
    this.cubeCamera.position.copy(position)
    this.enabled = true
  }

  /** Disable panorama rendering mode (return to normal perspective). */
  disable(): void {
    this.enabled = false
  }

  /**
   * Render one panorama frame:
   * 1. Hide the projection plane so it does not appear in the cubemap capture.
   * 2. Update the CubeCamera to capture the scene from 6 directions.
   * 3. Show the projection plane.
   * 4. Render the equirectangular result to the canvas.
   *
   * Call this INSTEAD of `renderer.render(scene, camera)` when panorama mode
   * is enabled.
   */
  render(scene: THREE.Scene): void {
    // Step 1 — hide projection plane so CubeCamera doesn't capture it
    this.panoMesh.visible = false

    // Step 2 — capture 360° cubemap
    this.cubeCamera.update(this.renderer, scene)

    // Step 3 — restore projection plane visibility
    this.panoMesh.visible = true

    // Step 4 — render equirectangular projection to canvas
    this.renderer.render(this.panoScene, this.panoCamera)
  }

  /** Release all GPU resources. */
  dispose(): void {
    this.cubeRenderTarget.dispose()
    this.panoShaderMaterial.dispose()
    this.panoMesh.geometry.dispose()
  }
}
