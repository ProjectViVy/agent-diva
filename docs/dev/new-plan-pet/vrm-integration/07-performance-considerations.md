# 07 - VRM 性能考量

> VRM 3D 渲染的性能基准与优化策略

---

## 1. 性能目标

| 指标 | 目标值 | 备注 |
|------|--------|------|
| VRM 首帧时间 | < 5s | 含模型加载 + Three.js 初始化 |
| 空闲帧率 | ≥ 30fps | 仅 idle 动画 + springBone |
| 活跃帧率 | ≥ 55fps | 动画播放 + 表情切换 |
| GPU 占用 (空闲) | < 15% | 集成显卡 |
| GPU 占用 (活跃) | < 25% | 集成显卡 |
| 内存占用 (VRM) | < 200MB | 单个 VRM 模型 |
| VRM 文件大小 | < 30MB (推荐) | VRoid Studio 默认导出约 5-15MB |

---

## 2. 帧时间分解

```
单帧渲染时间 (~10ms @ 60fps):
├── vrm.update(): ~3ms
│   ├── springBone.update()          ~1.5ms (最重)
│   ├── humanoid.update()            ~0.5ms
│   ├── expressionManager.update()   ~0.5ms
│   └── lookAt.update()              ~0.5ms
├── mixer.update(): ~1ms
│   └── 动画混合                     ~1ms
├── renderer.render(): ~4ms
│   ├── shadow map                   ~1ms (如果启用)
│   ├── mesh rendering               ~2.5ms
│   └── post-processing              ~0.5ms
└── 其他: ~2ms
```

---

## 3. 性能优化策略

### 3.1 DPR 限制

```typescript
// 避免高 DPI 屏幕渲染过大 Canvas
renderer.setPixelRatio(Math.min(window.devicePixelRatio, 2))
```

### 3.2 阴影优化

```typescript
// MVP 阶段不启用阴影，减少 30% 渲染开销
renderer.shadowMap.enabled = false  // MVP default

// v2 阶段可选：
renderer.shadowMap.enabled = true
renderer.shadowMap.type = THREE.PCFSoftShadowMap
directionalLight.shadow.mapSize.set(512, 512)  // 低分辨率阴影
```

### 3.3 SpringBone 限制

```typescript
// VRM SpringBone 是最大性能瓶颈，限制更新频率
// @pixiv/three-vrm 默认每帧更新，可接受

// 空闲时可降低 springBone 精度（v2 优化）
if (!isActive && vrm.springBone) {
  // 可考虑跳过部分 springBone 更新（需 SDK 支持）
}
```

### 3.4 页面不可见时暂停

```typescript
document.addEventListener('visibilitychange', () => {
  if (document.hidden) {
    stopRenderLoop()
  } else {
    startRenderLoop()
  }
})
```

### 3.5 VRMUtils 优化

```typescript
// 加载后立即优化
VRMUtils.removeUnnecessaryJoints(gltf.scene)  // 移除冗余骨骼
VRMUtils.removeUnnecessaryVertices(gltf.scene) // 移除重复顶点 (可选)
```

### 3.6 纹理压缩

```typescript
// Three.js 支持 KTX2 压缩纹理
// 预先将 VRM 贴图转换为 KTX2/Basis 格式可减少 GPU 内存 ~50%
// 但会增加构建复杂度，推荐 v2 阶段实现
```

---

## 4. 与 Live2D 性能对比

| 指标 | VRM (3D) | Live2D (2D) |
|------|----------|-------------|
| 空闲帧时间 | ~8ms | ~3ms |
| 活跃帧时间 | ~12ms | ~6ms |
| 空闲 GPU | ~12% | ~5% |
| 内存 | ~120MB | ~80MB |
| 模型大小 | 5-30MB | 1-5MB |
| 支持 60fps? | ✅ (集成显卡) | ✅ |

> **结论**：VRM 性能开销约为 Live2D 的 2x，但仍在可控范围。集成显卡（Intel UHD / AMD Vega）可保持 30-60fps。

---

## 5. 加载性能

### 5.1 启动时序

```
进入 VRM 页面:
├── T+0ms      Three.js 初始化 (Scene/Camera/Renderer)
├── T+100ms    GLTFLoader 开始加载 .vrm
├── T+1000ms   下载完成 (本地文件，~5MB)
├── T+1500ms   解析 GLTF JSON + 解码贴图
├── T+2000ms   VRMLoaderPlugin 初始化 (骨骼/表情/SpringBone)
├── T+2200ms   VRMUtils 优化
├── T+2500ms   添加到 Scene，首帧渲染
└── T+3000ms   目标：< 5s 首帧
```

### 5.2 模型缓存

```typescript
// 缓存已加载的 VRM 实例，避免重复加载
const vrmCache = new Map<string, VRM>()

async function loadVrmCached(path: string): Promise<VRM> {
  if (vrmCache.has(path)) return vrmCache.get(path)!

  const vrm = await loadVrm(path)
  vrmCache.set(path, vrm)
  return vrm
}
```

---

## 6. 包体积

| 新增依赖 | 大小 (min) | 大小 (gzip) |
|----------|-----------|-------------|
| `three` | ~600KB | ~150KB |
| `@pixiv/three-vrm` | ~200KB | ~50KB |
| `@pixiv/three-vrm-animation` | ~50KB | ~15KB |
| **合计** | **~850KB** | **~215KB** |

> 对比 Live2D：vendor 脚本 (~200KB) + live2dcubismcore (~1.5MB) ≈ 1.7MB。VRM 更小。

---

## 7. 性能回归测试

```typescript
describe('VRM Performance', () => {
  it('should maintain 30fps during idle', async () => {
    const fps = await measureFPS(10000, 'idle')
    expect(fps).toBeGreaterThanOrEqual(28)
  })

  it('should maintain 55fps during animation', async () => {
    const fps = await measureFPS(5000, 'active')
    expect(fps).toBeGreaterThanOrEqual(50)
  })

  it('should not leak memory over 200 frames', async () => {
    const memBefore = performance.memory?.usedJSHeapSize
    await runFrames(200)
    const memAfter = performance.memory?.usedJSHeapSize
    if (memBefore && memAfter) {
      expect(memAfter - memBefore).toBeLessThan(20 * 1024 * 1024)
    }
  })
})
```
