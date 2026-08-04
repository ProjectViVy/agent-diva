# 验收步骤

## 准备

1. `just fmt-check && just check && just test` 全绿；
2. `cd agent-diva-gui && pnpm install && pnpm tauri dev` 启动桌面端。

## 验收场景

### 场景 1：谨慎模式 → 每次都询问

1. 主页面右上角权限模式选 **"谨慎"**；
2. 发送 `桌面创建一个计算器cpp文件夹`；
3. **预期**：Approval Center drawer 自动从右侧弹出，卡片显示一条 `pending` 命令，命令名和路径清晰可见；
4. 点击 **Allow**；
5. **预期**：Agent 继续执行 `New-Item ...`，桌面出现 `计算器cpp` 文件夹。

### 场景 2：智能模式 → 沙箱失败才询问

1. 切回 **"智能"**；
2. 让 Agent 执行 `cmd /c echo hi`（受限令牌沙箱通常会拒绝）；
3. **预期**：命令先尝试，沙箱返回失败后，drawer 自动弹出 `pending` 审批；Allow 后以非沙箱方式重试。

### 场景 3：信任模式 → 仅高风险询问

1. 切到 **"信任"**；
2. 让 Agent 执行 `git status`；
3. **预期**：直接执行（不弹 drawer）。

### 场景 4：drawer 自动打开

- 关闭 drawer，再触发一次谨慎模式命令；
- **预期**：drawer 自动弹回。

### 场景 5：跨模式持久性

- 设置谨慎模式后切换会话再返回，仍保持谨慎模式（前端 ref 仍为 `cautious`）；
- 发送新消息，**预期**：仍按 OnRequest 策略询问。

## 回滚方案

- 代码侧 `git revert` 该 commit 即可；
- 配置侧默认值仍为 `OnFailure`，无持久化字段变更。
