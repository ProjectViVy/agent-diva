# 08 - 配置与依赖管理

> Diva Pet 模块的配置方案与依赖清单

---

## 1. 配置方案

### 1.1 agent-diva config.json 扩展

```json
{
  "schema_version": 2,
  "providers": { /* 现有，不变 */ },
  "agents": { /* 现有，不变 */ },
  "channels": { /* 现有，不变 */ },

  "pet": {
    "enabled": true,
    "live2d": {
      "modelPath": "live2d_resource/default/mao_pro.model3.json",
      "scale": 0.72,
      "offsetX": 0.0,
      "offsetY": 0.0,
      "renderQuality": "medium"
    },
    "voice": {
      "enabled": true,
      "asr": {
        "enabled": true,
        "provider": "browser",
        "language": "zh-CN",
        "autoRestart": true,
        "pauseDuringTts": true
      },
      "tts": {
        "provider": "siliconflow",
        "apiKey": "",
        "baseUrl": "https://api.siliconflow.cn/v1",
        "model": "FunAudioLLM/CosyVoice2-0.5B",
        "referenceVoice": "",
        "referenceText": "",
        "speed": 1.0,
        "volume": 1.0,
        "fallbackToBrowser": true
      }
    },
    "character": {
      "displayName": "Diva",
      "documentPath": "",
      "personaSummary": "A helpful AI desktop companion.",
      "speechStyle": "Warm, concise, supportive."
    },
    "interaction": {
      "clickHeadAction": "chat",
      "clickBodyAction": "drag"
    }
  }
}
```

### 1.2 配置字段说明

| 字段 | 类型 | 默认值 | 说明 |
|------|------|--------|------|
| `pet.enabled` | bool | `false` | 是否启用桌宠功能 |
| `pet.live2d.modelPath` | string | `""` | Live2D 模型路径 |
| `pet.live2d.scale` | float | `0.72` | 角色缩放（0.35-1.5） |
| `pet.live2d.offsetX` | float | `0.0` | 水平偏移 |
| `pet.live2d.offsetY` | float | `0.0` | 垂直偏移 |
| `pet.live2d.renderQuality` | enum | `"medium"` | `"low"` / `"medium"` / `"high"` |
| `pet.voice.enabled` | bool | `true` | 是否启用语音功能 |
| `pet.voice.asr.enabled` | bool | `true` | 启用语音输入 |
| `pet.voice.asr.provider` | string | `"browser"` | ASR Provider |
| `pet.voice.asr.language` | string | `"zh-CN"` | 识别语言 |
| `pet.voice.tts.provider` | string | `"browser"` | TTS Provider |
| `pet.voice.tts.apiKey` | string | `""` | API Key（敏感） |
| `pet.voice.tts.model` | string | `""` | 模型 ID |
| `pet.voice.tts.referenceVoice` | string | `""` | 参考音频路径（声音克隆） |
| `pet.voice.tts.speed` | float | `1.0` | 语速（0.5-2.0） |
| `pet.character.displayName` | string | `"Diva"` | 角色名称 |

### 1.3 配置验证

```typescript
// features/diva-pet/services/pet-config.ts

function validatePetConfig(config: unknown): PetConfig {
  const schema = {
    enabled: { type: 'boolean', default: false },
    live2d: {
      modelPath: { type: 'string', default: '' },
      scale: { type: 'number', min: 0.35, max: 1.5, default: 0.72 },
      renderQuality: { type: 'enum', values: ['low', 'medium', 'high'], default: 'medium' },
    },
    voice: {
      enabled: { type: 'boolean', default: true },
      tts: {
        provider: { type: 'enum', values: ['browser', 'openai', 'siliconflow'], default: 'browser' },
        speed: { type: 'number', min: 0.5, max: 2.0, default: 1.0 },
      },
    },
  }
  // ... 验证逻辑
}
```

### 1.4 运行时配置更新

```typescript
// 通过 Tauri IPC 持久化到 config.json
async function savePetConfig(config: PetConfig): Promise<void> {
  await invoke('save_pet_config', { config: JSON.stringify(config) })
}
```

---

## 2. 依赖清单

### 2.1 NPM 依赖（agent-diva-gui/package.json）

```json
{
  "dependencies": {
    // 现有（不变）
    "vue": "^3.5.13",
    "@tauri-apps/api": "^2",
    "tailwindcss": "^3.4.17",
    "lucide-vue-next": "^0.575.0",
    "vue-i18n": "^9.14.4",
    "markdown-it": "^14.1.1",

    // 新增 — Live2D 核心
    "live2dcubismcore": "1.0.2",
    "pixi.js": "6.5.10",
    "pixi-live2d-display": "0.4.0"
  }
}
```

### 2.2 Rust 依赖（src-tauri/Cargo.toml）

```toml
[dependencies]
# 现有（不变）
tauri = { version = "2", features = ["tray-icon"] }
tauri-plugin-opener = "2"
tauri-plugin-store = "2"
agent-diva-core = { path = "../../agent-diva-core" }
# ... 其他现有依赖

# 新增 — 文件操作
walkdir = "2"            # 目录遍历（读取模型文件目录）
base64 = "0.22"           # base64 编码（已有 crate 隐式依赖，显式声明）
```

### 2.3 Vendor 脚本（手动管理）

> **源目录**: [`AniPet/apps/desktop/src/vendor/cubism5-framework/`](../../../../../AniPet/apps/desktop/src/vendor/cubism5-framework/)  
> **Cubism Core**: [`AniPet/vendor/official-live2dcubismcore.min.js`](../../../../../AniPet/vendor/official-live2dcubismcore.min.js)

```
public/live2d/cubism5/
├── live2dcubismcore.js              # 从 AniPet vendor/ 复制（Wasm 运行时）
├── framework/
│   ├── live2dcubismframework.js     # 核心框架
│   ├── cubismdefaultparameterid.js  # 参数常量
│   ├── cubismmodelsettingjson.js    # .model3.json 解析
│   ├── cubismusermodel.js           # 用户模型基类
│   ├── cubismmotion.js              # 动作系统
│   ├── cubismmotionqueuemanager.js  # 动作队列
│   ├── rendering/
│   │   ├── cubismrenderer_webgl.js  # WebGL 渲染核心 ⚠️ 必需
│   │   ├── cubismshader_webgl.js    # 着色器管理 ⚠️ 必需
│   │   └── cubismoffscreenmanager.js # 离屏渲染
│   ├── model/
│   │   └── cubismmoc.js             # .moc3 解析 ⚠️ 必需
│   ├── effect/
│   │   ├── cubismbreath.js          # 呼吸效果
│   │   └── cubismeyeblink.js        # 眨眼效果
│   ├── physics/
│   │   ├── cubismphysics.js         # 物理引擎
│   │   └── cubismphysicsjson.js     # 物理配置解析
│   ├── math/
│   │   └── cubismmatrix44.js        # 矩阵运算
│   ├── id/
│   │   └── cubismid.js              # Cubism ID 系统
│   └── utils/
│       ├── cubismjson.js            # JSON 工具
│       └── cubismstring.js          # 字符串工具
└── shaders/
    ├── vertex_shader.glsl           # 顶点着色器
    └── fragment_shader.glsl         # 片段着色器
```

> **注意**: Cubism 5 着色器由 `cubismshader_webgl.js` 内联管理（见 AniPet 对应文件），外部 `.glsl` 文件仅作备选。实际路径由 `cubism5-model.ts` L29-L32 的 `LIVE2D_SHADER_BASE_PATH` 常量定义。

### 2.4 完整依赖树

```
agent-diva-gui
├── [生产依赖]
│   ├── vue 3.5                        (核心框架)
│   ├── @tauri-apps/api 2              (桌面 IPC)
│   ├── live2dcubismcore 1.0.2         (Cubism 5 Wasm 运行时)
│   ├── pixi.js 6.5.10                 (WebGL 渲染框架)
│   └── pixi-live2d-display 0.4.0      (PixiJS-Live2D 桥接)
├── [Vendor 脚本]
│   └── cubism5-framework/*.js         (Live2D 官方 SDK, ~200KB)
├── [Rust 后端]
│   ├── tauri 2                        (桌面壳)
│   ├── walkdir 2                      (文件遍历)
│   └── base64 0.22                    (编码)
└── [资源]
    ├── live2d_resource/               (Live2D 模型文件)
    ├── live2d/cubism5/shaders/        (WebGL 着色器)
    └── voice_resource/                (参考音频, 可选)
```

---

## 3. 许可合规

### 3.1 关键许可

| 依赖 | 许可 | 注意事项 |
|------|------|----------|
| Live2D Cubism SDK 5 | [Live2D 专有许可](https://www.live2d.com/eula/) | 需遵守 [Cubism SDK 发布许可](https://www.live2d.com/en/download/cubism-sdk/download-release/) |
| live2dcubismcore | Live2D 专有许可 | 随 SDK 分发，不可独立商用 |
| pixi.js 6.5.10 | MIT | ✅ 无限制 |
| pixi-live2d-display 0.4.0 | MIT | ✅ 无限制 |
| walkdir (Rust) | MIT / Unlicense | ✅ 无限制 |

### 3.2 Live2D SDK 特别说明

> ⚠️ **发布前必须确认**：Live2D Cubism SDK 的使用需遵守其 [EULA](https://www.live2d.com/eula/)。如 agent-diva 为开源项目（MIT），需确认是否与 Live2D SDK 许可兼容。建议：
> 1. 将 Live2D 功能作为可选插件
> 2. 用户需自行下载 Live2D SDK 运行时
> 3. 不在仓库中分发 `.moc3` 等专有格式模型
> 4. 发布时在 README 中明确说明 Live2D 许可承担方

---

## 4. 版本管理策略

### 4.1 版本锁定

```json
// package.json — 精确版本锁定
{
  "dependencies": {
    "live2dcubismcore": "1.0.2",       // 精确版本，Cubism 5 兼容性敏感
    "pixi.js": "6.5.10",               // 精确版本，v7 不兼容
    "pixi-live2d-display": "0.4.0"     // 精确版本
  }
}

// pnpm-lock.yaml 确保 CI 一致性
```

### 4.2 升级路径

| 依赖 | 当前版本 | 下一个大版本 | 迁移难度 |
|------|----------|-------------|----------|
| pixi.js | 6.5.10 | 8.x | 高（API 全面变更） |
| live2dcubismcore | 1.0.2 | 后续版本 | 低（API 稳定） |
| Tauri | 2.x | 3.x (未来) | 中 |

---

## 5. 环境变量

```bash
# .env 或系统环境变量（用于 CI/开发）
VITE_DIVA_PET_ENABLED=true            # 是否启用桌宠功能
VITE_LIVE2D_MODEL_PATH=./live2d_resource/default/
TAURI_LIVE2D_RESOURCE_DIR=./live2d_resource/

# TTS 密钥（建议通过 config.json 管理，环境变量作为 fallback）
AGENT_DIVA_PET_TTS_API_KEY=sk-xxxx
```

---

## 6. Tauri 资源配置

```json
// src-tauri/tauri.conf.json
{
  "bundle": {
    "resources": {
      "public/live2d/cubism5/shaders/*": "live2d/cubism5/shaders/",
      "public/live2d/cubism5/framework/*.js": "live2d/cubism5/framework/"
    }
  }
}
```

---

## 7. 构建配置

### 7.1 Vite 配置

```typescript
// vite.config.ts
export default defineConfig({
  // 将 live2dcubismcore 作为外部脚本加载（不打包进 bundle）
  build: {
    rollupOptions: {
      external: [],
    }
  },
  // 静态资源路径
  publicDir: 'public',
  // 允许导入 vendor 脚本
  optimizeDeps: {
    exclude: ['live2dcubismcore']
  }
})
```

### 7.2 TypeScript 配置

```json
// tsconfig.json
{
  "compilerOptions": {
    // 允许导入 PixiJS v6 的类型
    "types": ["pixi.js"],
    // Live2D vendor 脚本无类型声明
    "skipLibCheck": true
  },
  "include": [
    "src/**/*.ts",
    "src/**/*.vue",
    "src/features/diva-pet/**/*"
  ]
}
```
