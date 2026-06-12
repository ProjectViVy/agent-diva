# P1-4: ChannelManager 持锁跨 await

## 问题描述

`agent-diva-channels/src/manager.rs` 的 `ChannelManager` 使用 `RwLock<HashMap<String, ChannelHandlerPtr>>` 保存所有 channel handler：

```rust
handlers: RwLock<HashMap<String, ChannelHandlerPtr>>,
```

多个方法在持有 `handlers` 的读锁或写锁时继续 await handler 操作。

`start_all` 持有 `handlers.read().await`，循环中调用 `Self::start_handler(...).await`；`start_handler` 内部再持有单个 handler 的写锁并 await `handler.start()`：

```rust
let handlers = self.handlers.read().await;
for (name, handler) in handlers.iter() {
    if let Err(e) = Self::start_handler(name, handler).await {
        // ...
    }
}
```

`stop_all` 持有 `handlers.write().await`，循环中 await `handler.stop()`：

```rust
let mut handlers = self.handlers.write().await;
for (name, handler) in handlers.iter_mut() {
    let mut handler = handler.write().await;
    if let Err(e) = handler.stop().await {
        // ...
    }
}
handlers.clear();
```

`send` 持有 `handlers.read().await`，再 await `handler.send(message)`：

```rust
let handlers = self.handlers.read().await;
let handler = handlers.get(channel).ok_or_else(...)?;
let handler = handler.read().await;
handler.send(message).await
```

`update_channel` 持有 `handlers.write().await`，期间 stop 旧 handler、设置 inbound sender、start 新 handler，然后才 insert：

```rust
let mut handlers = self.handlers.write().await;
if let Some(handler) = handlers.get(name) {
    let mut handler = handler.write().await;
    if let Err(e) = handler.stop().await { /* ... */ }
}
// ...
Self::start_handler(name, &handler).await?;
handlers.insert(name.to_string(), handler);
```

这些路径把全局 channel map 锁的生命周期扩大到网络连接、远端 API 调用、轮询启动/停止等不可控异步操作。

## 影响评估

- 性能影响：某个慢 channel 的 start/stop/send 会阻塞其他 channel 的查询、发送和配置更新。
- 可用性影响：`update_channel` 持有写锁期间，所有读锁请求都会等待，可能导致 GUI 或控制面接口卡顿。
- 死锁风险：handler 的 `start/stop/send` 若回调到 manager 或等待依赖 manager 状态，容易形成锁顺序反转。
- 故障扩散：单个 Telegram/Discord/Email handler 的网络超时会拖住整个 ChannelManager。

## 解决方案

核心原则：只在锁内查找、克隆或替换 `Arc<RwLock<dyn ChannelHandler>>`，不要在持有全局 `handlers` 锁时 await 外部操作。

示例拆分 `send`：

```rust
pub async fn send(&self, channel: &str, message: OutboundMessage) -> Result<()> {
    let handler = {
        let handlers = self.handlers.read().await;
        handlers
            .get(channel)
            .cloned()
            .ok_or_else(|| ChannelError::NotConfigured(format!("Channel {} not found", channel)))?
    };

    let handler = handler.read().await;
    handler.send(message).await
}
```

示例拆分 `start_all`：

```rust
pub async fn start_all(&self) -> Result<()> {
    let handlers: Vec<(String, ChannelHandlerPtr)> = {
        let handlers = self.handlers.read().await;
        handlers
            .iter()
            .map(|(name, handler)| (name.clone(), handler.clone()))
            .collect()
    };

    for (name, handler) in handlers {
        if let Err(e) = Self::start_handler(&name, &handler).await {
            // collect failure
        }
    }
    Ok(())
}
```

`update_channel` 建议分阶段：

1. 在锁内取出旧 handler 并从 map 移除。
2. 释放 map 锁后 stop 旧 handler。
3. 构造并 start 新 handler。
4. 重新获取 map 写锁 insert 新 handler。

需要注意回滚语义：如果新 handler start 失败，是否恢复旧 handler 应明确定义并测试。

## 验证方法

执行：

```powershell
cargo test -p agent-diva-channels channel_manager
cargo test -p agent-diva-channels
just fmt-check
just check
```

建议新增异步测试：

- 一个 fake handler 的 `send` 挂起时，`list_channels` 不应被阻塞。
- 一个 fake handler 的 `stop` 挂起时，对其他 channel 的 `get_handler` 或 `send` 不应等待全局写锁。
- `update_channel` 新 handler 启动失败时，返回错误且 map 状态符合预期。
- 用 `tokio::time::timeout` 包裹并发场景，防止测试永久挂起。

## 优先级

P1
