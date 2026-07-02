# 05 - VRM 兼容性与迁移分析

> VRM 模块对现有系统的影响及许可兼容性

---

## 1. 平台兼容性

| 平台 | WebGL 2.0 | Three.js | @pixiv/three-vrm | 评估 |
|------|-----------|----------|-------------------|------|
| **Windows 10/11** | ✅ | ✅ | ✅ | 完全支持 |
| **macOS 12+** | ✅ | ✅ | ✅ | 完全支持 |
| **Linux (X11)** | ✅ | ✅ | ✅ | 需 GPU 驱动 |
| **Linux (Wayland)** | ✅ | ✅ | ✅ | 需 GPU 驱动 |

> GPU 最低要求：支持 WebGL 2.0（2017 年后几乎所有 GPU 均支持）

---

## 2. 对现有功能的影响

### 2.1 零影响模块

| 模块 | 影响 | 说明 |
|------|------|------|
| agent-diva-core | 零 | 不涉及核心逻辑 |
| agent-diva-providers | 零 | 不新增 Provider |
| agent-diva-channels | 零 | 不涉及通道 |
| ChatView | 零 | 不修改 Chat 组件 |
| sendMessage() | 零 | 复用现有 IPC |

### 2.2 低影响模块

| 模块 | 影响 | 说明 |
|------|------|------|
| package.json | +3 npm 依赖 | three, @pixiv/three-vrm, @pixiv/three-vrm-animation |
| DivaPetView | 新增 VRM 分支 | `v-if="renderer === 'vrm'"` |
| config.json | 新增 vrm section | 约 10 个配置字段 |
| 包体积 | +~800KB (gzip) | Three.js ~600KB + three-vrm ~200KB |

---

## 3. 许可合规（关键）

### 3.1 核心库许可

| 包 | 版本 | 许可 | 源 |
|----|------|------|-----|
| `three` | ^0.170 | MIT | npm |
| `@pixiv/three-vrm` | ^3.0 | MIT | npm / GitHub |
| `@pixiv/three-vrm-animation` | ^3.0 | MIT | npm / GitHub |

### 3.2 代码来源合规

| 来源 | 许可 | 可否使用 | 说明 |
|------|------|----------|------|
| @pixiv/three-vrm 官方示例 | MIT | ✅ | GitHub README 示例代码 |
| @pixiv/three-vrm 官方文档 | MIT | ✅ | https://pixiv.github.io/three-vrm/ |
| super-agent-party `static/js/vrm.js` (4573行) | **AGPL-3.0** | ❌ | **不可直接复制**任何代码行 |
| super-agent-party `static/vrm.html` (76行) | **AGPL-3.0** | ❌ | 不可复制 import map 配置 |
| super-agent-party 整体 | **AGPL-3.0** | ❌ | **不可作为依赖引入** |
| super-agent-party `vrm/animations/*.vrma` | 各动画独立许可 | ⚠️ | 需逐一确认许可 |
| super-agent-party `vrm/Alice.vrm` / `Bob.vrm` | 各模型独立许可 | ⚠️ | 需确认可否重新分发 |

> **实施原则**：基于 `@pixiv/three-vrm` 的 MIT 官方文档和示例自主实现，不参考 super-agent-party 源码。本文档中的行号引用仅供理解架构思路，不代表应复制对应代码。

### 3.3 VRM 模型许可

```
VRM 模型来源 & 典型许可：
├── VRoid Hub (https://hub.vroid.com/)
│   └── 各模型独立许可 (CC0, CC-BY, 等等)
├── VRoid Studio 自制
│   └── 用户拥有完全版权
├── Booth.pm
│   └── 各模型独立许可
└── 自由素材站
    └── 需逐一确认

建议：默认模型使用明确 CC0 许可的 VRM 文件
```

### 3.4 与 Live2D 许可对比

| 许可维度 | Live2D | VRM |
|----------|--------|-----|
| SDK 许可 | Live2D 专有 EULA | MIT |
| 可自由分发？ | 需遵守 EULA | ✅ 无限制 |
| 可商用？ | 需额外授权 | ✅ 无限制 (MIT) |
| 模型制作工具 | Cubism Editor (付费) | VRoid Studio (免费) |
| 模型分发 | 需遵守模型作者许可 | 需遵守模型作者许可 |

> **VRM 方案在许可层面显著优于 Live2D**。

---

## 4. 从 super-agent-party 的学习路径

由于不能复制 AGPL-3.0 代码，从 super-agent-party 学习的**合法方式**：

### ✅ 可以做的：
1. 研究其**架构思路**（Three.js + VRM 的整合方式，见 `vrm.html` import map 结构 L61-L72）
2. 参考其**功能清单**（表情系统、口型同步、动画播放，见 `vrm.js` 函数索引）
3. 了解其**VRM 资源组织方式**（`.vrm` / `.vrma` / `.spz` 文件结构，见 `vrm/` 目录）
4. 学习 **@pixiv/three-vrm 的 API 用法**（官方文档通用知识，非 AGPL 代码）
5. 参考其**空闲动画管理器的设计模式**（`IdleAnimationManager` class L536，仅思路层面）

### ❌ 不能做的：
1. 复制/粘贴 `vrm.js` 中的任何代码（即使是改写的"衍生作品"也违反 AGPL-3.0）
2. 复制 `vrm.html` 中的 import map 配置（L61-L72）
3. 将 super-agent-party 作为 npm/git 依赖引入
4. 分发 super-agent-party 的 `.vrma`/`.spz`/`.vrm` 文件（除非逐一确认许可）

---

## 5. 向后兼容性

### 5.1 配置向后兼容

```json
// 旧版 config.json 无 VRM section → 默认使用静态图，不崩溃
{
  "pet": {
    "enabled": true,
    "renderer": "static"   // 或缺失此字段
  }
}

// 新版 config.json
{
  "pet": {
    "enabled": true,
    "renderer": "vrm",     // "vrm" | "live2d" | "static"
    "vrm": { /* ... */ }
  }
}
```

### 5.2 运行时降级

```typescript
if (config.renderer === 'vrm' && !isWebGLAvailable()) {
  console.warn('WebGL 不可用，降级为静态图')
  config.renderer = 'static'
}
```

---

## 6. 迁移路径

```
当前状态: 纯 Chat (无桌宠)
    │
    ├─ Step 1: 添加 VRM 依赖 + 基础组件
    │   └─ pnpm add three @pixiv/three-vrm
    │
    ├─ Step 2: 实现 DivaVrmAvatar.vue
    │   └─ 基于官方示例从零编写
    │
    ├─ Step 3: 镜像可以同时支持 Live2D
    │   └─ renderer 切换
    │
    └─ Step 4: 发布
        └─ 默认 VRM + 可选 Live2D
```
