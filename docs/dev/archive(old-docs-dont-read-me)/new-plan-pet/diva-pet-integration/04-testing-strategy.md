# 04 - 测试策略

> Diva Pet 模块的测试金字塔与验证方案

---

## 1. 测试金字塔

```
         ┌─────────┐
         │  E2E    │  手动验收测试（1-2 个场景）
         │  1-2    │
         └─────────┘
       ┌───────────────┐
       │  集成测试      │  Vue 组件测试 + Tauri IPC 集成
       │  5-8           │
       └───────────────┘
     ┌─────────────────────┐
     │  单元测试            │  纯逻辑函数测试（模型加载/表达式/TTS）
     │  15-20              │
     └─────────────────────┘
```

---

## 2. 单元测试

### 2.1 TTS Service 测试

```typescript
// features/diva-pet/__tests__/tts-service.test.ts

describe('TTSService', () => {
  describe('synthesize', () => {
    it('should return null when voice is disabled')
    it('should use browser TTS when provider is "browser"')
    it('should call CosyVoice2 API when provider is "siliconflow"')
    it('should fallback to standard TTS when voice cloning fails')
    it('should fallback to browser TTS when API fails')
    it('should handle empty text gracefully')
    it('should respect speed parameter')
    it('should timeout after specified duration')
  })

  describe('voice cloning', () => {
    it('should upload reference voice file')
    it('should reuse cached cloned voice URI')
    it('should rebuild clone when URI is invalidated')
    it('should fallback to inline reference when rebuild fails')
  })

  describe('playback', () => {
    it('should stop current playback when new audio starts')
    it('should resolve promise when playback ends')
    it('should reject promise when playback errors')
  })
})
```

### 2.2 Cubism5 Model 测试

```typescript
// features/diva-pet/__tests__/cubism5-model.test.ts

describe('DivaPetCubism5Model', () => {
  describe('model loading', () => {
    it('should load a valid .model3.json bundle')
    it('should reject invalid .model3.json')
    it('should reject missing .moc3 file')
    it('should decode and upload textures to GPU')
    it('should handle missing optional resources (physics/pose)')
    it('should generate mipmaps for textures')
  })

  describe('expressions', () => {
    it('should load all expressions from .model3.json')
    it('should set expression by name')
    it('should return false for unknown expression')
    it('should clear current expression')
    it('should list all available expression names')
  })

  describe('motions', () => {
    it('should load all motion groups')
    it('should play random motion from group')
    it('should apply priority-based queueing')
    it('should transition between motion groups')
    it('should return to idle group after reaction')
  })

  describe('viewport', () => {
    it('should apply scale and offset')
    it('should clamp minimum scale to 0.35')
    it('should update viewport matrix on resize')
  })

  describe('dispose', () => {
    it('should release all WebGL resources')
    it('should revoke object URLs')
    it('should delete all motions and expressions')
  })
})
```

---

## 3. 组件测试

### 3.1 DivaPetAvatar 组件测试

```typescript
// features/diva-pet/__tests__/DivaPetAvatar.test.ts

describe('DivaPetAvatar', () => {
  it('should render canvas element')
  it('should emit load-start on model loading')
  it('should emit load-success on successful load')
  it('should emit load-error on failed load')
  it('should handle pointer down/up for click detection')
  it('should distinguish head vs body click areas')
  it('should handle drag for window movement')
  it('should update expression when prop changes')
  it('should update motion group when prop changes')
  it('should cleanup resources on unmount')
  it('should show fallback when WebGL is unavailable')
})
```

### 3.2 useVoiceInput Composable 测试

```typescript
// features/diva-pet/__tests__/useVoiceInput.test.ts

describe('useVoiceInput', () => {
  it('should detect SpeechRecognition support')
  it('should set isSupported to false when API unavailable')
  it('should start listening when enabled')
  it('should emit recognized text on final result')
  it('should handle no-speech error gracefully')
  it('should handle not-allowed error by disabling')
  it('should auto-restart after recognition ends')
  it('should pause during TTS playback')
})
```

---

## 4. 集成测试

### 4.1 Tauri IPC 集成测试

```typescript
// features/diva-pet/__tests__/integration/ipc.test.ts

describe('Live2D IPC commands', () => {
  it('should list available live2d models via IPC')
  it('should load a live2d model bundle via IPC')
  it('should return error for missing model path')
  it('should read voice file as base64 via IPC')
})
```

### 4.2 Agent Reply → TTS 集成测试

```typescript
// features/diva-pet/__tests__/integration/agent-reply-to-tts.test.ts

describe('Agent Reply → TTS', () => {
  it('should trigger TTS on agent-response-complete event')
  it('should stop current TTS on new agent response')
  it('should not trigger TTS when voice is disabled')
  it('should use reply voiceStyle to adjust speech rate')
})
```

---

## 5. 验收测试（手动）

### 5.1 冒烟测试清单

| # | 测试场景 | 预期结果 | 状态 |
|---|----------|----------|------|
| 1 | 启动 GUI，进入 Diva Pet 页面 | Live2D 角色渲染正常 | |
| 2 | 点击角色头部 | 触发互动反馈 | |
| 3 | 拖拽角色 | 窗口跟随移动 | |
| 4 | 发送文字消息 | 角色回复气泡 + TTS 播放 | |
| 5 | 在 Settings 切换 Live2D 模型 | 新模型加载并渲染 | |
| 6 | 配置 TTS Provider + API Key | 语音合成正常 | |
| 7 | 关闭语音功能 | TTS 不再播放 | |
| 8 | 关闭桌宠功能 | 侧边栏入口隐藏 | |
| 9 | 从 Diva Pet 页切换到 Chat 页 | 对话正常继续 | |
| 10 | 最小化窗口后再恢复 | Live2D 渲染恢复 | |

### 5.2 边界条件测试

| # | 测试场景 | 预期结果 |
|---|----------|----------|
| 1 | 加载损坏的 Live2D 模型 | 显示错误提示，不崩溃 |
| 2 | WebGL 不可用环境 | 降级显示静态角色图 |
| 3 | TTS API Key 无效 | 降级到浏览器 TTS |
| 4 | 网络断开时 TTS | 降级到浏览器 TTS 或静默 |
| 5 | 麦克风权限被拒 | ASR 按钮禁用，显示引导 |
| 6 | 同时打开多个 Tauri 窗口 | 仅主窗口渲染 Live2D |

---

## 6. 测试工具与配置

### 6.1 测试框架

```json
// vitest.config.ts 扩展
{
  "test": {
    "environment": "jsdom",
    "globals": true,
    "setupFiles": ["./tests/setup.ts"],
    "include": ["src/**/*.test.ts", "src/**/*.spec.ts"],
    "coverage": {
      "provider": "v8",
      "reporter": ["text", "lcov"],
      "include": ["src/features/diva-pet/**"]
    }
  }
}
```

### 6.2 Mock 策略

```typescript
// tests/setup.ts

// Mock WebGL context
class MockWebGLRenderingContext { /* ... */ }
globalThis.WebGLRenderingContext = MockWebGLRenderingContext as any

// Mock Tauri invoke
vi.mock('@tauri-apps/api/core', () => ({
  invoke: vi.fn(),
}))

// Mock SpeechRecognition
class MockSpeechRecognition { /* ... */ }
globalThis.SpeechRecognition = MockSpeechRecognition as any

// Mock Audio API
globalThis.Audio = vi.fn().mockImplementation(() => ({
  play: vi.fn().mockResolvedValue(undefined),
  pause: vi.fn(),
}))
```

### 6.3 测试数据

```
tests/fixtures/
├── live2d/
│   ├── valid-model.model3.json
│   ├── valid-model.moc3
│   ├── invalid-model.model3.json
│   └── texture_00.png
├── voice/
│   └── sample-reference.mp3
└── tts/
    └── cosyvoice-response.mp3
```

---

## 7. CI 集成

```yaml
# .github/workflows/test.yml 新增步骤
- name: Run Diva Pet tests
  run: pnpm --filter agent-diva-gui test -- --reporter=verbose

- name: Check TypeScript
  run: pnpm --filter agent-diva-gui typecheck
```

---

## 8. 性能测试基准

| 指标 | 目标 | 测量方法 |
|------|------|----------|
| Live2D 首帧渲染时间 | < 3s | Performance API |
| 模型切换时间 | < 2s | Performance API |
| TTS 首字延迟 | < 1.5s | 事件时间戳 |
| 空闲 GPU 占用 | < 10% | Chrome DevTools |
| 内存占用 (Live2D) | < 150MB | Chrome DevTools |
| 60fps 稳定性 | > 95% 帧率 | requestAnimationFrame 计时 |

---

## 9. 回归测试风险点

| 风险点 | 监控方式 |
|--------|----------|
| 新 npm 依赖破坏现有构建 | `pnpm install` + `pnpm build` 在 CI |
| Live2D vendor 脚本与现有 JS 冲突 | 使用 `data-anipet-cubism5-core` 属性隔离 |
| TailwindCSS 新类名冲突 | 使用 `diva-pet-` 前缀 |
| Tauri IPC 新增 commands 注册冲突 | 确保命令名唯一（`pet_*` 前缀） |
