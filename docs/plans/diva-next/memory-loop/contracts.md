# 共享测试接口与证据契约

版本：`diva.memory-verification/v1`。这是验证工具的计划契约，不修改产品 RPC/领域 DTO。产品字段和权限继续以 DN-W3/C2、Garden agentapi 为准。新测试文件和校验器尚未实现。

## 1. 基线与候选

S01 生成 `docs/logs/2026-10-memory-loop-verification/baseline.json`，必需字段：

- `schema` 固定 `diva.memory-baseline/v1`。
- `candidate_id`：同一源码、依赖、recipe、模型资源组合的 SHA-256 标识。
- `baselines[]`：每项包括 `baseline_id`、`candidate_id`、`repositories`（三个 SHA 与 dirty 状态）、`lock_sha256`、`recipe_sha256`、`generation_id`、`artifact_path`、`artifact_sha256`、`os`、`arch`、`go_version`、`wails_version`、`model_assets`（文件和哈希）、`test_root`。
- `config_template` 指向去敏测试配置，不包含密钥值；具体凭据由本地配置入口提供。

同 candidate 的 Linux/Windows 可有不同 binary hash、Generation 和 baseline_id，但源码输入必须可对应。不同比较基线分别保存；不能用原锁定版本的存储测试加新候选的 UI 测试拼接通过。

## 2. 已存在的产品入口

接手人按 S01 选定版本再次核对以下签名；签名变化先更新本计划的对应引用，不写兼容旁路。

```go
// VIVY sdk/host/v1 (DIVA 只能使用公共宿主边界)
func Open(ctx context.Context, options Options) (*Host, error)
func (h *Host) Call(ctx context.Context, method string, params json.RawMessage) (json.RawMessage, error)
func (h *Host) Next(ctx context.Context, limit int) (EventBatch, error)
func (h *Host) Close(ctx context.Context) error
// Options{ConfigPath: absolutePath, WithoutEars: true}

// Garden agentapi（绑定 authority，不从模型参数构造主体）
func Open(ctx context.Context, cfg Config) (*Client, error)
func (c *Client) BindHumanSession(sessionID, workspaceID string) (*HumanClient, error)
func (h *HumanClient) MutateMemory(ctx context.Context, m memory.AuthorizedMutation) (memory.MutationReceipt, error)
func (h *HumanClient) SearchMemory(ctx context.Context, q memory.AuthorizedSearch) (memory.CardPage, error)
func (h *HumanClient) ExpandMemory(ctx context.Context, q memory.AuthorizedExpansion) (memory.EvidencePage, error)
func (h *HumanClient) MemoryReceipt(ctx context.Context, operationID string) (memory.MutationReceipt, error)
```

实际 UI 通过 `module.action.invoke` 调用 `diva.cognitive.*` 动作。HumanClient/这些控制动作的可用性不自动证明 Agent 的召回能力。ContextHost provider 或显式工具必须在 S08 的真实模型输入中证明有效。

## 3. S03 提供的测试夹具接口

位置：VIVY `internal/app/memory_loop_fixture_test.go`，package `app`。以下全部是拟新增 test-only 接口，后续 Story 使用同一组名称，不另造一套 helper。

```go
type memoryLoopOptions struct {
    ConfigPath string // 绝对路径；所有状态均处于独立测试根
    ModelMode  string // ack | reflection | recall；仅依据收到的请求作答
}

type memoryLoopSnapshot struct {
    RunID            string
    EventSeq         uint64
    IngestionID      string
    CaptureSeq       uint64
    ProcessedThrough uint64
    OperationID      string
    RecordID         string
    Revision         uint64
    CanonicalCount   int
    SourceBody       string // 仅合成测试数据；来自持久来源
    SourceRole       string
    SourceHash       string
    State            string // 真实公开/持久状态原样投影
}

func newMemoryLoopFixture(t *testing.T, opts memoryLoopOptions) *memoryLoopFixture
func (f *memoryLoopFixture) Call(ctx context.Context, method string, params json.RawMessage) (json.RawMessage, error)
func (f *memoryLoopFixture) Wait(ctx context.Context, stage, runID string) (memoryLoopSnapshot, error)
func (f *memoryLoopFixture) ModelRequests() []json.RawMessage
func (f *memoryLoopFixture) Restart(ctx context.Context) error
func (f *memoryLoopFixture) Close(ctx context.Context) error
```

构造器必须启动真实 `App.New`/生成 Assembly、`DialControl`、持久 backend、ObserverHost 和 cognition，不复制 `cognitiveOriginFixture` 中的空 Service。若默认 Generation 与 DIVA 不同，用仓库支持的编译/pack 路径生成测试组合，并将组合差异记录为 integration fixture；密封 DIVA 产品仍由 S12 证明，不能伪造 manifest。

`Wait`仅支持 `terminal`、`accepted`、`canonical`、`reflected` 四个测试阶段。读取实际数据后按调用方断言，阶段不可观测则报错；不依据固定 sleep 假设阶段已完成。字段不适用于某阶段时使用零值并在证据说明，不能伪造 ingestion/operation。

`Restart`必须重新启动运行宿主进程并保持同一独立数据根；S03 可以先完成同进程 close/open 冒烟，但不能把该冒烟替代 S08/S10 所需的跨进程结果。S10 可为夹具增加 `Crash(ctx context.Context, cutPoint string) error`，只允许以下 C01–C06。

模型模式：

- `ack`：普通回合只答“收到”。
- `reflection`：从实际模型请求中的 stage/input 提取事实，按策略输出 schema 返回有效内容；不得从测试断言变量获取答案。
- `recall`：仅解析实际收到的获准记忆证据作答，没有证据时答“未知”。禁止通过闭包预埋随机事实。

超时默认：本地 RPC 10 秒、确定性阶段等待 60 秒、正常关闭 5 秒、真实模型单次上限 180 秒。超时属于失败/阻塞，不是 no-change；这些是测试截止时间，不是产品性能 SLA。测试 runner 必须可取消，长任务期间持续输出可读进度。

## 4. 断言范式

以下是测试设计的最小断言，不是已实现代码。每个 Story 的测试表补充具体阶段和否定条件。

```go
// S04: fact 由测试器随机生成，仅通过真实 user 消息提交。
if !strings.Contains(snapshot.SourceBody, fact) || snapshot.SourceRole != "user" {
    t.Fatal("user-only fact lost or misattributed in durable source")
}
if replay.RecordID != first.RecordID || replay.CanonicalCount != first.CanonicalCount {
    t.Fatal("redelivery duplicated canonical effect")
}
// S08: input 是实际 provider 请求内提取的授权记忆片段。
if !strings.Contains(input, fact) || strings.Contains(emptyProfileInput, fact) {
    t.Fatal("recall causal control failed")
}
// S10: 对未知效果的特定窗口，不允许水位越过未确定项。
if after.CanonicalCount > expectedCount || after.ProcessedThrough > knownSafeThrough {
    t.Fatal("effect replay or premature watermark advancement")
}
```

终态/source 的真实字段不能提供 `SourceRole` 时应暴露“角色信息缺失”缺陷，不能在测试观察器中默认填 `user` 让测试通过。

## 5. 证据记录与校验器

每个 Story 写 `docs/logs/2026-10-memory-loop-verification/Sxx/evidence.json`：

```json
{
  "schema": "diva.memory-evidence/v1",
  "story_id": "S04",
  "candidate_id": "actual-candidate-hash",
  "baseline_id": "actual-platform-baseline-id",
  "status": "blocked",
  "case_results": [],
  "commands": [],
  "artifacts": [],
  "defects": [],
  "reason": "example structure only; not execution evidence"
}
```

上面示例不是运行结果，禁止原样提交为通过。`status`取 `pass|fail|blocked`。每个 `case_results` 元素必须有：`case_id`、`sample_id`、`mode`、`status`、`profile_id`、`scope`、`session_a`、`session_b`、`run_id`、`event_seq`、`ingestion_id`、`capture_seq`、`processed_through`、`operation_id`、`record_id`、`revision`、`source_hash`、`prompt_hash`、`answer_check`、`timestamps`、`artifact_paths`、`failure_reason`。不适用项用 null，并在 `not_applicable` 中逐项说明。

- `mode`：`scripted-real-storage|native-ui|live-model|infrastructure`。
- `commands`：每项记录 cwd、去敏 command、exit_code、tests_discovered、tests_passed、tests_skipped、log_path；人工场景注明 manual，不伪造测试数量。
- `sample_id`区分三 profile 重复、六切点 × 十次和三十次真实模型样本。
- artifact 路径相对于证据根，拒绝绝对路径、`..`、逃逸符号链接；文件存在并校验 hash。截图不代替来源与请求证据。
- S01 用 V01；S03 使用 `INFRA-FIXTURE` 和 `INFRA-CHECKER` 自验收，不增加 28 类产品场景分母。
- 最终报告验证各 Story required case，尤其 V22 要求 C01–C06 各 10 个样本，V28 要求 10 类各 3 个样本，S12 至少 3 个独立 profile 正向链路。

S03 新建 Python 校验器接口：

```python
def validate_evidence(record: dict, evidence_root: Path) -> list[str]:
    """返回违反上述契约的错误列表；空列表表示结构和证据引用有效。"""
```

CLI：`python3 scripts/ci/check_memory_loop_evidence.py --story S04 --evidence-root docs/logs/2026-10-memory-loop-verification`。`--story ALL` 汇总全包。读取 plan 的 package-map.json 确认 Story 和 case 关系；退出 0 要求证据有效且相关 required case 全 pass，退出 1 为失败/阻塞/证据不完整，退出 2 为输入格式错误。不能因为 JSON 合法就宣布行为通过；行为断言来自实际测试与人工评分记录。

## 6. 六个崩溃切点

| ID | 注入位置 | 恢复断言 |
|---|---|---|
| C01 | 终态已 durable、Capture 尚未接收 | observer 重投同 event，最终只有一份源 |
| C02 | Capture 接收落盘、observer 尚未 ACK | 原 receipt/seq 复用，不丢或重复源 |
| C03 | canonical effect 已提交、调用方回执未确认 | atomic receipt 可查则复用；否则 unknown 阻断盲重试 |
| C04 | 多 effect 中部分已成功 | 保留已知效果，未知阻断后续，不能整体重放 |
| C05 | 效果均完成、水位尚未持久推进 | 恢复核对回执后完成窗口，不再次执行效果 |
| C06 | canonical 已提交、派生索引尚未完成 | canonical 保持权威，索引恢复，不写重一份 memory |

每次必须由已达位置的握手触发强制终止，用新进程读取原数据根。日志保留 PID、切点、前后 snapshot；产品正常构建不得暴露 failpoint。

## 7. 缺陷与跨仓库变更交接

`MEM-Sxx-nn`记录：首个断点、复现命令、预期/实际、根因、最小文件范围、关联 case、修复 commit 和重测证据。多个仓库的提交要有同一个 candidate 记录。已知缺陷修复不靠大范围重构；共享接口变化更新本文件及所有直接消费者后继续。
