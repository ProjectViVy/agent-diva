# 04 - VRM 测试策略

> VRM 模块的测试金字塔与验证方案

---

## 1. 测试金字塔

```
         ┌─────────┐
         │  手动验收 │  VRM 渲染 + 表情 + 口型
         │  2-3     │
         └─────────┘
       ┌───────────────┐
       │  组件测试      │  DivaVrmAvatar 挂载/卸载/Props
       │  4-6           │
       └───────────────┘
     ┌─────────────────────┐
     │  单元测试            │  composables 纯逻辑
     │  8-12               │
     └─────────────────────┘
```

---

## 2. 单元测试

### 2.1 useVrmExpression 测试

```typescript
describe('useVrmExpression', () => {
  it('should default to neutral mood')
  it('should detect happy mood from keywords like "哈哈"')
  it('should detect sad mood from keywords like "难过"')
  it('should detect surprised mood from "wow"')
  it('should set expression value to 1.0 on matched mood')
  it('should reset previous expression when mood changes')
  it('should handle empty content gracefully')
  it('should handle null vrm gracefully')
})
```

### 2.2 useVrmMouthSync 测试

```typescript
describe('useVrmMouthSync', () => {
  it('should set mouth shapes to 0 when not speaking')
  it('should generate non-zero values when speaking')
  it('should cycle through mouth shapes over time')
  it('should handle null vrm gracefully')
  it('should reset all mouth shapes on stop speaking')
})
```

### 2.3 vrm-loader 测试

```typescript
describe('vrm-loader', () => {
  it('should call Tauri IPC to list VRM models')
  it('should return empty array when no models found')
  it('should filter only .vrm files')
  it('should parse model metadata from VRM header')
})
```

---

## 3. 组件测试

### 3.1 DivaVrmAvatar 测试

```typescript
describe('DivaVrmAvatar', () => {
  it('should create Three.js renderer with transparent background')
  it('should create PerspectiveCamera with correct FOV')
  it('should add ambient and directional lights to scene')
  it('should load VRM model from path prop')
  it('should emit load-start event before loading')
  it('should emit load-success after model loaded')
  it('should emit load-error on invalid VRM file')
  it('should dispose renderer on unmount')
  it('should cancel animation frame on unmount')
  it('should update vrm each frame in animation loop')
  it('should handle window resize')
  it('should cap DPR at 2')
})
```

---

## 4. 集成测试

### 4.1 消息 → 表情 集成

```typescript
describe('Message → VRM Expression', () => {
  it('should update expression when agent reply contains happy keywords')
  it('should reset to neutral on non-emotional messages')
  it('should update expression when messages prop changes')
  it('should not throw when VRM model not yet loaded')
})
```

### 4.2 TTS → 口型 集成

```typescript
describe('TTS → VRM Mouth Sync', () => {
  it('should start mouth animation when isSpeaking becomes true')
  it('should stop mouth animation when isSpeaking becomes false')
  it('should reset mouth shapes to 0 on stop')
})
```

---

## 5. 验收测试（手动）

| # | 测试场景 | 预期结果 |
|---|----------|----------|
| 1 | 进入 Diva Pet (VRM) 页面 | 3D 角色正常渲染，约 3-5s 加载完成 |
| 2 | 鼠标拖拽旋转视角 | OrbitControls 正常响应 |
| 3 | 滚轮缩放 | 相机距离变化 |
| 4 | 发送 "哈哈太开心了" | 角色表情变为 happy |
| 5 | 发送 "好难过..." | 角色表情变为 sad |
| 6 | TTS 播放时 | 嘴巴同步开合 |
| 7 | 切换 VRM 模型 | 新模型加载，旧模型释放 |
| 8 | 切换到 Live2D 渲染器 | Live2D 正确渲染 |
| 9 | 低端 GPU (集成显卡) | 渲染不卡顿 (>20fps) |
| 10 | 窗口 resize | 渲染自适应 |

---

## 6. Mock 策略

```typescript
// tests/setup.ts

// Mock Three.js WebGL context
class MockWebGLRenderingContext {
  createTexture() { return {} }
  // ... minimal mock
}

// Mock Tauri invoke
vi.mock('@tauri-apps/api/core', () => ({ invoke: vi.fn() }))

// Mock GLTFLoader
vi.mock('three/addons/loaders/GLTFLoader.js', () => ({
  GLTFLoader: vi.fn().mockImplementation(() => ({
    register: vi.fn(),
    loadAsync: vi.fn().mockResolvedValue({
      userData: {
        vrm: {
          scene: {},
          expressionManager: { setValue: vi.fn(), resetValues: vi.fn() },
          update: vi.fn(),
        }
      }
    })
  }))
}))
```
