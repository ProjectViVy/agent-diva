# Summary — LAPUTA-STORAGE-STALE-LOCK-FLAKE

- 版本：`v0.5.0-stale-lock-recovery`
- 日期：2026-08-23
- 类型：锁回收正确性修复

## 根因

`recover_stale_lock` 在 `age >= stale_after` 之后还要求 `lock_file_is_recoverable`：
锁文件**没有**可解析的 `pid=`。生产 `try_create_lock` 总是写入 `pid=`，所以自己留下的
锁永远不能按 mtime 回收，Windows 全量负载下表现为偶发 `LockTimeout`。

## 做了什么

- 按 mtime 回收；默认 `stale_after` 仍是 5 分钟。
- 补测：带 `pid=1` 的陈旧锁在 `stale_after=0` 时能被拿下。
- 新鲜锁超时测仍在 `tests/storage.rs`。

## 影响范围

- `agent-diva-laputa/src/lock.rs`
- `TODOLIST.md`
- 本日志
