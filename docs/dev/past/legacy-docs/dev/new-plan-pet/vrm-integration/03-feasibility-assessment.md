# 03 - VRM 可行性评估

> 评估将 super-agent-party 的 VRM 方案整合到 Agent Diva 的可行性

---

## 1. 总体结论

| 维度 | 评估 | 说明 |
|------|------|------|
| **技术可行性** | ⭐⭐⭐⭐⭐ (100%) | Three.js + @pixiv/three-vrm 均为 npm 包，Vite 6 完全兼容 |
| **许可可行性** | ⭐⭐⭐⭐⭐ (100%) | 所有核心库均为 MIT，无 EULA 限制 |
| **开发工作量** | ⭐⭐⭐ (中等) | 约 5 天（基于官方示例从头实现） |
| **模型生态** | ⭐⭐⭐⭐⭐ | VRoid Hub 数十万模型，VRoid Studio 免费制作 |
| **与 Live2D 相比** | ⭐⭐⭐⭐ | 3D 更沉浸，许可更自由，但 GPU 开销略高 |

**最终结论：✅ 完全可行，推荐作为首选渲染引擎。**

---

## 2. 许可分析（关键）

| 组件 | 许可 | 风险 |
|------|------|------|
| `@pixiv/three-vrm` v3.x | MIT | ✅ 零风险 |
| `@pixiv/three-vrm-animation` | MIT | ✅ 零风险 |
| `three` (Three.js) | MIT | ✅ 零风险 |
| super-agent-party 源码 | **AGPL-3.0** | ❌ 不可复制 |
| VRM 模型 (VRoid / Booth) | 各模型独立许可 | ⚠️ 需确认单个模型许可 |

> **对比 Live2D**：Live2D Cubism SDK 为专有许可，发布需遵守 EULA。VRM 方案无此限制。

---

## 3. 技术可行性

### 3.1 兼容性矩阵

| 检查项 | 状态 | 说明 |
|--------|------|------|
| Vite 6 构建 | ✅ | ES module imports 完全兼容 |
| Vue 3 集成 | ✅ | Canvas 渲染不依赖 Vue 版本 |
| Tauri 2 WebView2 | ✅ | WebGL 2.0 完整支持 |
| TypeScript | ✅ | @pixiv/three-vrm 有完整类型声明 |
| macOS | ✅ | Metal/WebGL 均支持 |
| Linux | ✅ | WebGL 支持 (需 GPU 驱动) |

### 3.2 与 Live2D 对比

| 维度 | VRM (@pixiv/three-vrm) | Live2D (Cubism SDK 5) |
|------|------------------------|------------------------|
| 许可 | MIT ✅ | 专有 EULA ⚠️ |
| npm 安装 | `pnpm add @pixiv/three-vrm` | 需手动放置 vendor 脚本 |
| 模型格式 | `.vrm` (标准 glTF) | `.model3.json` + `.moc3` (专有) |
| 模型制作 | VRoid Studio (免费) | Live2D Cubism Editor (付费) |
| 模型来源 | VRoid Hub (数十万) | 极少 (Cubism 5 格式) |
| 渲染维度 | 3D | 2D |
| 口型同步 | 原生支持 | 不原生支持 |
| 3D 场景 | ✅ | ❌ |
| GPU 开销 | 中高 (~15% GPU) | 低 (~5% GPU) |

---

## 4. 风险评估

| 风险 | 概率 | 影响 | 缓解 |
|------|------|------|------|
| Three.js 版本与 @pixiv/three-vrm 不兼容 | 低 | 高 | 版本锁定 |
| WebGL 在低端设备性能不足 | 中 | 中 | DPR cap + 降级到 Live2D |
| VRM 1.0 模型生态初期模型较少 | 低 | 低 | 同时支持 VRM 0.x |
| 需从零实现（不可复用 AGPL 代码） | 确定 | 中 | 官方示例足够，约 500 行核心逻辑 |
| VRM 模型许可不明确 | 中 | 中 | 使用明确许可的模型 + README 说明 |

---

## 5. 与 Live2D 方案的关系

**推荐策略**：VRM 优先，Live2D 备选。

```
用户选择渲染器：
├── VRM (默认)  → MIT 许可，3D 渲染，模型丰富
├── Live2D      → 2D 渲染，低 GPU 开销（备选）
└── 静态图       → 最低开销，无需任何模型
```

**不推荐完全放弃 Live2D 方案**，原因：
1. AniPet 的 Live2D 代码已经分析完毕，迁移成本已沉没
2. Live2D 在低端设备上 GPU 压力更低
3. 一些用户可能已有 Live2D 模型资产

---

## 6. 决策矩阵

| 决策 | 推荐方案 | 理由 |
|------|----------|------|
| VRM 渲染库 | `@pixiv/three-vrm` v3.x | 官方维护，MIT 许可，VRM 1.0 支持 |
| 模型格式 | VRM 1.0 (优先) + VRM 0.x (兼容) | VRoid Studio 默认导出 |
| 口型同步 | 简版正弦波 (MVP) → Audio Analyser (v2) | 先快后好 |
| 3D 场景 | 纯色背景 (MVP) → Gaussian Splatting (v2) | 先简后繁 |
| VRM 与 Live2D 关系 | VRM 优先，Live2D 备选 | 双引擎可切换 |

---

## 7. Go/No-Go 建议

### ✅ Go
- [x] 核心库 MIT 许可
- [x] npm 包可直接安装
- [x] 官方文档 + 示例完善
- [x] 与现有 session 架构完全兼容
- [x] 开发工时可接受（5 天）

### ⚠️ 前置条件
- [ ] 准备 1-2 个明确允许使用的 VRM 模型用于测试和分发
- [ ] 确认 Three.js + @pixiv/three-vrm 在 WebView2 的 WebGL 性能达标

### 建议
**推荐启动 VRM MVP 开发**，优先于 Live2D，以规避 Live2D SDK 许可风险。
