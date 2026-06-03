# 08 — 配置与依赖管理

## 1. 新增 npm 依赖

### 1.1 @sparkjsdev/spark

| 属性 | 值 |
|------|-----|
| 包名 | `@sparkjsdev/spark` |
| 用途 | Gaussian Splatting 渲染 (SplatMesh) |
| 安装 | `pnpm add @sparkjsdev/spark` |
| 使用位置 | `avatar-runtime-vrm/src/runtime/gauss-scene-controller.ts` (已 import) |
| 体积 | ~200KB minified |

**备选方案**: `src/vendor/spark.module.js` + vite.config.ts alias.

### 1.2 无需新增的其他依赖

`three` 0.184.0, `@pixiv/three-vrm` 3.5.2 已满足需求。

---

## 2. PetConfig 扩展

```typescript
// 新增字段
selectedGaussSceneId: 'transparent',  // 当前场景 ID
gaussSceneList: [                      // 可用场景列表
  { id: 'transparent', name: '透明背景', path: '', isDefault: true },
  { id: 'home',        name: '室内场景', path: 'vrm/scene/home.spz',  isDefault: true },
  { id: 'sea',         name: '海边场景', path: 'vrm/scene/sea.spz',   isDefault: true },
  { id: 'space',       name: '太空场景', path: 'vrm/scene/space.spz', isDefault: true },
]
```

**持久化**: localStorage key `diva-pet-config`，JSON 格式。`pet-config.ts` 加载时合并默认值。

---

## 3. 场景变换参数 (GaussSceneController 内置)

| 场景 ID | height | scale | 来源 |
|---------|--------|-------|------|
| `space` | 1.55 | 2 | gauss-scene-controller.ts:69-71 |
| `home` | 1.6 | 2 | gauss-scene-controller.ts:72-74 |
| `sea` | 2.4 | 4 | gauss-scene-controller.ts:75-78 |
| 自定义 | 1.6 (默认) | 2 (默认) | gauss-scene-controller.ts:66-68 |

如需支持自定义场景的变换参数，可在 `GaussSceneEntry` 中增加可选 `height` 和 `scale` 字段（当前阶段不实现）。

---

## 4. 资源目录

```
agent-diva-gui/public/vrm/
├── models/              ← 现有 VRM 模型
├── animations/          ← 现有 VRM 动画
└── scene/               ← ★ 新增
    ├── home.spz
    ├── sea.spz
    └── space.spz
```

Vite dev: `http://localhost:1420/vrm/scene/home.spz`  
Vite build: 自动复制到 `dist/vrm/scene/`  
Tauri production: 包含在 bundle 中

---

## 5. Tauri 配置

**不需要修改**。主窗口的 `tauri.conf.json` 配置已支持 WebGL：

```json
{ "label": "main", "url": "/index.html" }
```

`.spz` 文件通过 Vite `public/` → Tauri 自定义协议 serve，无需额外权限。
