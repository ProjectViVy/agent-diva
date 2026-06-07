# 07 - 性能考量

> Diva Pet 模块的性能基准、优化策略与监控方案

---

## 1. 性能目标

| 指标 | 目标值 | 测量基准 |
|------|--------|----------|
| Live2D 首帧时间 (TTFR) | < 3.0s | 从 init 到第一帧渲染 |
| 空闲帧率 | ≥ 30fps | 仅呼吸/眨眼动画 |
| 活跃帧率 | ≥ 60fps | 动作/表情/拖拽 |
| 帧渲染时间 | < 16ms/frame | WebGL draw + update |
| TTS 首字延迟 | < 1.5s | 从触发到音频开始播放 |
| ASR 识别延迟 | < 500ms | 从说话结束到文本输出 |
| 内存占用 (Live2D) | < 150MB | 单个模型 + 纹理 |
| GPU 占用 (空闲) | < 10% | 集成显卡 |
| GPU 占用 (活跃) | < 20% | 集成显卡 |
| 包体积增加 | < 3MB (gzip < 1MB) | Tauri bundle |

---

## 2. Live2D 渲染性能分析

### 2.1 帧时间分解

```
单帧渲染时间 (~8ms @ 60fps):
├── updateModel: ~2ms
│   ├── motionManager.update()      ~0.5ms
│   ├── expressionManager.update()  ~0.2ms
│   ├── physics.evaluate()          ~0.5ms (如果有)
│   ├── breath/eyeBlink.update()    ~0.1ms
│   └── model.update() (矩阵运算)   ~0.7ms
├── WebGL 渲染: ~4ms
│   ├── gl.clear()                  ~0.2ms
│   ├── renderer.setRenderState()   ~0.5ms
│   ├── renderer.drawModel()        ~3.0ms (最耗时)
│   └── offscreenManager            ~0.3ms
└── Canvas 合成: ~2ms
```

### 2.2 自适应帧率策略

```typescript
// 从 AniPet 已验证的策略直接复用

const FRAME_INTERVAL_ACTIVE_MS = 16.67;   // ~60fps
const FRAME_INTERVAL_IDLE_MS = 33.33;     // ~30fps
const FRAME_INTERVAL_TOLERANCE = 0.85;
const FORCE_HIGH_FPS_DURATION_MS = 2000;  // 切换动作后保持 2s 高帧率

function shouldRenderThisFrame(now: number, lastFrameTime: number): boolean {
  const elapsed = now - lastFrameTime
  const isActive =
    model.isAnimating() ||           // 有动画在播放
    dragSession !== null ||          // 正在拖拽
    now < forceHighFpsUntil          // 刚切换动作
  const targetInterval = isActive
    ? FRAME_INTERVAL_ACTIVE_MS
    : FRAME_INTERVAL_IDLE_MS

  return elapsed >= targetInterval * FRAME_INTERVAL_TOLERANCE
}
```

### 2.3 页面不可见时的优化

```typescript
// 使用 Page Visibility API
document.addEventListener('visibilitychange', () => {
  if (document.hidden) {
    stopRenderLoop()
  } else {
    startRenderLoop()
  }
})
```

### 2.4 纹理优化

```typescript
// cubism5-model.ts 中已有的优化（直接复用）

// 1. 并行解码贴图
const decodePromises: Promise<TextureSlot>[] = []
for (const texture of textures) {
  decodePromises.push(decodeTextureImage(texture.bytes, texture.name))
}
const decodedSlots = await Promise.all(decodePromises)

// 2. 使用 createImageBitmap 离线程解码
if (typeof createImageBitmap === 'function') {
  const bitmap = await createImageBitmap(blob, {
    premultiplyAlpha: 'premultiply',
    colorSpaceConversion: 'none'
  })
}

// 3. WebGL 各向异性过滤
const anisoExt = gl.getExtension('EXT_texture_filter_anisotropic')
gl.texParameteri(gl.TEXTURE_2D, anisoExt.TEXTURE_MAX_ANISOTROPY_EXT, 16)

// 4. 超尺寸纹理降采样
if (width > maxTextureSize || height > maxTextureSize) {
  // 缩放到 maxTextureSize 以内
}

// 5. Mipmap 生成
gl.generateMipmap(gl.TEXTURE_2D)

// 6. 上传后立即释放 ImageBitmap
if (decoded.source instanceof ImageBitmap) {
  decoded.source.close()
}
```

### 2.5 Canvas 分辨率管理

```typescript
// 根据 DPR 和性能档位动态调整

const QUALITY_SCALE_FACTOR = 1.5  // AniPet 默认

// 可扩展为性能档位：
// high:    devicePixelRatio * 2.0
// medium:  devicePixelRatio * 1.5 (默认)
// low:     devicePixelRatio * 1.0
const totalScale = devicePixelRatio * QUALITY_SCALE_FACTOR
canvas.width = Math.round(CANVAS_WIDTH * totalScale)
canvas.height = Math.round(CANVAS_HEIGHT * totalScale)
```

---

## 3. TTS 性能优化

### 3.1 声音克隆缓存

```typescript
// tts-service.ts 中的缓存策略（直接复用）

// 缓存已上传的克隆声音 URI，避免重复上传
private cachedCloneVoice: { key: string; uri: string } | null = null

// 请求去重：相同配置的 concurrently 调用共享一个 Promise
private pendingCloneVoicePromise: Promise<string | null> | null = null
```

### 3.2 预加载策略

```typescript
// 在应用启动时预加载声音克隆（如果启用）
onMounted(async () => {
  const voiceConfig = loadVoiceConfig()
  if (voiceConfig.enabled && voiceConfig.referenceVoice) {
    await ttsService.prepareVoiceClone(voiceConfig)
  }
})
```

### 3.3 音频播放优化

```typescript
// 使用 Audio 对象池避免频繁创建
class AudioPool {
  private pool: HTMLAudioElement[] = []
  private maxSize = 3

  acquire(): HTMLAudioElement {
    return this.pool.pop() || new Audio()
  }

  release(audio: HTMLAudioElement) {
    audio.pause()
    audio.src = ''
    if (this.pool.length < this.maxSize) {
      this.pool.push(audio)
    }
  }
}
```

---

## 4. ASR 性能优化

### 4.1 自动静音检测

```typescript
// 利用浏览器 VAD（Voice Activity Detection）
// SpeechRecognition 内置 VAD，返回最终结果时自动停止

recognition.continuous = false  // 单句模式，自动检测静音
```

### 4.2 防抖处理

```typescript
// 避免短时间内多次发送相同语音结果
let lastRecognizedText = ''
let lastRecognizedTime = 0
const DEBOUNCE_MS = 1000

function onRecognizedText(text: string) {
  const now = Date.now()
  if (text === lastRecognizedText && now - lastRecognizedTime < DEBOUNCE_MS) {
    return  // 重复识别
  }
  lastRecognizedText = text
  lastRecognizedTime = now
  // 发送消息
}
```

---

## 5. 内存管理

### 5.1 Live2D 模型生命周期

```typescript
// 严格遵循创建-销毁对

onMounted(() => {
  model = await createModel()
})

onUnmounted(() => {
  model?.dispose()            // 释放 WebGL 资源
  gl?.getExtension('WEBGL_lose_context')?.loseContext()
  URL.revokeObjectURL(audioUrl)
})

// dispose 实现（cubism5-model.ts 中已有）：
dispose() {
  // 1. 释放贴图 URL
  for (const url of this._textureUrls) URL.revokeObjectURL(url)
  // 2. 删除动作实例
  for (const motion of this._motions.values()) ACubismMotion.delete(motion)
  // 3. 删除表情实例
  for (const expression of this._expressions.values()) ACubismMotion.delete(expression)
  // 4. 释放模型设置
  this._modelSetting?.release()
  // 5. 调用父类释放
  super.release()
}
```

### 5.2 内存监控

```typescript
// 开发环境下监控内存
if (import.meta.env.DEV && performance.memory) {
  setInterval(() => {
    const mem = (performance as any).memory
    console.debug('[DivaPet] Memory:', {
      usedJSHeapSize: (mem.usedJSHeapSize / 1048576).toFixed(1) + 'MB',
      totalJSHeapSize: (mem.totalJSHeapSize / 1048576).toFixed(1) + 'MB',
    })
  }, 30000)
}
```

---

## 6. 加载性能

### 6.1 启动时序

```
应用启动
├── T+0ms     Tauri 窗口创建
├── T+200ms   Vue 挂载，渲染基础 UI
├── T+500ms   开始异步加载 Live2D
│   ├── 加载 Cubism 5 Wasm    (~1s)
│   ├── 加载模型文件 (IPC)     (~0.5s)
│   └── 初始化 WebGL + 纹理   (~0.5s)
├── T+2500ms  Live2D 首帧渲染 ← 目标 < 3s
└── T+3000ms  用户可交互
```

### 6.2 延迟加载策略

```typescript
// Live2D 非首屏必要内容，使用动态 import
const DivaPetAvatar = defineAsyncComponent(
  () => import('./components/DivaPetAvatar.vue')
)

// 仅在用户进入 Diva Pet 页面时才加载
```

---

## 7. 包体积优化

### 7.1 按需加载

```typescript
// vendor 脚本使用动态 script 标签加载（不打包进 bundle）
// cubism5-core.ts 中已有：
const script = document.createElement('script')
script.src = '/live2d/cubism5/live2dcubismcore.js'
document.head.appendChild(script)
```

### 7.2 Tree-shaking

```typescript
// PixiJS 仅导入需要的模块
import { Application, Container } from 'pixi.js'  // 避免 import * as PIXI
```

### 7.3 资源压缩

```
public/live2d/cubism5/shaders/    → 可内联为 JS 字符串（减少文件数）
live2d_resource/                   → 打包时排除（运行时加载或首次下载）
```

---

## 8. 性能回归测试

### 8.1 Lighthouse / DevTools 指标

```bash
# 使用 Tauri 的 WebView DevTools 测量
- Performance 面板：录制 10 秒 Live2D 渲染
- Memory 面板：检查 JS Heap 和 GPU Memory
- Rendering 面板：FPS Meter
```

### 8.2 自动化基准

```typescript
// features/diva-pet/__tests__/benchmarks/render-bench.ts

describe('Render Performance', () => {
  it('should maintain 60fps during idle animation', async () => {
    const fps = await measureFPS(5000, 'idle')
    expect(fps).toBeGreaterThanOrEqual(55)
  })

  it('should maintain 30fps during idle (energy saving)', async () => {
    const fps = await measureFPS(10000, 'idle')
    expect(fps).toBeLessThanOrEqual(35)
  })

  it('should render first frame within 3s', async () => {
    const ttfr = await measureTTFR()
    expect(ttfr).toBeLessThan(3000)
  })

  it('should not leak memory over 100 frames', async () => {
    const memBefore = getJSHeapSize()
    await runFrames(100)
    const memAfter = getJSHeapSize()
    expect(memAfter - memBefore).toBeLessThan(10 * 1024 * 1024) // < 10MB
  })
})
```
