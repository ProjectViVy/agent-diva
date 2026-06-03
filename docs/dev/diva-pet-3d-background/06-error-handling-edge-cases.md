# 06 — 错误处理与边界条件

## 1. Three.js 渲染层的错误处理

### 1.1 SplatMesh 加载异常

`.spz` 文件可能因多种原因加载失败：

| 原因 | SplatMesh 行为 | 兜底方案 |
|------|---------------|----------|
| HTTP 404 / 文件不存在 | fetch reject → `new SplatMesh` 的 Promise reject | catch → `setBackgroundScene('transparent')` |
| 文件损坏 (二进制不完整) | 解码阶段 throw | 同上 |
| 文件过大导致 OOM | JS 堆溢出 → 浏览器抛异常 | 同上 + console.error 记录文件大小 |
| 网络超时 (远程 spz) | fetch timeout | 同上 |

**关键实现** — DivaVrmAvatar 的双重 try-catch：

```typescript
async function syncBackgroundScene() {
  if (!runtime.value || !props.backgroundScene) return
  const seq = ++sceneLoadSeq
  try {
    await runtime.value.setBackgroundScene(props.backgroundScene, props.backgroundSceneUrl)
    if (seq !== sceneLoadSeq) return
  } catch (err) {
    if (seq !== sceneLoadSeq) return
    console.warn('[DivaVrmAvatar] 场景设置失败:', err)
    // 第二层兜底: 透明模式永远可用 (无需文件、无网络)
    try { await runtime.value.setBackgroundScene('transparent') } catch {}
  }
}
```

### 1.2 竞态条件 (快速切换场景)

用户 0.2s 内连点 3 次不同场景 → 3 个异步 SplatMesh 加载并行启动。

**内部保护**: `GaussSceneController.loadScene()` 的第一步是 `unloadCurrent()` — 立即卸载当前场景 Group。即使 3 个异步加载交叉完成，每次 `loadScene` 都先清除旧 Group → 旧 Group 的 SplatMesh (包含旧的 GPU buffer) 被 dispose，不会与新 Group 叠加。

**外部保护**: `sceneLoadSequence` 递增序号：
- 请求 A (seq=1) 加载 home.spz (2 秒)
- 请求 B (seq=2) 加载 sea.spz (3 秒)
- 请求 C (seq=3) 加载 transparent (立即)
- C 完成 → seq=3 匹配 → 场景正确为 transparent
- A 完成 → seq=1 ≠ seq=3 → 丢弃结果（场景已被 C 设置）
- B 完成 → seq=2 ≠ seq=3 → 丢弃结果

### 1.3 WebGL Context Lost

Three.js 默认处理 context lost/restored：
- `contextlost` 事件: 渲染器暂停 (renderer 不再调用 draw)
- `contextrestored` 事件: 重新上传纹理/buffer → 场景恢复正常

Gaussian Splat 的 SplatMesh buffer (positions/scales/rotations/colors/opacities) 在 context restore 后需重新上传。SplatMesh 内部由 `BufferGeometry` 管理，Three.js 的 `onContextRestore` 机制自动处理。

**额外保护**: 在 context lost 期间，避免任何 `renderer.render()` 调用 → SceneManager 的 `scheduleFrame()` 中 `this.paused || this.destroyed` 检查。

---

## 2. 场景图边界条件

### 2.1 场景 Group 与模型 Group 的 transform 隔离

两个 Group 共享同一个 `THREE.Scene`，但 transform 完全独立：

```
scene
├─ Group: gaussScene_xxx       ← 场景变换 (position/scale)
│   └─ SplatMesh               ← 场景 splat 局部坐标
└─ Group: modelWrapper         ← 模型变换 (TransformController)
    └─ VRM.scene               ← 模型骨骼局部坐标
```

- 场景的 `position.set(0, height, 2)` 不改变模型位置
- 用户缩放模型 (TransformController) 不改变场景 scale
- 相机 OrbitControls 同时影响两者 (场景和模型在同一 view 中)

### 2.2 场景文件路径解析

- **Vite dev**: `/vrm/scene/home.spz` → Vite 从 `public/` 目录 serve
- **Tauri production**: `public/` 打包进 `dist/`，Tauri 用自定义协议 serve 静态文件
- **自定义 URL**: 如果 `backgroundSceneUrl` 是完整 URL，直接传给 SplatMesh

### 2.3 透明模式 vs 不透明内嵌 Canvas

内嵌模式下 canvas 的 clearColor 是不透明的：
- `transparent` 场景 → canvas 显示 renderer.clearColor (与 .diva-pet-view 的 CSS 渐变背景配合)
- `home`/`sea`/`space` → 3D 场景填充 canvas，遮蔽 clearColor

视觉过渡: 从 transparent 切换到 3D 场景时，canvas 内容从 clearColor → 3D SplatMesh（可能有一帧闪烁）。可通过 `renderer.setClearColor` 设置为与场景色调接近的颜色来减少闪烁。

---

## 3. 组件生命周期边界

### 3.1 DivaPetView 的 v-show 切换

`DivaVrmAvatar` 通过 `v-show="!desktopPetActive"` 控制可见性：
- `desktopPetActive=true` → Avatar v-show=false → Canvas display:none → 场景不渲染
- `desktopPetActive=false` → Avatar v-show=true → 恢复渲染

场景状态在 v-show 切换中保持不变 (GaussSceneController 内部的 Group 仍在 scene 中，只是 Canvas 不可见时不 render)。

### 3.2 DivaVrmAvatar 卸载

`onUnmounted` → `runtime.destroy()` → `gaussController.dispose()` → 卸载场景 Group、释放 GPU 资源。所有 buffer/texture 被 Three.js dispose 方法释放。

### 3.3 模型切换与场景的关系

```
用户切换 VRM 模型 (DivaPetModelManager 中选择新模型)
  → DivaPetView.onModelChanged → updateConfig({ vrmModel })
  → DivaVrmAvatar watch modelPath → loadCurrentCharacter(true)
    → unloadCurrentModel()    ← 仅移除 VRM.scene
    → modelLoader.load()      ← 加载新 VRM
    → scene.add(vrm.scene)    ← 加入场景图
```

**关键**: `unloadCurrentModel()` 在 `vrm-runtime.ts:547` 中仅操作 `this.currentModel.scene`，不触碰 `gaussController.currentGroup`。场景 Group 保持不变。

---

## 4. 日志策略

| 级别 | 标签 | 内容 |
|------|------|------|
| `log` | `[SceneLoader]` | `场景 ${id} 加载完成` |
| `warn` | `[DivaVrmAvatar]` | `背景场景设置失败: ${err}` |
| `error` | `[DivaVrmAvatar]` | `回退透明模式也失败` (极端) |

无弹窗/Toast → 静默处理，匹配 super-agent-party 行为。
