# 13 — 验收标准

## 1. 功能验收

| ID | 功能 | 操作 | 期望 |
|----|------|------|------|
| F1 | 默认透明背景 | 启动 → 查看桌宠 | 背景透明 |
| F2 | 室内场景 | 场景按钮 → 室内 | Canvas 出现室内 3D 背景 |
| F3 | 海边场景 | 场景按钮 → 海边 | 海边 3D 背景 |
| F4 | 太空场景 | 场景按钮 → 太空 | 太空 3D 背景 |
| F5 | 切回透明 | 选透明 | 恢复透明 |
| F6 | 设置面板同步 | PetSettings 中切换 | DivaPetView 实时更新 |
| F7 | 重启持久化 | 选场景 → 重启 | 场景保留 |
| F8 | 模型切换后场景保持 | 换 VRM 模型 | 背景不变 |
| F9 | 场景加载失败回退 | 删除 home.spz → 选室内 | 自动透明，无崩溃 |
| F10 | 快速切换 | 连切 5 次 | 无闪烁/崩溃 |

## 2. UI 验收

| ID | 检查项 |
|----|--------|
| U1 | 齿轮按钮旁出现场景按钮 (Image 图标) |
| U2 | 点击展开下拉 → 显示 4 个场景 |
| U3 | 当前场景高亮 (粉色系) |
| U4 | 点击场景项 → 下拉关闭 + 配置更新 |
| U5 | 点击空白区域 → 下拉关闭 |
| U6 | PetSettings 显示 "📺 3D 背景场景" section |
| U7 | radio 选中匹配 petConfig.selectedGaussSceneId |

## 3. 性能验收

| ID | 指标 | 目标 | 工具 |
|----|------|------|------|
| P1 | 透明 FPS | ≥ 50 | DevTools |
| P2 | 室内 FPS | ≥ 40 | DevTools |
| P3 | 海边 FPS | ≥ 30 | DevTools |
| P4 | 太空 FPS | ≥ 50 | DevTools |
| P5 | 场景加载 | ≤ 10s | 手动计时 |
| P6 | 场景 JS heap 增量 | ≤ +200 MB | Memory panel |
| P7 | 30min 运行 | 无持续内存增长 | Memory timeline |

## 4. 回归验收

- [ ] VRM 模型正常
- [ ] 动画 (idle/one-shot) 正常
- [ ] 表情/情绪切换正常
- [ ] TTS 语音播报正常
- [ ] 字幕正常
- [ ] 消息输入/发送正常
- [ ] PetSettings 其他功能 (ASR/TTS/音色) 正常
- [ ] 模型管理器正常
- [ ] 桌面覆盖层模式 (DesktopPetOverlay) 不受影响

## 5. 代码质量

```bash
npx vue-tsc --noEmit   # → 无类型错误
npm run test             # → 全部通过
```

## 6. 快速验证脚本

```bash
cd agent-diva/agent-diva-gui
npx vue-tsc --noEmit && npm run test && npm run dev

# 手动:
# 1. 齿轮旁场景按钮 → 逐个选场景 → 观察 Canvas
# 2. PetSettings → 切换场景 → 观察同步
# 3. Console 无 error
# 4. 重启 → 场景保留
```
