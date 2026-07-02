# P0-3: 文件持久化原子性不一致且不完整

## 问题描述

main 分支中配置、记忆、会话三类本地持久化的写入策略不一致。

`agent-diva-core/src/config/loader.rs` 的 `ConfigLoader::save` 直接写目标文件：

```rust
pub fn save(&self, config: &Config) -> crate::Result<()> {
    std::fs::create_dir_all(&self.config_dir)?;
    let content = serde_json::to_string_pretty(config)?;
    std::fs::write(&self.config_path, content)?;
    Ok(())
}
```

`agent-diva-core/src/memory/manager.rs` 中 `save_memory`、`append_history`、`save_daily_note` 也直接 `std::fs::write`：

```rust
std::fs::write(&self.memory_path, &memory.content)?;
std::fs::write(&self.history_path, content)?;
std::fs::write(&path, &note.content)?;
```

`agent-diva-core/src/session/manager.rs` 当前已经不是直接覆盖：`save` 调用 `write_session_atomically`，后者使用临时文件、`sync_all`、备份文件和 rename：

```rust
let mut temp_file = File::create(&temp_path)?;
temp_file.write_all(content)?;
temp_file.sync_all()?;
drop(temp_file);
```

但该实现仍存在三个缺口：

- 临时路径固定为 `<session>.tmp`，并发写入同一 session 时可能互相覆盖。
- rename 后未 fsync 父目录，崩溃后目录项持久性不能完全保证。
- 没有跨进程文件锁，多个 agent-diva 进程同时写配置、记忆或 session 时仍可能竞争。

因此，审计问题应归纳为“持久化原子性不一致且不完整”，其中 config/memory 是直接覆盖，session 是部分原子但缺少并发与目录持久化保护。

## 影响评估

- 稳定性影响：进程崩溃、断电或写入中断可能留下截断的 `config.json`、`MEMORY.md`、`HISTORY.md` 或 daily note。
- 数据一致性影响：多个前端、CLI、服务进程同时运行时，后写覆盖先写，可能丢失会话、记忆或配置更新。
- 恢复成本：配置 JSON 截断会导致启动解析失败；记忆 Markdown 损坏会影响长期记忆和上下文注入。
- 运维风险：session 已有备份逻辑但 config/memory 没有统一备份和恢复策略，故障行为难以预测。

## 解决方案

抽取共享原子写工具，统一用于 config、memory、history、daily note、session。

建议实现一个 `atomic_write` 辅助函数：

```rust
pub fn atomic_write(path: &Path, content: &[u8]) -> crate::Result<()> {
    let parent = path.parent().unwrap_or_else(|| Path::new("."));
    std::fs::create_dir_all(parent)?;

    let file_name = path
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("file");
    let tmp_path = parent.join(format!(
        ".{}.{}.tmp",
        file_name,
        std::process::id()
    ));

    {
        let mut file = std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&tmp_path)?;
        use std::io::Write;
        file.write_all(content)?;
        file.sync_all()?;
    }

    std::fs::rename(&tmp_path, path)?;

    if let Ok(dir) = std::fs::File::open(parent) {
        let _ = dir.sync_all();
    }

    Ok(())
}
```

使用方式：

```rust
pub fn save(&self, config: &Config) -> crate::Result<()> {
    let content = serde_json::to_vec_pretty(config)?;
    atomic_write(&self.config_path, &content)
}

pub fn save_memory(&self, memory: &Memory) -> crate::Result<()> {
    atomic_write(&self.memory_path, memory.content.as_bytes())
}
```

进一步建议：

- 对同一目标文件增加进程内 mutex 和跨进程 lock file。
- session 临时文件使用唯一名称，避免并发覆盖固定 `.tmp`。
- 对配置写入保留 `.bak`，启动时检测 JSON 损坏并尝试恢复。
- 在 Windows 上验证 `rename` 覆盖行为，必要时使用 `replace_file` 或平台封装。

## 验证方法

执行：

```powershell
cargo test -p agent-diva-core atomic
cargo test -p agent-diva-core config
cargo test -p agent-diva-core memory
cargo test -p agent-diva-core session
just fmt-check
just check
```

应新增并通过以下测试：

- 写入成功后目标文件内容完整，临时文件被清理。
- 模拟写入中途失败时，旧文件仍保留。
- 多次并发写同一文件不产生截断文件或固定临时文件冲突。
- `ConfigLoader::save`、`MemoryManager::save_memory`、`append_history`、`save_daily_note` 均不再直接 `std::fs::write` 目标路径。
- session 写入仍保留现有恢复能力，并补足唯一临时文件和目录 fsync。

## 优先级

P0
