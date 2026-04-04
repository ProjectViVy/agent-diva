# 验收标准

## Phase 3: Hook 系统 / 软删除 / Channel 文件

满足以下条件时，可认为本轮目标达成：

### 1. Hook 系统

- [ ] `StorageHook` trait 可在存储前后执行自定义逻辑
- [ ] `ReadHook` trait 可在读取前后执行自定义逻辑
- [ ] `MetadataHook` trait 可提取和验证元数据
- [ ] `CleanupHook` trait 可在清理时执行自定义逻辑
- [ ] `HookAction` 支持 Continue/Modify/Stop/Error 四种返回值
- [ ] `HookRegistry` 可注册和执行多种 Hook
- [ ] 内置 `LoggingStorageHook` / `LoggingReadHook` / `LoggingCleanupHook`
- [ ] Hook 执行顺序正确（before_* → 主逻辑 → after_*）

### 2. 软删除

- [ ] `soft_delete(id, deleted_by)` 标记文件为已删除
- [ ] `restore(id)` 可恢复已删除文件
- [ ] `list_deleted()` 列出所有已删除文件
- [ ] `hard_delete(id)` 绕过保留期直接删除
- [ ] `purge_expired(retention_days)` 自动清理过期文件
- [ ] `is_deleted(id)` 查询文件是否已删除
- [ ] 清理时自动跳过软删除文件

### 3. Channel 文件

- [ ] `upload_to_channel()` 上传文件到指定 Channel
- [ ] `add_file_to_channel()` 已有文件加入 Channel
- [ ] `list_channel_files()` 列出 Channel 中文件
- [ ] `remove_from_channel()` 从 Channel 移除
- [ ] `delete_channel()` 删除整个 Channel
- [ ] `list_file_channels()` 查看文件属于哪些 Channel
- [ ] `channel_stats()` 获取 Channel 统计信息
- [ ] 文件去重（相同内容共享存储）

### 4. 代码质量

- [ ] `cargo test` 30 个测试全部通过
- [ ] `cargo clippy -D warnings` 无警告
- [ ] `cargo fmt` 代码格式正确
- [ ] 所有公开 API 有文档注释

### 5. 文档

- [x] `README.md` 快速上手指南
- [x] `docs/LEARNING.md` 详细学习教程
- [x] `docs/debugging.md` 调试指南

---

## Phase 1 已验收功能

- [x] 内容寻址存储（SHA256）
- [x] 引用计数管理
- [x] 自动去重
- [x] 文件存储和读取
- [x] SQLite 持久化索引
- [x] 懒清理策略
- [x] 分渠道配置

## Phase 2 已验收功能

- [x] agent-diva-core 集成 FileAttachment
  - [x] `FileAttachment` 结构体实现
  - [x] `from_handle()` - 从 FileHandle 创建
  - [x] `from_metadata()` - 从元数据重建
  - [x] `is_image()`, `is_video()`, `is_audio()`, `is_document()` 辅助方法
  - [x] `display()` 显示格式
  - [x] 102 个单元测试全部通过
