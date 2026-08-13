# G2D+ 故障采集与安全恢复

## 1. 先保全证据

故障发生后先停止点击，不重复触发写操作。记录本地时间、场景、profile 标签、当前
页面、run/proposal/request/receipt/changelog ID 和 authority revision。截图前遮蔽内容，
只复制故障时间窗内的脱敏日志。

日志目录通常是 `<profile>/logs`；若 `config.json` 中 `logging.dir` 为绝对路径，则使用
该目录。`gateway.port` 位于 profile 根目录。不得将整个日志目录或 profile 加入 Git。

可安全执行的只读诊断：

```powershell
Get-Process | Where-Object { $_.ProcessName -like '*agent-diva*' }
Get-ChildItem -LiteralPath '<profile>\logs' | Sort-Object LastWriteTime -Descending
Get-Content -Tail 200 -LiteralPath '<selected-log-file>'
Get-Content -Raw -LiteralPath '<profile>\gateway.port'
```

输出在保存前必须人工检查并脱敏。不要在终端中输出 `keys.txt` 或完整 `config.json`。

## 2. 常见问题

| 现象 | 优先检查 | 处理边界 |
|---|---|---|
| GUI 无法连接 Gateway | 是否有旧进程、`gateway.port`、启动日志 | 关闭旧进程后用同一候选包重启；不改数据库 |
| typed Memory degraded | health/Evolution reason、workspace identity、备份清单 | 立即判 FAIL；保留副本，不启用 legacy fallback |
| SQLite/PDB/端口占用 | 旧 GUI/Gateway/debug 进程 | 正常退出进程后重试启动；不删除数据库/WAL |
| 提案状态不刷新 | 两窗口状态、事件重连、刷新后服务端状态 | 记录 UI 与服务端差异；不重复点击写操作 |
| stale/consumed receipt | proposal revision、编辑时间、授权版本 | 预期 fail closed；若仍写入则 P0 |
| apply/rollback 超时 | changelog/audit 是否已落盘 | 先刷新只读状态；不得盲目重放，确认幂等结果后再决定 |
| AutoDream 无候选 | run 终态、failure code、input omissions | `no_candidates` 可是诚实结果；provider/error 必须明确显示 |
| Recall 回滚后仍命中 | record ID、会话是否为新建、rollback 状态 | 停止验收并登记阻断缺陷 |

## 3. 恢复原则

- 只在验收副本上执行恢复；原始升级 profile 保持不变。
- 优先使用产品提供的重启、重试、回滚路径，不直接修改 SQLite/manifest。
- identity migration 的回滚使用正式 migration CLI，并在执行前保存 manifest 与备份
  元数据；不要手工替换正在被进程打开的数据库。
- 任何删除、覆盖 profile 或恢复备份的动作都需要再次确认准确路径并获得用户授权。
- 恢复成功不自动将原失败场景改为 PASS；必须从干净副本重新执行并保留两次记录。

## 4. 缺陷分级

- `sev-P0`：越权/重复 authority 写、数据丢失、回滚无效、敏感载荷泄露、错误 authority。
- `sev-P1`：主场景不可完成、重启不可恢复、typed authority 持续 degraded。
- `sev-P2`：明确替代路径存在的交互/诊断问题或非阻断契约不一致。
- `sev-P3`：不影响判定的视觉或文案问题。

缺陷必须写入 `TODOLIST.md`；若在同一迭代修复，也应在验收记录中保留原始 FAIL、
修复 commit 和重新执行结果。
