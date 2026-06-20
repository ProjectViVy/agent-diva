# 08 - VRM 配置与依赖管理

> VRM 模块的配置方案与完整依赖清单

---

## 1. 配置方案

### 1.1 config.json 扩展

```json
{
  "pet": {
    "enabled": true,
    "renderer": "vrm",
    "vrm": {
      "modelPath": "vrm/models/alice.vrm",
      "cameraDistance": 2.5,
      "cameraHeight": 1.3,
      "cameraFov": 30.0,
      "enableShadows": false,
      "enableOrbitControls": true,
      "autoRotate": false,
      "backgroundColor": "transparent",
      "ambientLightIntensity": 0.6,
      "directionalLightIntensity": 0.8,
      "pixelRatio": "auto",
      "animationOnGreeting": "greeting.vrma",
      "mouthSync": {
        "enabled": true,
        "mode": "sine"
      },
      "expressionMapping": {
        "happy": ["happy", "开心"],
        "sad": ["sad", "难过"],
        "surprised": ["surprised", "惊讶"]
      }
    }
  }
}
```

### 1.2 配置字段说明

| 字段 | 类型 | 默认值 | 说明 |
|------|------|--------|------|
| `renderer` | enum | `"vrm"` | `"vrm"` / `"live2d"` / `"static"` |
| `vrm.modelPath` | string | `"vrm/models/alice.vrm"` | 相对于 resource_dir 的路径 |
| `vrm.cameraDistance` | float | `2.5` | 相机距离 (1.0 - 5.0) |
| `vrm.cameraHeight` | float | `1.3` | 相机高度 |
| `vrm.enableShadows` | bool | `false` | 阴影（性能开销大） |
| `vrm.pixelRatio` | string | `"auto"` | `"auto"` / `1` / `2` |
| `vrm.mouthSync.enabled` | bool | `true` | 口型同步 |

---

## 2. 依赖清单

### 2.1 NPM 依赖

```json
{
  "dependencies": {
    "three": "^0.170.0",
    "@pixiv/three-vrm": "^3.2.0",
    "@pixiv/three-vrm-animation": "^3.2.0"
  },
  "devDependencies": {
    "@types/three": "^0.170.0"
  }
}
```

### 2.2 零 Vendor 依赖

与 Live2D 不同，VRM 方案**不需要**任何手动复制的 vendor 脚本。所有库均可通过 npm 直接安装。

```
❌ 不需要: public/live2d/cubism5/framework/*.js  (Live2D vendor)
❌ 不需要: public/live2d/cubism5/shaders/*       (Live2D shaders)
✅ 纯 npm: three + @pixiv/three-vrm
```

### 2.3 Rust 依赖（可选）

```toml
# 仅用于模型文件管理，无 VRM 专有依赖
walkdir = "2"  # 目录遍历（与 Live2D 共用）
```

### 2.4 完整依赖树

```
agent-diva-gui
├── [VRM 专属 npm 依赖]
│   ├── three ^0.170                    (MIT)  3D 渲染引擎
│   ├── @pixiv/three-vrm ^3.2          (MIT)  VRM 加载与表情
│   └── @pixiv/three-vrm-animation ^3.2 (MIT)  VRM 动画
├── [VRM 资源]
│   └── public/vrm/
│       ├── models/*.vrm                (独立许可) 模型文件
│       └── animations/*.vrma           (独立许可) 动画文件
└── [无 Vendor 脚本]
    └── (零手动管理)
```

---

## 3. 许可记录

| 依赖 | 许可 | 源 | 合规 |
|------|------|-----|------|
| three v0.170 | MIT | npm | ✅ |
| @pixiv/three-vrm v3.2 | MIT | npm | ✅ |
| @pixiv/three-vrm-animation v3.2 | MIT | npm | ✅ |
| 默认 VRM 模型 | 待确认 | CC0 资源 | ⚠️ 需确认 |

---

## 4. 版本管理

### 4.1 版本锁定

```json
// package.json 精确版本
{
  "three": "0.170.0",
  "@pixiv/three-vrm": "3.2.0",
  "@pixiv/three-vrm-animation": "3.2.0"
}
```

### 4.2 Three.js 版本注意

> `@pixiv/three-vrm` 对 Three.js 版本有严格要求，两者版本必须匹配。升级时需同步。

---

## 5. 构建配置

### 5.1 Vite 配置

```typescript
// vite.config.ts — Three.js 不需要特殊配置
export default defineConfig({
  build: {
    rollupOptions: {
      // Three.js + VRM 可正常 tree-shaking
    }
  }
})
```

### 5.2 TypeScript 配置

```json
{
  "compilerOptions": {
    "types": ["three"],
    "skipLibCheck": false
  }
}
```
