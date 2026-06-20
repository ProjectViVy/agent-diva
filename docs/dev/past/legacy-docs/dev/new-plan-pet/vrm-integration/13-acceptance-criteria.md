# 13 - VRM 验收标准

> VRM 模块的交付验收清单

---

## 1. 功能验收

### 1.1 VRM 基础渲染

| # | 测试用例 | 验收标准 | 优先级 |
|---|----------|----------|--------|
| F1.1 | 进入 VRM Diva Pet 页面 | 3D 角色在 5s 内渲染完成 | P0 |
| F1.2 | 角色显示在透明背景上 | 无黑边，背景穿透到桌面 | P0 |
| F1.3 | 鼠标左键拖拽旋转视角 | OrbitControls 正常工作 | P0 |
| F1.4 | 滚轮缩放 | 相机距离变化 | P0 |
| F1.5 | 加载 VRM 1.0 模型 | 正确渲染，表情可用 | P0 |
| F1.6 | 加载 VRM 0.x 模型 | 自动旋转适配，正确渲染 | P1 |
| F1.7 | 加载损坏的 .vrm 文件 | 显示错误提示，不崩溃 | P1 |
| F1.8 | 加载非 VRM 的 glTF 文件 | 检测 userData.vrm === null，报错 | P2 |
| F1.9 | 切换模型 | 新模型加载，旧模型释放 | P1 |

### 1.2 表情系统

| # | 测试用例 | 验收标准 | 优先级 |
|---|----------|----------|--------|
| F2.1 | Agent 回复含 "哈哈" | 角色表情变为 happy | P0 |
| F2.2 | Agent 回复含 "难过" | 角色表情变为 sad | P0 |
| F2.3 | Agent 回复含 "wow" | 角色表情变为 surprised | P0 |
| F2.4 | 普通回复 | 角色表情保持 neutral | P0 |
| F2.5 | 手动点击 "开心" 按钮 | 角色表情变为 happy | P1 |
| F2.6 | 模型缺少某表情 | 跳过该表情，不报错 | P2 |

### 1.3 口型同步

| # | 测试用例 | 验收标准 | 优先级 |
|---|----------|----------|--------|
| F3.1 | TTS 播放中 | 嘴巴开合动画可见 | P0 |
| F3.2 | TTS 停止 | 嘴巴闭合 | P0 |
| F3.3 | 模型无口型 BlendShape | 功能静默禁用 | P2 |

### 1.4 渲染器切换

| # | 测试用例 | 验收标准 | 优先级 |
|---|----------|----------|--------|
| F4.1 | 配置 renderer = "vrm" | VRM 角色渲染 | P0 |
| F4.2 | 配置 renderer = "live2d" | Live2D 角色渲染 | P0 |
| F4.3 | 配置 renderer = "static" | 静态图显示 | P1 |
| F4.4 | 运行时切换渲染器 | 旧渲染器释放，新渲染器加载 | P1 |

### 1.5 Session 共享（与 Live2D 通用）

| # | 测试用例 | 验收标准 | 优先级 |
|---|----------|----------|--------|
| F5.1 | VRM 页面输入文字 | 消息出现在 ChatView | P0 |
| F5.2 | ChatView 输入文字 | VRM 气泡 + 表情同步 | P0 |
| F5.3 | 切换 Chat → VRM | 消息不丢失 | P0 |

---

## 2. 非功能验收

### 2.1 性能

| # | 指标 | 目标 |
|---|------|------|
| NF1.1 | VRM 首帧时间 | < 5s |
| NF1.2 | 空闲帧率 | ≥ 30fps |
| NF1.3 | 活跃帧率 | ≥ 50fps |
| NF1.4 | 内存 (含 VRM) | < 200MB |
| NF1.5 | 包体积增量 | < 1MB (gzip) |

### 2.2 稳定性

| # | 标准 | 验证方法 |
|---|------|----------|
| NF2.1 | 连续运行 30 分钟无崩溃 | 长时间运行测试 |
| NF2.2 | 模型切换 10 次无泄漏 | Memory Profiler |
| NF2.3 | 窗口最小化/恢复 20 次 | 手动压力测试 |

### 2.3 许可

| # | 标准 |
|---|------|
| NF3.1 | 所有 npm 依赖为 MIT 许可 |
| NF3.2 | 默认 VRM 模型为 CC0 或明确允许分发 |
| NF3.3 | README 含模型许可声明 |
| NF3.4 | 不包含任何 AGPL-3.0 代码 |

---

## 3. Go/No-Go

### ✅ Go
- [ ] 所有 P0 用例通过
- [ ] 性能指标达标
- [ ] 零崩溃（30 分钟）
- [ ] 许可合规确认

### ❌ No-Go
- [ ] P0 失败
- [ ] 崩溃
- [ ] 许可违规

---

## 4. 文档交付

| # | 文档 | 状态 |
|---|------|------|
| D1 | 01-architecture-exploration.md | ✅ |
| D2 | 02-implementation-plan.md | ✅ |
| D3 | 03-feasibility-assessment.md | ✅ |
| D4 | 04-testing-strategy.md | ✅ |
| D5 | 05-compatibility-migration.md | ✅ |
| D6 | 06-error-handling-edge-cases.md | ✅ |
| D7 | 07-performance-considerations.md | ✅ |
| D8 | 08-configuration-dependencies.md | ✅ |
| D9 | 09-project-management.md | ✅ |
| D10 | 10-ui-design.md | ✅ |
| D11 | 11-code-examples.md | ✅ |
| D12 | 12-deployment-release.md | ✅ |
| D13 | 13-acceptance-criteria.md | ✅ |
