# P1-8: SQLite 未启用 foreign_keys/WAL

## 问题描述

`agent-diva-core/src/planning/store.rs` 的 `SqlitePlanningStore::new(pool: SqlitePool)` 负责初始化 planning 域 SQLite 表结构。表定义中已经使用了外键和级联删除：

```rust
plan_id TEXT NOT NULL REFERENCES plans(id) ON DELETE CASCADE
```

该外键出现在 `plan_steps`、`todo_items`、`planning_events` 和 `active_plan` 等表中。但初始化逻辑只执行 `CREATE TABLE IF NOT EXISTS ...`，没有在连接上启用：

```sql
PRAGMA foreign_keys = ON;
PRAGMA journal_mode = WAL;
PRAGMA busy_timeout = ...;
```

SQLite 的外键约束默认按连接关闭。`ON DELETE CASCADE` 写在 schema 里并不足以保证运行时生效；每个连接都需要启用 `foreign_keys`。当前 `SqlitePlanningStore::new` 接收外部传入的 `SqlitePool`，因此如果调用方没有在池连接生命周期中设置 PRAGMA，planning store 无法保证引用完整性。

代码搜索结果也显示 `agent-diva-core/src/planning/store.rs` 中不存在 `PRAGMA`、`foreign_keys`、`journal_mode` 或 `busy_timeout` 初始化语句。测试 helper 使用 `SqlitePoolOptions::new().connect("sqlite::memory:")` 后直接调用 `SqlitePlanningStore::new(pool)`，同样没有启用这些选项。

## 影响评估

数据一致性影响较高。删除 plan 后，依赖 `ON DELETE CASCADE` 的 step、todo、event、active_plan 可能不会自动删除，形成孤儿数据。

稳定性影响中等。多 writer 或 GUI/agent 并发访问 planning store 时，默认 rollback journal 和无 busy timeout 更容易暴露 `database is locked`。

性能影响中等。WAL 模式通常能改善读写并发，适合计划、事件、todo 这类频繁读写的小型本地数据库。

可维护性影响中等。schema 看起来声明了级联关系，但运行时没有保证，容易让业务代码错误依赖不存在的约束。

## 解决方案

推荐将 SQLite 连接池构造收敛到 store 内部，或提供一个受控构造器，确保每个连接创建后执行 PRAGMA。`sqlx::sqlite::SqliteConnectOptions` 支持通过 `pragma` 配置连接：

```rust
use sqlx::sqlite::{SqliteConnectOptions, SqliteJournalMode, SqlitePoolOptions};
use std::str::FromStr;

pub async fn connect(database_url: &str) -> Result<Self, PlanningError> {
    let options = SqliteConnectOptions::from_str(database_url)?
        .pragma("foreign_keys", "ON")
        .journal_mode(SqliteJournalMode::Wal)
        .pragma("busy_timeout", "5000");

    let pool = SqlitePoolOptions::new()
        .max_connections(5)
        .connect_with(options)
        .await?;

    Self::new(pool).await
}
```

如果必须继续保留 `new(pool)`，则至少在 `new()` 开始处执行连接级初始化，并在文档中说明调用方仍需保证池中新连接也启用 PRAGMA：

```rust
sqlx::query("PRAGMA foreign_keys = ON").execute(&pool).await?;
sqlx::query("PRAGMA journal_mode = WAL").execute(&pool).await?;
sqlx::query("PRAGMA busy_timeout = 5000").execute(&pool).await?;
```

更稳妥的做法是：

1. 新增 `SqlitePlanningStore::connect(path_or_url)`，生产代码统一使用该入口。
2. 将 `new(pool)` 改名为 `new_with_pool(pool)`，标注为测试或高级入口。
3. 增加外键级联测试：创建 plan、step、todo、event、active_plan，删除 plan 后验证子表为空。
4. 增加 PRAGMA 测试：查询 `PRAGMA foreign_keys` 返回 `1`，`PRAGMA journal_mode` 在文件数据库下返回 `wal`。

## 验证方法

推荐命令：

```powershell
cargo test -p agent-diva-core planning::store
cargo test -p agent-diva-agent planning
cargo clippy -p agent-diva-core -- -D warnings
```

新增测试预期：

1. `DELETE FROM plans WHERE id = ?` 后，`plan_steps`、`todo_items`、`planning_events`、`active_plan` 中对应记录自动删除。
2. `SELECT PRAGMA foreign_keys` 或 `PRAGMA foreign_keys` 查询结果为 `1`。
3. 文件数据库连接的 `PRAGMA journal_mode` 为 `wal`。
4. 并发写入测试不再因为短暂锁竞争立即失败，而是在 `busy_timeout` 内等待或明确超时。

## 优先级

P1
