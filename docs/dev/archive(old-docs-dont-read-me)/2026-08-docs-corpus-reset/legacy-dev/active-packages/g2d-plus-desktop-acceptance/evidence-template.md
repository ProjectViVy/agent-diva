# G2D+ 验收证据模板

> 复制本文件到本次验收迭代目录后填写。只记录脱敏元数据，不粘贴 prompt、Memory、
> 工具输出、密钥、数据库或完整 profile。

## A. 基线

- 执行人：
- 开始/结束时间（Asia/Shanghai）：
- Git branch / commit：
- `git status --short --branch`：
- 桌面版本 / 包文件 / SHA-256：
- Windows / WebView2 版本：
- Rust / Cargo / Node / npm 版本：
- 自动发布门记录：`just e7-automated-release-gate`（时间/结果）：

## B. Profile

| profile | 类型 | 脱敏路径标签 | 备份标签/时间 | 启动 revision | typed ready | degraded reason |
|---|---|---|---|---:|---|---|
| FRESH-01 | fresh | | N/A | | | |
| UPGRADE-01 | upgrade copy | | | | | |

## C. 场景结果

| 场景 | profile | 开始/结束 | 结果 PASS/FAIL/BLOCKED | 前→后 authority revision | 截图/日志证据编号 | 缺陷编号 |
|---|---|---|---|---|---|---|
| S1 仅批准 | FRESH-01 | | | | | |
| S2 拒绝 | FRESH-01 | | | | | |
| S3 编辑后批准 | FRESH-01 | | | | | |
| S4 双窗口重复操作 | FRESH-01 | | | | | |
| S5 重启恢复 | UPGRADE-01 | | | | | |
| S6 应用与回滚 | UPGRADE-01 | | | | | |
| S7 完整闭环 | FRESH-01 | | | | | |
| S7 完整闭环 | UPGRADE-01 | | | | | |

## D. 单场景证据卡（每场景复制一份）

- 场景 / profile：
- 操作窗口与点击顺序：
- run ID：
- proposal ID / revision / digest 前缀：
- governance request / receipt ID：
- changelog / audit / rollback ID：
- authority revision（前 / 后 / 回滚后）：
- GUI 成功或错误提示：
- Manager/Tauri reason code：
- 日志时间窗与证据编号：
- 截图编号（截图前已脱敏）：
- 最多一次写入检查：
- 重启后持久状态检查：
- Recall 结果（仅写 hit/miss 与记录 ID，不写内容）：
- 结果与备注：

## E. 安全检查

- [ ] 截图不含 token、路径中的个人信息、prompt 或 Memory 正文。
- [ ] 提交的日志片段不含 prompt/Memory/tool payload。
- [ ] 没有提交 profile、SQLite、`keys.txt`、备份或生成包。
- [ ] 所有失败均有稳定 reason code 或已登记缺陷。
- [ ] 未经授权没有调用真实外部 API。

## F. 最终签署

- fresh profile：PASS / FAIL / BLOCKED
- upgrade profile：PASS / FAIL / BLOCKED
- 未关闭 P0/P1 缺陷：
- G2D+ 建议：ACCEPT / REJECT / BLOCKED
- 执行人确认：
- 产品/用户确认：
