# 09 — 开发项目管理资料

## 1. WBS

```
Diva Pet 3D 背景场景
├─ A. 依赖与资源 (0.5h)
│   ├─ A1. pnpm add @sparkjsdev/spark
│   ├─ A2. 复制 .spz → public/vrm/scene/
│   └─ A3. 验证 import 可解析
├─ B. 类型与配置 (0.5h)
│   ├─ B1. 新增 GaussSceneId, GaussSceneEntry 类型
│   ├─ B2. 扩展 PetConfig + DEFAULT_PET_CONFIG
│   └─ B3. pet-config.ts 合并逻辑
├─ C. VRM 组件集成 (1h)
│   ├─ C1. DivaVrmAvatar 新增 props
│   ├─ C2. 场景同步 watcher + 防竞态
│   └─ C3. DivaPetView 传递 prop
├─ D. DivaPetView UI (1h)
│   ├─ D1. 场景切换按钮 (齿轮旁)
│   ├─ D2. 场景下拉菜单
│   └─ D3. selectScene 交互
├─ E. PetSettings UI (1h)
│   ├─ E1. 场景配置 section
│   ├─ E2. 场景单选组
│   └─ E3. 样式 (复用 .settings-card 风格)
└─ F. 测试 (1h)
    ├─ F1. 单元测试 (types.test.ts, pet-config.test.ts)
    ├─ F2. 集成测试 (DivaVrmAvatar)
    └─ F3. E2E 手动验证
```

## 2. 里程碑

| ID | 描述 | 验证 |
|----|------|------|
| M1 | 依赖 + 资源就绪 | `ls public/vrm/scene/` → 3 个 .spz |
| M2 | 类型编译通过 | `vue-tsc --noEmit` |
| M3 | 场景可在运行时加载 (代码层面) | DivaVrmAvatar 调用 setBackgroundScene |
| M4 | DivaPetView UI 可用 | 齿轮旁出现场景按钮 |
| M5 | PetSettings UI 可用 | 设置中有场景选择 |
| M6 | 测试通过 | `npm run test` 全绿 |
| M7 | 文档完成 | 13 份文档 |

## 3. 影响范围

| 文件 | 操作 | 行数 (估) |
|------|------|----------|
| `types.ts` | 扩展 | +25 |
| `pet-config.ts` | 扩展 | +10 |
| `DivaVrmAvatar.vue` | 修改 | +30 |
| `DivaPetView.vue` | 修改 | +40 |
| `PetSettings.vue` | 修改 | +50 |
| `package.json` | 新增依赖 | +1 |
| `public/vrm/scene/` | 新增 | 3 个文件 |

## 4. 风险

| ID | 风险 | 缓解 |
|----|------|------|
| R1 | spark 与 Three 184 不兼容 | vendor alias 备选 |
| R2 | .spz 文件过大 | 提供降级选项 |
| R3 | 内嵌 Canvas 视口闪烁 | 设置接近场景色调的 clearColor |

## 5. 依赖

```
A ──┐
    ├──→ C ──→ D
B ──┘        └──→ E
```
