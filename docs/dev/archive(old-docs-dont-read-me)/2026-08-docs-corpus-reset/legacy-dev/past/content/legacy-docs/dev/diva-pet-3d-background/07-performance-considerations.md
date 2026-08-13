# 07 — 性能考量

## 1. Gaussian Splat 渲染性能分析

### 1.1 GPU 管线开销

SplatMesh 的每帧 GPU 工作：

```
Vertex Shader (每 splat):
  - 投影 3D 中心 → 屏幕坐标
  - 计算 2D 协方差椭圆 (J·W·Σ·W^T·J^T)
  - 传递椭圆参数给 fragment shader

Fragment Shader (每像素, 覆盖该像素的所有 splat):
  - 计算 splat 在该像素的高斯衰减值: exp(-½Δx^T Σ'⁻¹ Δx)
  - Alpha blending: C_out = C_in + c·α·T,  T *= (1-α)
  - SH 颜色评估 (需要在 fragment shader 中计算球谐函数)
```

**复杂度**: O(splatCount × avgCoverage) + O(pixelCount × avgSplatsPerPixel)

- splatCount: 100K ~ 1M
- avgCoverage: 每个 splat 覆盖的像素数 (与 splat 大小、距离相关)
- avgSplatsPerPixel: 每个像素累积的 splat 数 (与场景密度相关)

**最坏情况**: 海边场景 (sea) — 500K+ splats, 覆盖整个视口, 高 alpha blending 深度。

### 1.2 内嵌模式下的性能特征

vs 桌面覆盖层 (独立透明窗口):

| 因素 | 内嵌 (DivaPetView) | 桌面覆盖层 |
|------|-------------------|-----------|
| Canvas 尺寸 | ~300×400 (受 .avatar-section 约束) | 400×600 (全窗口) |
| 填充像素 | ~120K | ~240K |
| 每像素 splat 数 | ~2-5 | ~5-10 (更大视口) |
| 窗口合成 | 无 (单个 Tauri 窗口) | 有 (透明窗口需与桌面合成) |
| FPS 上限 | 60 | 24 |

**结论**: 内嵌模式性能优于桌面覆盖层 — Canvas 更小，无窗口合成开销。

### 1.3 性能优化手段 (内置)

SplatMesh 已实现的优化：

1. **View frustum culling**: 不在视口内的 splat 不在 vertex shader 中处理
2. **Depth sorting**: CPU 或 GPU 预排序，确保 alpha blending 正确
3. **SH 阶数控制**: 默认 0-3 阶 (1-16 个系数)，可降为 0 阶 (仅 diffuse 颜色)
4. **Splat 数量裁剪**: 距离过远的 splat 可跳过

---

## 2. 内存管理

### 2.1 GPU 内存 (VRAM)

| 资源 | 大小计算 | 示例 (500K splats) |
|------|---------|-------------------|
| positions | N × 3 × 4B | 6 MB |
| scales | N × 3 × 4B | 6 MB |
| rotations | N × 4 × 4B | 8 MB |
| colors (SH) | N × 16 × 4B (0-3 阶) | 32 MB |
| opacities | N × 1 × 4B | 2 MB |
| .spz 纹理 (如有) | 可变 | ~0 |
| **总计/场景** | | **~54 MB** |

3 个预设场景 (home + sea + space) 同时加载: ~150 MB VRAM。实际仅当前场景占用。

### 2.2 JS 堆内存

- .spz 解码后的 Float32Array: ~50 MB (500K splats)
- SplatMesh 对象开销: ~1 MB
- BufferGeometry + BufferAttribute: ~2 MB

**场景切换时**: `unloadCurrent()` → `disposeObject()` → 释放所有 GPU buffer + JS 堆引用。GC 在下一帧回收。

### 2.3 内存泄漏预防

- `GaussSceneController.dispose()` 递归清理所有子对象
- `SceneManager.destroy()` 调用 `gaussController.dispose()` → `renderer.dispose()`
- `DivaVrmAvatar.onUnmounted()` 调用 `runtime.destroy()` → 完整清理链

**验证**: Chrome DevTools Memory → 录制 allocation timeline → 场景切换 5 次 → 对比 JS heap 大小 → 应无明显增长。

---

## 3. 加载性能

### 3.1 .spz 文件加载

```
流程: fetch(url) → ArrayBuffer → 二进制解码 → Float32Array × 5 → GPU buffer upload
```

| 步骤 | 耗时 (500K splats, ~50MB) | 优化 |
|------|--------------------------|------|
| fetch | 0.1-0.5s (本地文件) | 无需优化 |
| 二进制解码 | 0.5-2s | SplatMesh 内部已优化 |
| GPU buffer upload | 0.05-0.5s | 异步，不阻塞渲染 |

**总加载时间**: 1-3 秒 (本地 .spz)

### 3.2 加载期间的 UI 状态

SplatMesh 构造函数 `new SplatMesh({ url })` 是异步的 — 内部 fetch → decode → 返回。在此期间:
- Three.js 渲染循环继续运行 (模型正常渲染)
- 场景 Group 已 `scene.add(group)`，但 Group 内无子对象
- SplatMesh 加载完成后自动出现在场景中 (无闪烁过渡)

---

## 4. 性能测试建议

```javascript
// 在 devtools console 中获取实时指标
const metrics = await runtime.getMetrics()
// { fps: 55.2, frameTimeMs: 18.1, memoryMb: 180.3 }

// 场景切换内存对比
performance.memory.usedJSHeapSize  // 前
await switchScene('home')
performance.memory.usedJSHeapSize  // 后 → 取差值
```
