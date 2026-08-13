# Hermes cronjob tool —— EWait/EWake 借鉴原型

> 子代理产出 · 2026-06-18
> 任务来源: 大湿要求调研 hermes 有无 EWait/EWake 对应工具

## 核心发现

**Hermes 没有 EWait/EWake 这两个独立 tool,而是有一个更统一的 `cronjob` tool,把延迟、绝对时间、周期任务三合一。**

## 工具签名

```python
# ~/.hermes/hermes-agent/tools/cronjob_tools.py:465
def cronjob(
    action: str,           # create / list / pause / resume / remove / run / update
    schedule: Optional[str] = None,  # 关键参数:支持 3 态格式
    prompt: Optional[str] = None,    # 触发后注入 AI context 的内容
    job_id: Optional[str] = None,
    name: Optional[str] = None,
    deliver: Optional[str] = None,   # 推回 channel
    ...
)
```

**注册**:`tools/cronjob_tools.py:868` `registry.register(name="cronjob", toolset="cronjob", ...)`

## Schedule 三态格式

| 格式 | 含义 | 对应 alife |
|------|------|-----------|
| `"30m"` / `"2h"` / `"1d"` | 一次性延迟(从现在起) | `EWait(seconds)` |
| `"2026-02-03T14:00"` | 一次性绝对时间 | `EWake(iso8601, remark)` |
| `"every 30m"` / `"0 9 * * *"` | 周期 / cron | diva 现有 `agent-diva-core/src/cron/` |

**parse 逻辑**:`cron/jobs.py:206` 的 `parse_schedule()`,统一解析三种格式。

**duration parser**:`cron/jobs.py:185` `parse_duration()`,支持 m/h/d 后缀。

## Deliver 渠道(关键借鉴点)

`deliver` 参数支持把任务结果推回不同 channel:
- `local` —— 推到本地会话
- `telegram` / `discord` / `feishu` / `slack` / `dingtalk` / `webchat` —— 推 IM
- (见 `cron/scheduler.py` 的 `cron_delivery_targets` 体系)

**含义**:AI 不只是"被叫醒",还能选择"被叫醒时结果送到哪"。对 diva 来说 = 自主行为的多 channel 出口。

## CLI 配套

`hermes_cli/cron.py:60+` 提供完整 CLI:

```bash
hermes cron list              # 列出所有任务
hermes cron create            # 创建(交互式 prompt)
hermes cron edit <id>         # 编辑
hermes cron pause <id>        # 暂停
hermes cron resume <id>       # 恢复
hermes cron remove <id>       # 删除
hermes cron run <id>          # 立即跑一次
hermes cron tick              # 手动 tick scheduler
hermes cron status            # 状态总览
```

## diva 借鉴方案

**目标**: 给 diva 加 `cronjob` tool,封装现有 `agent-diva-core/src/cron/` 子系统。

### 文件结构

```
agent-diva-pro/
├── agent-diva-tools/
│   └── src/
│       └── cronjob.rs       (新) AI 可调的 self-schedule tool
└── agent-diva-cli/
    └── src/
        └── commands/
            └── cron.rs      (新) diva cron 子命令
```

### cronjob.rs 关键 API

```rust
#[tool(name = "cronjob", description = "...")]
pub async fn cronjob(
    action: CronAction,        // Create / List / Pause / Resume / Remove / Run / Update
    schedule: Option<String>,  // "30m" / "2026-06-18T20:00" / "0 9 * * *"
    prompt: Option<String>,
    deliver: Option<String>,   // channel id
    job_id: Option<String>,
    name: Option<String>,
) -> Result<CronJobResult> {
    // 1. parse schedule(复用 cron/parser 已有能力)
    // 2. 调 agent-diva-core::cron::Scheduler
    // 3. deliver 通过 channel manager 推送
}
```

### Schedule parser 复用

```rust
// agent-diva-core/src/cron/parser.rs (新,或扩充现有)
pub enum ParsedSchedule {
    OnceAfter(Duration),     // "30m"
    OnceAt(NaiveDateTime),    // ISO8601
    Cron(String),             // cron 表达式
}

pub fn parse_schedule(s: &str) -> Result<ParsedSchedule, ParseError>;
```

### Deliver 实现

复用 `agent-diva-channels/src/manager.rs`,deliver 字段直接是 channel id(local / telegram / feishu ...)。

### 跟 diva 现有 cron 子系统集成

diva 已经有 `agent-diva-core/src/cron/`(1275 LOC,系统级 cron),**cronjob tool 直接调用它的内部 API**,不重造。

## 跟 alife EWait/EWake 对比

| | alife(EWait/EWake) | hermes(cronjob) | diva 借鉴 |
|--|---------------------|------------------|----------|
| API 数 | 2 个 | 1 个(统一) | 选 hermes(更紧凑) |
| Schedule 格式 | 秒数 / DateTime | 字符串(3 态) | 字符串(更可读) |
| Deliver | 无(只 Poke 到 AI) | 多 channel | 多 channel(diva 强项) |
| 周期任务 | ❌ | ✅ | ✅ |
| CLI 管理 | 无 | ✅ | ✅ |
| 持久化 | 进程内存(重启丢) | 持久(具体存储未知) | Laputa 持久化 |

**diva 借鉴 hermes 的版本**,并加 diva 特色:
- Laputa 持久化(替换 hermes 的未知存储)
- Authority Spine 集成(cronjob 增删走提案)
- Channel manager 复用(已有)
- 当成 harness engineering W2 的子任务
