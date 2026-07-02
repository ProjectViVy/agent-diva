# 13 - 验收标准

> Diva Pet 模块的交付验收清单

---

## 1. 功能验收

### 1.1 Live2D 角色渲染

| # | 测试用例 | 验收标准 | 优先级 |
|---|----------|----------|--------|
| F1.1 | 启动应用，进入 Diva Pet 页面 | Live2D 角色在 3s 内渲染完成 | P0 |
| F1.2 | 角色渲染包含呼吸动画 | 可见细微的身体起伏和眨眼 | P0 |
| F1.3 | 角色渲染包含空闲动作 | 随机播放 Idle 动作组（如挥手、转头） | P1 |
| F1.4 | 在 Settings 切换模型 | 新模型加载，旧模型释放，2s 内完成 | P0 |
| F1.5 | 导入新版 .model3.json 模型文件夹 | 正确解析，文件复制到 live2d_resource/ | P1 |
| F1.6 | 导入损坏的模型文件 | 显示明确错误提示，不崩溃 | P1 |
| F1.7 | 调节缩放滑条 | 角色大小实时变化 | P2 |
| F1.8 | 调节位置偏移 | 角色位置实时变化 | P2 |
| F1.9 | WebGL 不可用环境 | 降级显示静态占位图 + 说明文字 | P1 |
| F1.10 | 窗口最小化后恢复 | Live2D 渲染正确恢复 | P1 |
| F1.11 | DPI 变化（外接显示器） | Canvas 尺寸自适应，渲染不模糊 | P2 |

### 1.2 语音合成（TTS）

| # | 测试用例 | 验收标准 | 优先级 |
|---|----------|----------|--------|
| F2.1 | 文字回复自动触发 TTS | 回复完成后自动播放语音 | P0 |
| F2.2 | 配置 SiliconFlow API Key + 模型 | 使用 API 合成语音，音色自然 | P0 |
| F2.3 | API Key 无效 | 自动降级到浏览器 TTS | P1 |
| F2.4 | API 超时 (>30s) | 自动降级到浏览器 TTS，不阻塞 UI | P1 |
| F2.5 | 未配置 API Key | 直接使用浏览器 TTS（不报错） | P0 |
| F2.6 | 浏览器 TTS 不可用 | 静默跳过，不影响其他功能 | P1 |
| F2.7 | 新回复到达时旧 TTS 还在播放 | 停止旧播放，开始新播放 | P0 |
| F2.8 | 关闭语音功能 | TTS 不再播放 | P0 |
| F2.9 | 调节语速 | 语音播放速度变化 | P2 |
| F2.10 | 长文本 (>2000 字) | 分段合成播放，不截断 | P2 |

### 1.3 语音识别（ASR）

| # | 测试用例 | 验收标准 | 优先级 |
|---|----------|----------|--------|
| F3.1 | 点击麦克风按钮 | 开始语音监听 | P1 |
| F3.2 | 说中文短句 | 正确识别并发送到对话 | P1 |
| F3.3 | 说完后自动停止 | 识别完成时自动提交，恢复监听 | P1 |
| F3.4 | 麦克风权限被拒 | 显示权限引导提示 | P1 |
| F3.5 | 无麦克风设备 | 禁用按钮 + 提示 | P2 |
| F3.6 | TTS 播放中说话 | ASR 自动暂停直到 TTS 完成 | P2 |
| F3.7 | 关闭语音输入 | 按钮隐藏/禁用 | P2 |

### 1.4 Session 共享（关键验收）

| # | 测试用例 | 验收标准 | 优先级 |
|---|----------|----------|--------|
| F4.1 | DivaPet 发送消息 | 消息出现在 ChatView 中，使用同一 `currentChatId` | **P0** |
| F4.2 | ChatView 发送消息 | DivaPet 气泡同步显示最新回复 | **P0** |
| F4.3 | DivaPet ASR 语音输入 | 语音识别文本发送到当前 session | **P0** |
| F4.4 | 从 Chat 切换到 DivaPet | 历史消息在气泡中可见（latest 回复） | **P0** |
| F4.5 | 从 Settings 切回 DivaPet | 消息列表未丢失，气泡仍显示最新 | P1 |
| F4.6 | DivaPet 页面中清空对话 | ChatView 和 DivaPet 同步清空 | P1 |
| F4.7 | DivaPet 页面中加载历史 session | ChatView 和 DivaPet 同步切换到历史 | P1 |
| F4.8 | Agent 流式回复中切到 DivaPet | 气泡实时显示 streaming 文本（"..." 动画） | P0 |
| F4.9 | Agent 工具调用中切到 DivaPet | 气泡显示 tool-start / tool-end 状态 | P2 |

### 1.5 UI 与交互

| # | 测试用例 | 验收标准 | 优先级 |
|---|----------|----------|--------|
| F5.1 | 侧边栏包含 "Diva Pet" 入口 | 点击后切换到 DivaPet 页面 | P0 |
| F5.2 | Diva Pet 页面显示 Live2D + 控制栏 | 布局正确，无溢出 | P0 |
| F5.3 | 暗色/亮色主题切换 | 控制栏和气泡颜色适配 | P1 |
| F5.4 | 中英文切换 | 所有文案正确切换 | P1 |
| F5.5 | 从 Diva Pet 切到 Chat 再切回 | 角色状态保持，不重新加载 Live2D | P1 |
| F5.6 | Diva Pet 禁用时 | 侧边栏入口隐藏 | P1 |
| F5.7 | 对话气泡显示 | 流式更新回复文本 | P0 |

---

## 2. 非功能验收

### 2.1 性能

| # | 指标 | 目标 | 测量方法 |
|---|------|------|----------|
| NF1.1 | Live2D 首帧时间 | < 3s | 从 `init` 到 `load-success` 事件 |
| NF1.2 | 空闲帧率 | ≥ 30fps | DevTools FPS Meter |
| NF1.3 | 活跃帧率 | ≥ 55fps | DevTools FPS Meter |
| NF1.4 | 内存（含 Live2D） | < 200MB | Task Manager / DevTools Memory |
| NF1.5 | 包体积增量 | < 3MB | 对比 v0.4.x 和 v0.5.0 安装包 |
| NF1.6 | TTS 首字延迟 | < 2s | 从事件触发到音频开始 |

### 2.2 稳定性

| # | 标准 | 验证方法 |
|---|------|----------|
| NF2.1 | 连续运行 30 分钟无崩溃 | 长时间运行测试 |
| NF2.2 | 快速切换模型 10 次无内存泄漏 | Memory Profiler |
| NF2.3 | 窗口最小化/恢复 20 次无异常 | 手动压力测试 |
| NF2.4 | 快速切换页面 50 次无状态混乱 | 自动化 UI 测试 |

### 2.3 兼容性

| # | 标准 | 目标 |
|---|------|------|
| NF3.1 | Windows 10 兼容 | 全部功能正常 |
| NF3.2 | Windows 11 兼容 | 全部功能正常 |
| NF3.3 | macOS 12+ 兼容 | 主要功能正常 |
| NF3.4 | 旧版 config.json 兼容 | 无 `pet` section 时默认禁用 |

---

## 3. 回归验收

以下现有功能不得因 Diva Pet 的加入而受影响：

| # | 回归测试 | 验证方法 |
|---|----------|----------|
| R1 | Chat 正常发送/接收消息 | 发送消息 → LLM 正常回复 |
| R2 | Chat 流式输出正常 | 观察 "agent-response-delta" 事件 |
| R3 | Settings Providers 面板正常 | 打开/修改/保存 Provider 配置 |
| R4 | Settings Channels 面板正常 | 打开/修改/保存 Channel 配置 |
| R5 | 定时任务 (Cron) 正常 | 添加/触发/删除定时任务 |
| R6 | 会话管理正常 | 新建/切换/删除会话 |
| R7 | 工具调用正常 | 触发 tool-start / tool-end 事件 |
| R8 | 系统托盘正常 | 最小化到托盘/恢复 |
| R9 | 国际化正常 | 中英文所有页面文案正确 |
| R10 | CLI 版本不受影响 | `cargo build -p agent-diva-cli` 通过 |

---

## 4. 文档验收

| # | 文档 | 状态 |
|---|------|------|
| D1 | `01-architecture-exploration.md` | ✅ |
| D2 | `02-implementation-plan.md` | ✅ |
| D3 | `03-feasibility-assessment.md` | ✅ |
| D4 | `04-testing-strategy.md` | ✅ |
| D5 | `05-compatibility-migration.md` | ✅ |
| D6 | `06-error-handling-edge-cases.md` | ✅ |
| D7 | `07-performance-considerations.md` | ✅ |
| D8 | `08-configuration-dependencies.md` | ✅ |
| D9 | `09-project-management.md` | ✅ |
| D10 | `10-ui-design.md` | ✅ |
| D11 | `11-code-examples.md` | ✅ |
| D12 | `12-deployment-release.md` | ✅ |
| D13 | `13-acceptance-criteria.md` | ✅ |
| D14 | CHANGELOG.md 更新 | 待实现时 |
| D15 | README 更新 | 待实现时 |

---

## 5. 签核流程

| 角色 | 签核内容 | 签核标准 |
|------|----------|----------|
| **开发者** | 代码质量 + 测试覆盖 | 全部自动化测试通过 |
| **QA** | 功能验收 | 全部 P0/P1 用例通过 |
| **PM** | 用户体验 + 范围确认 | M1 Demo 通过 |
| **法务（如需）** | Live2D SDK 许可 | 许可兼容性确认 |
| **Maintainer** | 代码审查 + 合并 | PR Review Approved |

---

## 6. Go/No-Go 决策

### ✅ Go （全部满足）
- [ ] 所有 P0 功能用例通过
- [ ] 所有 P0 性能指标达标
- [ ] 零 Crash（30 分钟运行测试）
- [ ] 全部回归测试通过
- [ ] Live2D SDK 许可确认完成
- [ ] CI 构建通过（Windows + macOS）

### ⚠️ Conditional Go （可接受）
- [ ] P1 功能 ≤ 3 个未通过（需有已知问题记录）
- [ ] 1 个性能指标轻微超标（< 20%）

### ❌ No-Go （阻止发布）
- [ ] 任何 P0 功能用例未通过
- [ ] 任何 Crash
- [ ] 任何回归测试失败
- [ ] Live2D SDK 许可未确认
