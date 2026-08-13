# 04 — 测试策略

## 1. 测试层次

```
E2E: 手动验证 (启动应用, 切换场景, 观察渲染, 检查内存)
  ├─ 集成测试 (Vitest + mock runtime)
  │   ├─ DivaPetView 场景切换 UI 测试
  │   └─ PetSettings 场景配置 UI 测试
  └─ 单元测试 (Vitest)
      ├─ types.test.ts: 类型定义完整性
      └─ pet-config.test.ts: 配置持久化
```

---

## 2. 单元测试

### 2.1 类型定义测试 (`types.test.ts`)

```typescript
describe('GaussScene 类型定义', () => {
  it('DEFAULT_PET_CONFIG.selectedGaussSceneId === "transparent"')
  it('DEFAULT_PET_CONFIG.gaussSceneList 长度 === 4')
  it('每个 GaussSceneEntry 有 id/name/path/isDefault')
  it('transparent 的 path 为空字符串')
})
```

### 2.2 配置持久化测试 (`pet-config.test.ts`)

```typescript
describe('场景配置持久化', () => {
  it('旧配置(无场景字段) → 自动合并默认值')
  it('场景ID不在列表中 → 回退 transparent')
  it('选择 home → 读取 home')
  it('清空列表 → UI 显示"暂无可用场景"')
})
```

---

## 3. 集成测试

### 3.1 DivaVrmAvatar 背景场景 (`DivaVrmAvatar.test.ts` 扩展)

```typescript
describe('背景场景集成', () => {
  it('backgroundScene prop → runtime.setBackgroundScene 被调用')
  it('backgroundScene=transparent → 不清除现有模型')
  it('场景切换 → 旧场景先 dispose 再加载新场景')
  it('场景加载失败 → 自动调用 setBackgroundScene("transparent")')
  it('快速切换 3 次 → 无竞态，最终场景正确')
  it('模型加载期间切换场景 → 正常工作')
})
```

### 3.2 DivaPetView 场景 UI (`DivaPetView.test.ts` 扩展)

```typescript
describe('场景快捷切换 UI', () => {
  it('场景按钮可见')
  it('点击展开下拉菜单 → 显示所有场景')
  it('当前场景高亮 (active class)')
  it('点击场景项 → petConfig.selectedGaussSceneId 更新')
  it('点击空白区域 → 下拉关闭')
})
```

### 3.3 PetSettings 场景 UI

```typescript
describe('PetSettings 场景配置', () => {
  it('显示 "3D 背景场景" section')
  it('radio 选择后 → petConfig 更新')
  it('当前选中 radio 匹配 petConfig.selectedGaussSceneId')
})
```

---

## 4. E2E 手动验证

### 4.1 功能验证

| ID | 操作 | 期望 |
|----|------|------|
| E1 | 启动应用 → 查看桌宠 | 背景透明 (默认) |
| E2 | 齿轮按钮 → 场景下拉 → 室内 | Canvas 中出现室内 3D 背景 |
| E3 | 切换到海边 | 海边背景 |
| E4 | 切换到太空 | 太空背景 |
| E5 | 切回透明 | 恢复透明 |
| E6 | PetSettings → 切换场景 | DivaPetView 实时更新 |
| E7 | 重启应用 | 上次选择的场景保留 |
| E8 | 切换 VRM 模型 | 场景不变 |

### 4.2 稳定性

| ID | 操作 | 期望 |
|----|------|------|
| S1 | 快速切换场景 10 次 | 无崩溃 |
| S2 | 模型说话中切换场景 | 口型同步不中断 |
| S3 | 打开设置面板不关闭桌宠 | 场景持续渲染 |
| S4 | 删除 scene/home.spz → 选择室内 | 自动回退透明，无报错 |
| S5 | 运行 10 分钟 | JS heap 无持续增长 |

### 4.3 性能

| ID | 指标 | 目标 |
|----|------|------|
| P1 | 透明背景 FPS | ≥ 55 |
| P2 | 室内场景 FPS | ≥ 40 |
| P3 | 海边场景 FPS | ≥ 30 |
| P4 | 太空场景 FPS | ≥ 50 |
| P5 | 场景切换内存增量 (vs 透明) | ≤ +200 MB |
| P6 | 场景加载时间 | ≤ 10s |

### 4.4 回归检查

- [ ] VRM 模型正常加载/渲染
- [ ] 动画 (idle/one-shot) 正常
- [ ] 表情/情绪切换正常
- [ ] TTS 语音播报正常
- [ ] 字幕显示正常
- [ ] 消息输入/发送正常
- [ ] PetSettings 其他功能正常 (ASR/TTS/音色)
- [ ] 模型管理器 (齿轮按钮) 正常
