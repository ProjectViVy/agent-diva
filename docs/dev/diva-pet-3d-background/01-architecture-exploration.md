# 01 — 架构设计探索

> **参考**: `super-agent-party/static/js/vrm.js`, `vrm/scene/*.spz`  
> **目标**: `agent-diva/agent-diva-gui/src/features/diva-pet/components/DivaPetView.vue`  
> **功能**: 为主窗口内嵌 diva 桌宠添加 3D 场景背景

---

## 1. Gaussian Splatting 渲染原理

### 1.1 数学基础

3D Gaussian Splatting (3DGS) 用一组各向异性 3D 高斯函数表示场景。每个 splat 由以下参数定义：

- **均值 μ ∈ ℝ³**: splat 中心位置
- **协方差 Σ ∈ ℝ³ˣ³**: 编码 splat 的形状和朝向
  - 分解: Σ = R S S^T R^T，其中 R 是旋转矩阵，S 是对角缩放矩阵
- **不透明度 α ∈ [0,1]**: splat 的整体透明度
- **球谐系数 (SH)**: 编码视角相关的颜色 c₀, c₁, ... (通常 0-3 阶，即 1-16 个系数)

**投影** (3D → 2D):
将 3D 高斯投影到图像平面，协方差变换为:
Σ' = J W Σ W^T J^T

其中 W 是视图变换矩阵，J 是投影变换的 Jacobian（在点 μ 处线性化）。

**体积渲染** (alpha compositing):
像素颜色 C 通过从前到后累积所有 splat 的颜色贡献:
C = Σᵢ cᵢ · αᵢ · Πⱼ₌₁ⁱ⁻¹ (1 - αⱼ)

其中:
- cᵢ = 球谐函数在观察方向上的颜色值
- αᵢ = splat 的 2D 投影不透明度（含高斯衰减因子）
- 累积透明度: Tᵢ = Πⱼ₌₁ⁱ⁻¹ (1 - αⱼ)

### 1.2 SplatMesh 在 Three.js 中的实现

`@sparkjsdev/spark` 的 `SplatMesh` 继承自 `THREE.Object3D`，可直接插入场景图：

```
THREE.Scene
└─ THREE.Object3D (Group: gaussScene_xxx)
   └─ SplatMesh ← 自定义 BufferGeometry + ShaderMaterial
      ├─ positions: THREE.BufferAttribute (Float32, N×3)
      ├─ scales: THREE.BufferAttribute (Float32, N×3)
      ├─ rotations: THREE.BufferAttribute (Float32, N×4)
      ├─ colors: THREE.BufferAttribute (Float32, N×M)  ← SH 系数
      └─ opacities: THREE.BufferAttribute (Float32, N)
```

**自定义 Shader 管线**:
- **Vertex shader**: 将每个 splat 的 3D 中心投影到屏幕空间，计算 2D 协方差椭圆
- **Fragment shader**: 使用 2D 高斯函数计算当前像素的 splat 贡献（α × exp(-½Δx^T Σ'⁻¹ Δx)），alpha blend
- 使用 `THREE.CustomBlending` 和自定义 blend 方程控制 alpha 累积

**排序策略**:
SplatMesh 内置 GPU 排序（或 CPU 预排序），按 depth 从前到后排列 splat，满足体积渲染方程的前向 alpha 累积要求。

### 1.3 .spz 文件格式

.spz 是 Gaussian Splat 数据的压缩二进制格式：

- 文件头: magic bytes + version + splatCount
- 数据段: 压缩的 (position, scale, rotation, SH, opacity) 元组数组
- 压缩算法: 推测为量化 + LZ 系列压缩
- 加载: SplatMesh 构造函数中异步 fetch → decode → GPU buffer upload

---

## 2. 参考实现: super-agent-party 场景系统

### 2.1 场景加载流程

文件: `super-agent-party/static/js/vrm.js:342-419` (`loadGaussScene`)

```
loadGaussScene()
  1. fetchVRMConfig() → get selectedGaussSceneId
  2. 查找场景 URL:
     defaultModels ∪ userModels 按 id 匹配
     未找到 → 回退 'transparent'
  3. 卸载旧场景:
     scene.remove(group)
     group.traverse(o => o.dispose?.())  ← 递归释放 GPU 资源
  4. 构建新场景 Group:
     if transparent:
       PlaneGeometry(20,20) + ShadowMaterial(opacity:0.4)
       rotation.x = -π/2, receiveShadow=true
     else:
       new SplatMesh({ url })
       position.set(0, height, 2)
       scale.set(scale, scale, scale)
       receiveShadow = true
  5. scene.add(group)
```

### 2.2 场景参数表

| ID | height | scale | 说明 |
|----|--------|-------|------|
| `space` | 1.55 | 2 | 深空，splat 少，性能最佳 |
| `home` | 1.6 | 2 | 室内，中等 splat 密度 |
| `sea` | 2.4 | 4 | 海滩，大场景需大缩放因子 |
| custom | 1.6 (默认) | 2 (默认) | 用户自定义 |

### 2.3 配置获取

场景配置由 Python 后端 `/vrm_config` API 返回：

```json
{
  "selectedGaussSceneId": "home",
  "gaussDefaultScenes": [
    { "id": "home", "name": "室内", "path": "/vrm/scene/home.spz" },
    { "id": "sea", "name": "海边", "path": "/vrm/scene/sea.spz" }
  ],
  "gaussUserScenes": []
}
```

---

## 3. 目标架构: DivaPetView 内嵌桌宠

### 3.1 组件层级

```
NormalMode.vue                     ← 主窗口
└─ DivaPetView.vue                 ← 内嵌桌宠面板
   ├─ .avatar-section              ← 上部: VRM + 背景
   │   └─ DivaVrmAvatar.vue        ← Three.js 渲染
   │       └─ VrmRuntime            ← avatar-runtime-vrm
   │           ├─ SceneManager      ← 管理 scene/camera/renderer
   │           │   ├─ GaussSceneController ← ★ 3D 背景场景
   │           │   └─ PanoramaRenderer
   │           └─ ...
   └─ .chat-section                ← 下部: 消息 + 输入
       ├─ DivaPetVoicePanel
       ├─ 消息列表
       └─ 输入框
```

### 3.2 内嵌模式与桌面覆盖层模式的关键差异

| 属性 | 内嵌 (embedded) | 桌面覆盖层 (desktop-pet) |
|------|----------------|------------------------|
| window transparent | `false` (不透明窗口) | `true` (透明覆盖层) |
| renderer alpha | `false` | `true` |
| renderer clearColor | 不透明色 | `0x00000000` |
| FPS 限制 | 60 | 24 |
| Canvas 容器 | flex-1 自适应 | 全窗口 |
| UI 共存 | 与 CSS UI 层叠 | 独立窗口 |
| 场景可见性 | 被 CSS UI 部分遮挡 | 全窗口可见 |

**关键影响**: 在内嵌模式下，3D 场景背景只需要覆盖 VRM 模型后方的 canvas 区域，不需要填满整个窗口。`.chat-section` 用 CSS 背景色覆盖了下半部分。

### 3.3 集成点

| 文件 | 修改性质 | 内容 |
|------|----------|------|
| `types.ts` | 扩展 | `GaussSceneId`, `GaussSceneEntry`, `PetConfig` 新增字段 |
| `DivaVrmAvatar.vue` | 新增 prop + watcher | `backgroundScene` prop → `runtime.setBackgroundScene()` |
| `DivaPetView.vue` | 传递 prop + UI | 场景快速切换按钮 |
| `PetSettings.vue` | 新增区域 | 场景选择配置 |
| `public/vrm/scene/` | 新增资源 | 3 个 .spz 场景文件 |
| `package.json` | 新增依赖 | `@sparkjsdev/spark` |

**不需要修改**: `avatar-runtime-vrm/` (GaussSceneController 已完整), `NormalMode.vue`, Tauri 后端。

---

## 4. 运行时场景图结构

加载 3D 背景后，Three.js scene 的运行时结构：

```
THREE.Scene
├─ AmbientLight                               ← 环境光
├─ DirectionalLight (key)                     ← 主光源 + 阴影投射
├─ DirectionalLight (fill)                    ← 补光
│
├─ Group: gaussScene_home                     ← ☆ 3D 场景背景 (renderOrder: 0)
│   └─ SplatMesh
│       ├─ BufferGeometry (N splats)
│       └─ ShaderMaterial (GS custom shader)
│
├─ Mesh: shadowGround                         ← 阴影接收面
│   ├─ PlaneGeometry(20,20)
│   └─ ShadowMaterial({ opacity: 0.4 })
│
└─ Group: modelWrapper                        ← VRM 模型 (renderOrder: 1)
    └─ VRM.scene
        ├─ SkinnedMesh (body)
        │   ├─ castShadow = true
        │   └─ receiveShadow = true
        ├─ SkinnedMesh (hair)
        ├─ SkinnedMesh (face)
        └─ ...
```

**渲染顺序保证**: Three.js 默认按 `scene.children` 数组顺序渲染。场景 Group 在模型 Group 之前加入 → 先渲染（在屏幕空间中更远）。也可通过 `renderOrder` 显式控制。

---

## 5. 架构决策

| ID | 决策 | 理由 |
|----|------|------|
| AD1 | 复用 GaussSceneController（不修改运行时） | 已完整实现，含 dispose/unload/场景参数 |
| AD2 | 场景文件放 `public/vrm/scene/` | 与 VRM 模型/动画路径一致 |
| AD3 | 内嵌模式保持 `transparent: false` | 与现有 CSS 渐变背景视觉兼容 |
| AD4 | 场景选择 UI 在 DivaPetView + PetSettings 两处 | 快捷切换 + 深度配置 |
| AD5 | 场景切换失败静默回退 transparent | 不影响模型渲染，用户无感知 |
