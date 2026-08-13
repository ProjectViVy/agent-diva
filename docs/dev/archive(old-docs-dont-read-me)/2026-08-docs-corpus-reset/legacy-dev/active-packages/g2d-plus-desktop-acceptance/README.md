# G2D+ 真实桌面验收 Runbook

本目录是 E7 自动化发布候选之后的最终人工验收入口。执行者必须使用真实桌面
窗口观察结果；自动化测试、HTTP 调用或单元测试不能代替本验收。

## 1. 验收边界

- 覆盖原 G2D 六场景，以及完整“任务证据到 Recall 再回滚”的第七场景。
- 发布候选必须分别在一个全新 profile 和一个真实升级 profile 上执行。
- 不在本文档阶段修改生产数据、读取 `keys.txt` 或调用付费/外部 API。
- 若第七场景需要真实 provider，必须另行取得用户明确授权；授权前标为 `BLOCKED`，
  不得使用 fake provider 冒充通过。
- 任意重复 authority 修改、不可恢复状态、typed Memory degraded/fallback、原始
  Memory/prompt/tool payload 泄露，立即停止本轮并判定失败。

## 2. 开始前准备

### 2.1 发布候选基线

在仓库根目录记录以下信息到 `evidence-template.md` 的副本：

```powershell
git branch --show-current
git rev-parse HEAD
git status --short --branch --untracked-files=all
rustc --version
cargo --version
node --version
npm --version
```

要求：

1. 使用已通过 `just e7-automated-release-gate` 的同一 commit；
2. 除已说明的 `LOCK.md`/验收记录外没有未知代码变更；
3. 记录桌面包版本、文件名和 SHA-256：

```powershell
Get-FileHash -Algorithm SHA256 -LiteralPath '<release-candidate-path>'
```

如需重新产出候选包，应先执行 `just e7-automated-release-gate`，再从
`agent-diva-gui` 执行 `npm run tauri build`。重新构建后的 hash 必须写入记录。

### 2.2 Profile 隔离

准备两个目录，不复用同一个数据副本：

- `fresh-profile`：不存在历史 `config.json`、`.laputa` 或 Memory 数据；
- `upgrade-profile`：真实历史 profile 的可恢复副本，保留迁移前快照。

默认配置目录是用户目录下 `.agent-diva`；也可通过 CLI 的 `--config-dir` 或环境变量
`AGENT_DIVA_CONFIG_DIR` 指向验收副本。绝不直接在唯一生产 profile 上测试。

升级 profile 开始前，记录原路径、复制时间、目录大小和备份位置；不要把 profile、
数据库、日志或 `keys.txt` 提交到仓库。确保同一时刻只有一个 Agent Diva/Gateway
进程访问该 profile。

### 2.3 环境与可观察性

- Windows 桌面、WebView2、Rust/Node 工具链和足够磁盘空间可用；
- 关闭旧的 `agent-diva.exe`、Gateway 和开发版 GUI，避免端口/SQLite/PDB 占用；
- 应用启动后打开“进化 / Evolution”，确认 typed authority 为 ready，revision 可见，
  degraded reason 为空；
- 记录 GUI 版本、workspace、profile 类型、启动时间和 `gateway.port`；
- 日志默认位于 profile 下 `logs`（若 `config.json` 的 `logging.dir` 改为绝对路径，
  以该路径为准）；证据只保留脱敏后的相关片段。

## 3. 执行矩阵

每个场景都记录开始/结束时间、窗口、run/proposal/request/receipt/changelog/audit ID、
前后 revision、界面提示、日志相关 ID 和结论。ID 不存在时写 `N/A + 原因`。

| 场景 | 必须执行的 profile | 通过条件 |
|---|---|---|
| S1 仅批准 | fresh | 中风险提案只变为已批准，不提前应用；状态与 revision 一致 |
| S2 拒绝 | fresh | 拒绝后不能应用；相同内容受 suppression，持久 authority 不变 |
| S3 编辑后批准 | fresh | 编辑产生新 revision/digest；旧 request/receipt stale；新版本可批准应用 |
| S4 双窗口重复操作 | fresh | 两窗口同时“批准并应用”，只有一次 authority/changelog/audit 结果 |
| S5 重启恢复 | upgrade | 授权后、应用前退出并重启；状态恢复，执行仍最多一次 |
| S6 应用与回滚 | upgrade | apply 后可见；回滚一次成功；再次回滚被拒绝且 revision 一致 |
| S7 完整纵向闭环 | fresh + upgrade | evidence→AutoDream→proposal→apply→新会话 Recall；回滚后新会话不再 Recall |

## 4. 场景步骤

### S1 仅批准

1. 在 Evolution 提案收件箱选择一条中风险、证据可见的 pending 提案。
2. 记录 proposal ID、source run ID、当前 digest/revision 与 authority revision。
3. 点击“仅批准”并确认。
4. 验证成功提示、提案状态和审计关联；确认没有 apply changelog，authority revision
   没有因批准动作本身增加。

### S2 拒绝

1. 选择另一条 pending 提案，记录基线。
2. 点击“拒绝”并确认；尝试从当前界面再次应用。
3. 验证操作被禁止/拒绝、authority revision 不变，并记录明确 reason code。
4. 再次触发同内容候选时，验证原样内容不重新进入 pending；不要记录候选正文。

### S3 编辑后批准

1. 打开 pending 提案，记录编辑前 proposal revision/digest 和已产生的 request/receipt。
2. 使用可视化编辑保存一处非敏感变化。
3. 验证产生新 revision/digest；旧授权再次使用时返回 stale/版本冲突。
4. 对新版本执行“批准并应用”，确认恰好一个 changelog 和一次 authority revision 变化。

### S4 双窗口重复操作

1. 在两个真实桌面窗口打开同一 pending 提案，记录共同基线。
2. 尽可能同时点击“批准并应用”。
3. 验证一个成功，另一个得到幂等结果或稳定冲突；刷新两个窗口。
4. 确认只有一个 apply changelog、一个有效审计链和一次 authority 修改。

### S5 重启恢复

1. 在 upgrade profile 对 pending 提案完成“仅批准”，记录 receipt 和 revision。
2. 在应用前正常退出桌面；确认 Gateway 随桌面退出或单独关闭。
3. 从同一候选包和 profile 重启，不修改数据库文件。
4. 验证提案/receipt 恢复；继续应用并确认最多一次 authority 变化。
5. 若要验证异常退出，另开一轮副本，并清楚标记退出方式；不要在唯一升级副本执行。

### S6 应用与回滚

1. 对一条可追踪提案执行“批准并应用”，记录 changelog/audit/authority revision。
2. 在“审计与回滚”确认记录为可回滚，再点击“回滚”。
3. 验证 rollback ID、回滚后 revision 和界面状态；刷新/重启后状态保持。
4. 再次回滚同一 changelog，必须 fail closed 且不产生第二次 authority 变化。

### S7 完整纵向闭环

1. 在真实会话完成一个可验证、低敏感度的本地任务；记录任务结果的摘要/digest，
   不复制完整 prompt 或工具输出。
2. 打开 Evolution，点击“立即反思”；观察 run 从 queued/running 各阶段进入终态。
3. 验证 input coverage 包含该 evidence，且 proposal 关联 source run/evidence ID。
4. 审查后“批准并应用”；记录治理链和 typed authority revision。
5. 新建会话，用自然问题验证 Recall 命中刚应用的事实，并记录脱敏命中证据。
6. 回到“审计与回滚”执行回滚。
7. 再新建一个会话并重复问题；验证该记录不再被 Recall。
8. fresh 与 upgrade profile 各执行一次；其中任一轮依赖真实 provider 时，先暂停并请求
   用户授权使用相应桌面配置。

## 5. 统一停止条件

出现以下任一情况，立即停止当前 profile 后续场景，保留只读证据并创建缺陷：

- typed Memory 显示 degraded、fallback、identity mismatch 或 integrity failure；
- 同一操作造成多个 changelog、重复 authority revision 或无法解释的 revision 跳变；
- rejected/stale/consumed receipt 仍能产生写入；
- 重启后 proposal、receipt、run 或审计链丢失且无法恢复；
- 回滚后仍可 Recall，或回滚破坏了无关记录；
- GUI、日志、metrics 或事件暴露原始 Memory/prompt/tool payload；
- GUI 无明确错误但后台失败，或 reason code 在 Manager/Tauri/GUI 之间不一致。

缺陷必须加入根 `TODOLIST.md`，包括严重度、复现步骤、脱敏 ID、候选 commit、profile
类型、日志时间窗和预期行为。不得为了继续验收直接编辑 SQLite。

## 6. 收尾与判定

1. 完整填写 `evidence-template.md` 的副本；失败场景也必须保留记录。
2. 对日志和截图做脱敏检查；禁止提交 profile、数据库、token、prompt、Memory 内容。
3. fresh/upgrade 的要求场景全部 PASS，且无 P0/P1 缺陷时，才可建议关闭 G2D+。
4. G2D+ 通过后仍需单独决定是否发布、push 或清理 profile；本验收不自动授权这些动作。

故障采集与安全恢复见 [troubleshooting.md](troubleshooting.md)，记录格式见
[evidence-template.md](evidence-template.md)。
