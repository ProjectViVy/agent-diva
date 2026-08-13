# P1-7: S3 backend 6 个 todo!()

## 问题描述

`agent-diva-files/src/backend.rs` 中的 S3 后端仍是占位实现。文件中定义了 `S3StorageBackend` 和 `StorageBackend` impl，但整段目前被块注释包裹，并包含 6 个 `todo!()`：

```rust
async fn initialize(&self) -> Result<()> {
    todo!("Implement S3 initialization")
}

async fn write(&self, key: &str, data: &[u8]) -> Result<PathBuf> {
    let path = PathBuf::from(format!("{}/{}", self.prefix, key));
    todo!("Implement S3 upload")
}

async fn read(&self, path: &Path) -> Result<Vec<u8>> {
    todo!("Implement S3 download")
}

async fn delete(&self, path: &Path) -> Result<()> {
    todo!("Implement S3 delete")
}

async fn exists(&self, key: &str) -> bool {
    todo!("Implement S3 exists check")
}

async fn stats(&self) -> Result<BackendStats> {
    todo!("Implement S3 stats")
}
```

虽然当前代码块被注释，不会直接编译进产物，但它已经以“future implementation”的形式出现在主分支。后续一旦解除注释或接入配置，任何调用都会在运行时 panic。

## 影响评估

- 稳定性影响：`todo!()` 在 Rust 中会 panic，S3 后端一旦被启用会造成请求失败甚至任务崩溃。
- 可维护性影响：占位代码看起来接近完整实现，容易被误认为只是缺少配置 wiring。
- 产品风险：文件系统抽象声明支持 pluggable backend，但远端对象存储路径没有可验证实现。
- 测试风险：现有测试只覆盖 `LocalStorageBackend`，无法防止 S3 占位代码被误启用。

## 解决方案

短期修复：不要保留会 panic 的占位实现。若 S3 尚不支持，应改为显式错误并确保不会编译出半成品后端。

示例：

```rust
#[async_trait]
impl StorageBackend for S3StorageBackend {
    async fn initialize(&self) -> Result<()> {
        Err(FileError::UnsupportedBackend(
            "S3 backend is not implemented".to_string(),
        ))
    }

    async fn write(&self, _key: &str, _data: &[u8]) -> Result<PathBuf> {
        Err(FileError::UnsupportedBackend(
            "S3 backend is not implemented".to_string(),
        ))
    }
}
```

中期修复：完整实现 S3-compatible backend：

- `initialize`：使用 `head_bucket` 验证 bucket 存在和权限。
- `write`：`put_object` 上传，返回规范化 `s3://bucket/prefix/key`。
- `read`：解析 S3 URI 或相对 key，`get_object` 读取 bytes。
- `delete`：`delete_object` 并处理 idempotent 删除。
- `exists`：`head_object` 返回 true/false，不吞掉权限错误。
- `stats`：对象数量和总大小需要分页 `list_objects_v2`，注意大 bucket 成本。

建议用 feature gate 隔离：

```rust
#[cfg(feature = "s3")]
pub struct S3StorageBackend { /* ... */ }
```

并在未启用 feature 或未实现时返回配置错误，而不是 panic。

## 验证方法

执行：

```powershell
rg -n "todo!\\(" agent-diva-files/src/backend.rs
cargo test -p agent-diva-files
just fmt-check
just check
```

预期结果：

- `agent-diva-files/src/backend.rs` 中不再存在 `todo!()`。
- 未启用 S3 时，选择 S3 backend 返回明确的 unsupported/config error。
- 启用 S3 feature 时，使用 mock S3 或 trait fake 覆盖 initialize/write/read/delete/exists/stats。
- 所有错误路径返回 `Result::Err`，不发生 panic。

## 优先级

P1
