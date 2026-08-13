# R4 保护性分支协议

- 状态：`Research Draft / Protocol Only`
- 日期：2026-08-13
- 性质：何时、从哪次提交、如何验证与演练保护性分支；**现在不创建分支**
- 产品：保护分支只用于追溯与恢复，不能成为运行时 fallback，也不能并回作为兼容层

## 1. 为什么现在不能建

EPIC：Architecture Gate 通过、D4 指定且验证过的准确提交之后，才允许
`COGNITIVE-I0-PROTECTION-BASELINE`。

当前 `agent-diva-pro` tip（本包撰写时）是

`ac6edeb77914a6d1707f60a6c01c3a75c640d878`
`docs(research): deliver COGNITIVE-R3 persona workspace package`

这是**研究文档** tip，不是删除前基线：

- D0–D4 尚未写；实施切片未知
- R4 自身提交还会往前走
- tip 相对 `origin/agent-diva-pro` 已超前约 96+ 提交（会话初 git status），未经用户
  Research Gate / Architecture Gate
- `just ci` 在 R0–R4 文档轮次**未**作为交付门禁跑

把今天的 HEAD 标成保护分支会冻错对象。

## 2. 创建时机（门禁）

同时满足才建分支：

1. 用户完成 R0–R4 Research Gate
2. 用户批准 D0–D4，D4 写明「保护基线 = 提交 S」
3. 提交 S 上跑通 D4 指定的自动门禁（至少 `just ci` + GUI 自动门；真机可另记）
4. 工作树干净，S 是该线上的明确 commit（允许 annotated tag）
5. 用户显式授权「现在创建保护分支」

未满足时只许更新本协议，不许 `git branch` / `git tag` 保护基线。

## 3. 命名（建议，非批准）

| 对象 | 建议名 | 用途 |
| --- | --- | --- |
| 分支 | `protect/pre-cognitive-clean-break` | 长期可 checkout 的删除前树 |
| 轻量 tag | `protect-pre-cognitive-clean-break-<yyyy-mm-dd>` | 人读日期 |
| 精确 tag | `protect-pre-cognitive-clean-break-<12-char-sha>` | 与 S 绑定，不可移动 |

分支从 S **快进创建**，不在保护分支上做功能开发。若必须修保护树的构建断裂，另开
`protect/hotfix/*`，并在 D4 记录；hotfix 不得加入新 runtime 兼容。

禁止：`backup/`、`legacy-runtime/`、`compat/` 这类暗示可切回当生产的名字。

## 4. 基线验证（创建当下）

在 S 上记录并保存输出：

```text
git rev-parse HEAD
git status --short --untracked-files=all    # 必须空
just ci                                     # 允许预记录的已知失败（如 CLI-WIREMOCK-502）
just laputa-clean-break-check               # 现有 mentle/migration 门
just bml-boundary-check
```

GUI 若 D4 要求：`npm test` + `npm run build`。把日志放进
`docs/logs/<theme>/v*/verification.md`，并抄 SHA、日期、已知失败列表。

S 必须能 `cargo build -p agent-diva-cli`（或 D4 指定的二进制），以便恢复演练
真的能跑旧程序。

## 5. 使用规则

允许：

- 只读对照：`git show protect/...:path`
- 整树 checkout 到**隔离 worktree** 跑旧 GUI，导出用户要手抄的正文
- 灾难恢复：用户明确要求时，用旧二进制 + 用户备份目录回到删除前行为

禁止：

- 新二进制探测保护分支或旧路径
- 保护分支当 origin 默认、当 CI fallback job
- 从保护分支 cherry-pick 「兼容读取」进 mainline
- 把保护分支 merge 回 `agent-diva-pro` 当功能

## 6. 恢复演练（D4 后做一次，现只定剧本）

目标：证明「回旧程序」可操作，而不是「新程序读旧数据」。

1. 从保护 tag 建隔离 worktree
2. 构建旧 CLI/GUI
3. 指向**事先复制**的工作区副本（不要对生产目录演练）
4. 观察：Persona JSON 编辑器仍在、Memory 页仍列 BML、Approval Center 危险工具仍在
5. 记录：哪些用户正文只存在于 JSON / `memory_md` / proposals，哪些已在 BML
6. 销毁 worktree；不把演练提交推进保护分支

失败标准：旧树编不过、副本损坏、演练误写入生产 `.laputa`。

## 7. 发布失败时怎么退

| 失败点 | 退法 |
| --- | --- |
| 删除切片编不过 / `just ci` 红 | `git revert` 该切片；保护分支不动 |
| 新 runtime 启动后用户数据「看不见」 | 预期之内则按发布说明；若误删 KEEP 面（BML/skills/M3）则 revert 切片并从备份恢复文件 |
| 用户要回整段旧产品 | checkout 保护 tag + 用户全量目录备份；**不要**在新代码里加读旧格式 |

## 8. 与现有 git 状态的关系

- 本工作区在 `agent-diva-pro`，研究包未 push。保护分支创建时应同时 tag **已 push
  或至少用户确认不会 force-drop 的**对象。若 S 只在本地，D4 必须先决定 push 或
  额外打离线 bundle。
- `LOCK.md` 不是基线的一部分；创建保护分支时不要把未发布的锁文件内容当契约。

## 9. 本协议不决定

- S 的具体 SHA（D4 + 用户）
- 删除切片顺序与提交数量（D4）
- 是否同时打 GitHub release（超出本 EPIC 研究）
