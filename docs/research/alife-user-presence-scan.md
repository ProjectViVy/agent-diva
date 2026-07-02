# alife 用户在场检测扫描

> 子代理 SA4 产出 · 2026-06-18
> 耗时:18 秒,2 次工具调用

## 结论

**确认 alife 没有 user presence / idle / 用户在场检测机制。**

## 证据

### 1. 全仓 grep(13 关键词)

```bash
grep -rln -iE "(presence|idle|afk|lastSeen|lastActive|userActive|windowFocus|windowDeactivat|isOnline|userOnline|GetLastInputInfo|userIdle)" sources --include='*.cs'
```

结果:**`total_count: 0`** —— `sources/` 目录下所有 .cs 文件零命中。

### 2. 6 个关键文件逐一核查

| 文件 | 结论 |
|------|------|
| `sources/Alife/Alife.Framework/Systems/CharacterSystem.cs` | 仅角色 CRUD(创建/删除/保存/加载),无任何用户在场相关字段或事件 |
| `sources/Alife/Alife.Framework/Models/ChatActivity.cs` | 仅模块容器 + SemanticKernel + ChatBot 组装,与用户是否在场无关 |
| `sources/Alife/Alife.Framework/Systems/ChatActivitySystem.cs` | Activate/Deactivate 是"**角色激活**"(character activation),不是用户在场;无 idle timer、无 lastSeen 字段 |
| `sources/Alife/Alife.Framework/Systems/ConfigurationSystem.cs` | 纯配置读写,无 presence 相关 |
| `sources/Alife/Alife.Client/MainWindow.xaml.cs` | 仅 35 行,只有最小化/最大化/关闭;**无 Window.Activated/Deactivated 订阅,无 GetLastInputInfo 调用** |
| `开发规范/代码设计指南.md` | 设计原则文档,未涉及在场检测 |

### 3. WPF 原生能力未利用

- `Window.Activated` / `Window.Deactivated` / `Application.SessionEnding` —— alife 主窗口**均未订阅**
- Win32 API `GetLastInputInfo` / `LASTINPUTINFO` —— **未被引用**

## 含义

**alife 的"自主"假设是"主人随时可能响应"**。AI 主动说话不会打扰,因为:
- 主人要么在等(希望桌宠主动)
- 要么不在(桌宠在桌面上,回来看到 AI 自言自语也不奇怪)

**diva 不一样** —— diva 是后台 gateway + GUI + IM 多通道,**用户可能在严肃工作**(写代码/开会/睡觉),AI 突然说话是真打扰。

## 给 diva 设计建议

**diva 必须自己加 user presence 层**,不能依赖 alife 借鉴。起点:

```rust
// agent-diva-core/src/presence/mod.rs (新)
pub struct UserPresenceTracker {
    last_user_message: Arc<RwLock<Instant>>,
    // 未来扩展:屏幕活动检测 / 锁屏检测 / Web 焦点
}

impl UserPresenceTracker {
    /// 任何 channel 收到 user 消息时调用
    pub fn on_user_message(&self) { *self.last_user_message.write().now_or_never() = Instant::now(); }
    
    /// Heartbeat 触发前调用
    pub fn should_heartbeat_fire(&self, threshold: Duration) -> bool {
        Instant::now() - *self.last_user_message.read().now_or_never() >= threshold
    }
}
```

**配置项**:`presence.idle_threshold = "30m"`(用户可设,默认 30 分钟)
**接入点**:`MessageBus` inbound 事件 → 调 `tracker.on_user_message()`
**触发拦截**:Heartbeat service `decide()` 之前查 `tracker.should_heartbeat_fire()`

## 待验证(本波不动)

- diva-pro 是否有现成的"用户最后活跃时间"概念?—— 需查 `agent-diva-core/src/bus/` 和 `agent-diva-channels/`
- Tauri v2 是否暴露窗口焦点事件?—— 需查 `agent-diva-gui/`
