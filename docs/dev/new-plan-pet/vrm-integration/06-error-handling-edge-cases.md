# 06 - VRM 错误处理与边界条件

> VRM 模块的异常场景与容错策略

---

## 1. 错误分类与处理

### 1.1 致命错误（Crash Prevention）

| 错误 | 触发条件 | 处理 |
|------|----------|------|
| WebGL 不可用 | 无 GPU / 驱动问题 | 降级为静态图 + toast 提示 |
| Three.js 初始化失败 | JS 异常 | try/catch + 显示错误界面 |
| VRM 文件损坏 | 无效 .vrm | toast 错误 + 回退默认模型 |
| GPU 内存耗尽 | 加载超大模型 | 释放旧资源 + 降级到低分辨率 |

### 1.2 可恢复错误

| 错误 | 触发条件 | 处理 |
|------|----------|------|
| 模型加载超时 | 网络/文件 IO 慢 | 重试 2 次 → 显示进度 |
| 动画加载失败 | .vrma 文件缺失 | 跳过该动画 + console.warn |
| 表情不存在 | VRM 缺少某表情 | 跳过该表情 + 记录日志 |
| WebGL context lost | GPU 重置 | 监听 contextrestored → 重新初始化 |

---

## 2. 边界条件

### 2.1 模型文件

| 场景 | 处理 |
|------|------|
| .vrm 文件 > 100MB | 显示加载进度 + 警告 |
| VRM 0.x 格式模型 | 调用 `VRMUtils.rotateVRM0()` 自动适配 |
| VRM 1.0 格式模型 | 直接加载 |
| 非 VRM 文件（普通 glTF） | 检测 userData.vrm 为 null → 报错 |
| 模型无表情 BlendShape | 表情功能静默禁用 |
| 模型无口型 BlendShape | 口型同步静默禁用 |

### 2.2 渲染

| 场景 | 处理 |
|------|------|
| Canvas 尺寸为 0 | 跳过渲染帧 |
| Delta time 过大 (>100ms) | 限制为 100ms 防止跳帧 |
| 窗口最小化 | 暂停渲染循环 |
| DPR 极高 (>3) | Cap 为 2 |
| 多个 VRM 实例 | 仅活跃窗口渲染 |

### 2.3 动画

| 场景 | 处理 |
|------|------|
| 动画正在播放 + 新动画触发 | crossfade (0.3s) 过渡 |
| .vrma 文件不含动画数据 | 回退到 idle 状态 |
| 动画循环播放 | LoopRepeat → 循环；one-shot → LoopOnce |

---

## 3. 与 super-agent-party 的差异

由于不能复制 AGPL-3.0 代码，以下 super-agent-party 的高级功能将在 v2 阶段实现：

| 功能 | MVP (v1) | 后续 (v2) |
|------|----------|-----------|
| 全景 360 渲染 | ❌ | ✅ |
| WebXR 支持 | ❌ | ✅ |
| Gaussian Splatting 3D场景 | ❌ 纯色背景 | ✅ .spz |
| Audio Analyser 口型分析 | ❌ 简版正弦波 | ✅ FFT 分析 |
| 鼠标悬停自动隐藏 | ❌ | ✅ |
| PointerLockControls | ❌ | ✅ |

---

## 4. 用户提示文案

```typescript
// locales/zh.ts
pet: {
  vrm: {
    error: {
      webglUnavailable: '您的设备不支持 WebGL 2.0，无法渲染 3D 角色。',
      modelLoadFailed: 'VRM 模型加载失败，请检查文件是否完整。',
      modelNotFound: '未找到 VRM 模型，请在设置中导入一个 .vrm 文件。',
      invalidFormat: '文件不是有效的 VRM 格式，请使用 VRoid Studio 导出的 .vrm 文件。',
      tooLarge: '模型文件过大（{size}MB），加载可能较慢。',
    },
    hint: {
      howToGetModel: '在 VRoid Hub 下载或使用 VRoid Studio 自制 VRM 模型。',
      supportedFormats: '支持的格式：.vrm (VRM 0.x / 1.0)',
      loadingProgress: '正在加载角色... {progress}%',
    }
  }
}
```

---

## 5. 日志策略

```typescript
const LOG = '[VRM]'

console.log(`${LOG} Model loading: ${path}`)
console.log(`${LOG} Model loaded: ${vrm.meta.name}`)
console.log(`${LOG} Expressions: ${vrm.expressionManager.expressions.map(e => e.name)}`)
console.warn(`${LOG} Missing expression: ${exprName}`)
console.warn(`${LOG} WebGL context lost, attempting recovery`)
console.error(`${LOG} Failed to load VRM:`, error)
```
