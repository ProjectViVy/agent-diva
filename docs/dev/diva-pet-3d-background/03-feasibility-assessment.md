# 03 — 可行性评估

## 1. 技术可行性

### 1.1 依赖兼容性

| 依赖 | 当前 | 需要 | 兼容 |
|------|------|------|------|
| `three` | 0.184.0 | ≥ 0.160 | ✅ 184 > 160 |
| `@pixiv/three-vrm` | 3.5.2 | 3.x | ✅ 无需变更 |
| `@sparkjsdev/spark` | 未安装 | 最新 | ⚠️ 需验证 Three 184 兼容 |
| `vue` | 3.5.13 | 3.x | ✅ 无影响 |
| `@tauri-apps/api` | 2.x | 2.x | ✅ |

### 1.2 运行时就绪度

| 模块 | 状态 | 说明 |
|------|------|------|
| `GaussSceneController` | ✅ | 完整: load/unload/dispose, 3 种预设 + 自定义 |
| `SceneManager.setBackgroundScene()` | ✅ | 懒初始化 controller, 无副作用 |
| `VrmRuntime.setBackgroundScene()` | ✅ | 公开 API, 接受 sceneId + url |
| 场景切换安全 | ✅ | 递推 dispose, 卸载后才加载新场景 |
| 阴影系统 | ✅ | 独立于背景场景 |

### 1.3 SplatMesh 与 Three.js 渲染管线兼容性分析

**内嵌模式下的潜在问题**:
1. **Alpha blending 顺序**: SplatMesh 的自定义 shader 使用 `THREE.CustomBlending`，与 VRM 的 `THREE.NormalBlending` 共存需要正确的 `renderOrder` 和 `depthTest/depthWrite` 设置
2. **Clear color 覆盖**: 场景 SplatMesh 渲染在 `renderer.clear()` 之后，需确保覆盖整个 canvas 可见区域
3. **视口裁剪**: PerspectiveCamera 的近/远平面 (0.1/100) 足够覆盖场景的距离范围

**缓解措施**: `GaussSceneController` 已正确设置 `renderOrder`，场景 Group 先于模型 Group 加入 scene → Three.js 自动按顺序渲染。

---

## 2. 架构可行性

### 2.1 修改范围

| 层 | 文件数 | 修改行数 (估) |
|----|--------|-------------|
| 运行时 | 0 | 0 |
| 类型/配置 | 1-2 | +40 |
| VRM 组件 | 1 | +30 |
| Pet 视图 | 1 | +50 |
| 设置面板 | 1 | +50 |
| 依赖/资源 | 1-2 | +5 |
| **总计** | **4-5** | **~175** |

### 2.2 现有功能影响

| 功能 | 影响 | 说明 |
|------|------|------|
| VRM 渲染 | 无 | 场景 Group 与模型 Group 独立 |
| 动画系统 | 无 | 场景不参与 AnimationMixer |
| 表情系统 | 无 | expressionManager 仅操作 VRM blendshapes |
| 语音 + 字幕 | 无 | 独立于渲染层 |
| PTT 录音 | 无 | 按钮在 .chat-section 中 |
| CSS 渐变背景 | 部分覆盖 | Canvas 内场景覆盖渐变 |
| 聊天区域 | 无 | .chat-section 覆盖在下部，独立于 canvas |

### 2.3 向后兼容

- `selectedGaussSceneId: 'transparent'` → 行为完全不变
- 新字段全有默认值 → 旧配置自动合并
- 未安装场景文件 → 自动回退 transparent

---

## 3. 性能可行性

### 3.1 SplatMesh 渲染性能

| 指标 | 透明 (基线) | indoor (200K) | sea (500K) | space (100K) |
|------|-----------|---------------|------------|--------------|
| FPS (估) | 60 | 45-55 | 30-45 | 50-60 |
| JS heap 增量 | 0 | +30-80 MB | +50-150 MB | +15-40 MB |
| GPU VRAM 增量 | 0 | +50-150 MB | +100-300 MB | +20-70 MB |

### 3.2 内嵌模式的性能优势

vs 桌面覆盖层模式 (独立窗口):
- **无需窗口合成**: 不涉及 Tauri 透明窗口 → 桌面壁纸的 GPU 合成
- **共享 WebGL context**: 主窗口唯一 WebGL context，无需跨窗口同步
- **视口更小**: canvas 尺寸受 `.avatar-section` 约束，像素填充率更低

---

## 4. 风险矩阵

| 风险 | 概率 | 影响 | 缓解 |
|------|------|------|------|
| spark 与 Three 184 不兼容 | 低 | 高 | 备选: vendor alias |
| .spz 文件过大 (内存) | 中 | 中 | 提供场景选项，按需加载 |
| GPU 不支持 WebGL | 极低 | 高 | 自动回退 transparent |
| 场景 + 阴影渲染重叠 | 低 | 低 | 禁用独立阴影地面 |

---

## 5. 总结

| 维度 | 评估 |
|------|------|
| 技术可行性 | ✅ 极高 (运行时已就绪) |
| 架构可行性 | ✅ 高 (修改集中于前端) |
| 性能可行性 | ✅ 高 (内嵌模式视口小) |
| 向后兼容 | ✅ 高 (默认透明) |
| 工作量 | ✅ 低 (~4h) |
| 风险 | ✅ 低 (3 个可控风险) |
