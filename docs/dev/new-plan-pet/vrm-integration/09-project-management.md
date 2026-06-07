# 09 - VRM 开发项目管理

> VRM 模块的项目规划、里程碑与资源分配

---

## 1. 项目概览

| 属性 | 值 |
|------|-----|
| **项目名称** | VRM Integration — Agent Diva 3D 桌宠 |
| **项目代号** | `vrm-integration` |
| **目标** | 在 Agent Diva GUI 中实现基于 @pixiv/three-vrm 的 3D 角色渲染 |
| **开发周期** | 1 周（5 个工作日） |
| **依赖** | `three` + `@pixiv/three-vrm` (MIT 许可) |
| **风险等级** | 低（核心技术均为 MIT npm 包） |

---

## 2. 里程碑规划

```
Day 1              Day 2              Day 3              Day 4              Day 5
│                   │                   │                   │                   │
├─ Phase 1 ────────┤                   │                   │                   │
│ 基础设施          │                   │                   │                   │
│ - npm 依赖        │                   │                   │                   │
│ - 目录结构        │                   │                   │                   │
│ - 类型声明        │                   │                   │                   │
│                   │                   │                   │                   │
├─ Phase 2: VRM ────────────────────────┤                   │                   │
│ 基础渲染          │                   │                   │                   │
│ - Three.js 初始化 │                   │                   │                   │
│ - VRM 加载        │                   │                   │                   │
│ - 渲染循环        │                   │                   │                   │
│ - DivaVrmAvatar   │                   │                   │                   │
│                   │                   │                   │                   │
│                   ├─ Phase 3 ────────────────────────────┤                   │
│                   │ 表情 + 动画                          │                   │
│                   │ - useVrmExpression                   │                   │
│                   │ - useVrmAnimation                    │                   │
│                   │ - 情绪关键词映射                     │                   │
│                   │                                      │                   │
│                   │                                      ├─ Phase 4 ────────┤
│                   │                                      │ 口型 + 集成       │
│                   │                                      │ - useVrmMouthSync │
│                   │                                      │ - DivaPetView集成 │
│                   │                                      │ - 配置管理        │
│                   │                                      │                   │
│                   │                                      │                   ├─ Phase 5 ──┤
│                   │                                      │                   │ 优化打磨   │
│                   │                                      │                   │ - 性能     │
│                   │                                      │                   │ - 边界     │
│                   │                                      │                   │ - 文档     │
│                   │                                      │                   │            │
│                   │                                      │                   └─ M1: MVP ──┘
```

---

## 3. 任务分解（WBS）

### Phase 1: 基础设施（0.5d）

| ID | 任务 | 工时 |
|----|------|------|
| 1.1 | `pnpm add three @pixiv/three-vrm` | 0.1d |
| 1.2 | 创建 `features/diva-pet-vrm/` 目录 | 0.1d |
| 1.3 | TypeScript 声明文件 | 0.1d |
| 1.4 | 准备测试 VRM 模型 | 0.2d |

### Phase 2: VRM 基础渲染（1.5d）

| ID | 任务 | 工时 |
|----|------|------|
| 2.1 | Three.js 环境搭建 (Scene/Camera/Light) | 0.3d |
| 2.2 | VRM 模型加载 (GLTFLoader + VRMLoaderPlugin) | 0.3d |
| 2.3 | 渲染循环 (requestAnimationFrame + vrm.update) | 0.2d |
| 2.4 | DivaVrmAvatar.vue 组件 | 0.5d |
| 2.5 | 模型加载/错误状态处理 | 0.2d |

### Phase 3: 表情与动画（1d）

| ID | 任务 | 工时 |
|----|------|------|
| 3.1 | useVrmExpression composable | 0.3d |
| 3.2 | 情绪关键词映射 | 0.1d |
| 3.3 | useVrmAnimation composable | 0.3d |
| 3.4 | .vrma 动画加载与播放 | 0.2d |
| 3.5 | 动画状态机 (idle/talking/reacting) | 0.1d |

### Phase 4: 口型同步 + 集成（1.5d）

| ID | 任务 | 工时 |
|----|------|------|
| 4.1 | useVrmMouthSync (正弦波版) | 0.3d |
| 4.2 | DivaPetView 渲染器切换 | 0.3d |
| 4.3 | VRM 配置管理 (config.json + UI) | 0.3d |
| 4.4 | Tauri commands (pet_list_vrm_models) | 0.3d |
| 4.5 | 国际化文案 | 0.2d |

### Phase 5: 优化打磨（0.5d）

| ID | 任务 | 工时 |
|----|------|------|
| 5.1 | 性能优化 (DPR cap, visibility pause) | 0.2d |
| 5.2 | 错误处理完善 | 0.1d |
| 5.3 | 文档整理 | 0.2d |

---

## 4. 风险登记表

| ID | 风险 | 概率 | 影响 | 缓解 |
|----|------|------|------|------|
| R1 | Three.js 版本与 @pixiv/three-vrm 不兼容 | 低 | 高 | 版本锁定 |
| R2 | WebGL 性能在低端设备不足 | 中 | 中 | DPR cap + 降级到 Live2D |
| R3 | VRM 模型许可不明确 | 低 | 中 | 使用 CC0 模型 |
| R4 | 用户混淆 VRM 与 Live2D 选择 | 中 | 低 | 清晰 UI 引导 |

---

## 5. 交付清单

- [ ] `features/diva-pet-vrm/` 完整模块代码
- [ ] `public/vrm/models/` 至少 1 个默认 VRM 模型
- [ ] `public/vrm/animations/` 至少 3 个默认动画
- [ ] `config.json` vrm section 支持
- [ ] DivaPetView 多渲染器切换
- [ ] 中英文国际化
- [ ] 13 份开发文档
- [ ] 基础单元测试
- [ ] Windows 构建验证
