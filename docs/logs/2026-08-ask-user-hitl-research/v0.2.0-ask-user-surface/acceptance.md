# Acceptance — ask_user 表面闭环（Phase 2）

- 版本：`v0.2.0-ask-user-surface`
- 日期：2026-08-05

## 人工 smoke 步骤（真实 LLM 会话）

### 1. CLI 交互调研

```bash
just run -- chat
# 输入：帮我在 A/B/C 三个方案里做个偏好调研
# 期望：终端出现 [ask_user] 问题 + dialoguer 选项列表
# 选择某项（或 Other 输入）后：Agent 基于答案继续，不再声称「没有询问工具」
```

### 2. CLI headless（非 tty）不挂死

```bash
echo "做个调研" | just run -- chat < /dev/null
# 期望：挂起问题被自动 cancel，turn 正常结束（无 10 分钟等待）
```

### 3. GUI 互动调研

```bash
cd agent-diva-gui && npm run tauri dev   # 或 just start
# 发同样调研消息
# 期望：聊天内出现 ask_user 问题卡（选项按钮 + Other + 取消）
# 点选后：卡片消失，Agent 基于答案继续
```

## 自动化门（已执行）

- Manager handler 7 测试；CLI answerer headless 2 测试；GUI 组件 4 测试 +
  全量 451；vue-tsc；clippy -D warnings。

## 用户原对话级验收

多选题调研必须出现 `ask_user` tool call；GUI/CLI 可点选；答完后 Agent 基于
答案继续，而不是声称「没有询问工具」。第 1-3 步执行后勾选本项。
