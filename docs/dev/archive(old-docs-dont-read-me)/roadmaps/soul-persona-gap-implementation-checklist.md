# Agent-Diva SOUL 人格能力补齐清单

> 目标：把当前 SOUL 基础能力补齐为更完整的人格塑造闭环（生成 -> 演化 -> 透明 -> 继承）。
> 范围：仅覆盖 `agent-diva` 当前代码结构，不引入破坏性重构。

## 1. 当前结论（基线）

- 已有能力：
  - 已支持 `SOUL.md`、`IDENTITY.md`、`USER.md`、`BOOTSTRAP.md` 注入 system prompt。
  - 已支持 bootstrap 生命周期状态持久化（`soul-state.json`）。
  - 已支持 soul 文件变更透明通知（统一提示）。
  - 已支持子代理继承部分人格（`SOUL.md` + `IDENTITY.md`）。
- 关键差距：
  - 主身份开头仍硬编码（不是完全文件驱动）。
  - Onboarding 仍以技术配置为主，人格引导强度不足。
  - 透明通知粒度较粗（未说明改了什么、为什么）。
  - 子代理未继承 `USER.md`，人格一致性不足。

---

## 2. 实施原则

- 渐进式上线：每个阶段可独立发布、独立回滚。
- 向后兼容：旧 workspace 缺少 soul 文件时必须有回退路径。
- 小步可验证：每阶段至少包含单元测试 + 最小烟测。
- 配置可控：涉及行为变化的能力尽量受配置项控制。

---

## 3. 分阶段执行清单

## Phase 1：Identity 文件优先（高优先级，低风险）

### 目标

- 将身份来源从“硬编码优先”改为“`IDENTITY.md` 优先，硬编码兜底”。

### 改动文件

- `agent-diva-agent/src/context.rs`

### 执行项

- [ ] 在 `build_system_prompt()` 中优先读取 `IDENTITY.md`。
- [ ] 若文件缺失或内容无效，回退到现有硬编码身份头。
- [ ] 保持现有 `soul_settings.enabled` 逻辑不变。
- [ ] 保证 `max_chars` 截断行为与现有逻辑一致。

### 测试项

- [ ] 有 `IDENTITY.md` 时，prompt 中身份内容来自文件。
- [ ] 无 `IDENTITY.md` 时，prompt 回退到默认硬编码身份。
- [ ] 空文件/超长文件场景可正常处理。

### 验收标准

- [ ] 新旧 workspace 都可正常运行。
- [ ] 不影响 skills/memory 注入顺序和内容。

---

## Phase 2：Bootstrap 人格引导升级（高优先级，中风险）

### 目标

- 把 bootstrap 从“问答清单”升级为“对话式人格塑造脚本”。

### 改动文件

- `agent-diva-core/src/utils/mod.rs`（默认模板）

### 执行项

- [ ] 升级 `DEFAULT_BOOTSTRAP_MD`：明确要求收集名字、语气、边界、禁区、协作偏好。
- [ ] 明确引导完成条件：写入/更新 `SOUL.md`、`IDENTITY.md`、`USER.md`。
- [ ] 明确完成动作：删除 `BOOTSTRAP.md` 或写入完成标记（保持与现有状态机制兼容）。

### 测试项

- [ ] 新 workspace 自动生成升级后的 bootstrap 模板。
- [ ] 模板文本具备可执行步骤，不与现有状态机制冲突。
- [ ] `sync_workspace_templates()` 仍保持幂等和不覆盖已有文件。

### 验收标准

- [ ] 首次对话具备可操作的人格初始化指令。
- [ ] bootstrap 只在需要时注入，不重复干扰后续会话。

---

## Phase 3：透明通知细化（中优先级，低风险）

### 目标

- 从“统一一句话通知”升级到“结构化透明通知”。

### 改动文件

- `agent-diva-agent/src/agent_loop.rs`

### 执行项

- [ ] 记录本轮被修改的 soul 文件集合（`SOUL.md`/`IDENTITY.md`/`USER.md`/`BOOTSTRAP.md`）。
- [ ] 结束回复时输出结构化提示（示例：改动文件列表 + 简短原因）。
- [ ] 保留 `notify_on_soul_change` 配置开关行为。

### 测试项

- [ ] 单文件更新时只提示该文件。
- [ ] 多文件更新时去重并完整列出。
- [ ] 工具失败或非 soul 文件更新时不提示。

### 验收标准

- [ ] 用户可读到“改了什么”，而不是泛化通知。
- [ ] 不增加额外模型调用。

---

## Phase 4：子代理人格继承补齐（中优先级，低风险）

### 目标

- 子代理继承 `USER.md`，提升人格与用户偏好一致性。

### 改动文件

- `agent-diva-agent/src/subagent.rs`

### 执行项

- [ ] `build_identity_summary()` 增加 `USER.md` 读取与拼接。
- [ ] 对 `USER.md` 应用和现有一致的截断策略。
- [ ] 兜底文案保持简洁，不泄露主会话内容。

### 测试项

- [ ] 存在 `USER.md` 时子代理 prompt 包含该节。
- [ ] 不存在 `USER.md` 时行为与当前一致。
- [ ] 超长文件时仍满足长度约束。

### 验收标准

- [ ] 子代理输出风格更接近主代理与用户偏好。
- [ ] 无明显 prompt 膨胀风险。

---

## Phase 5：人格演化治理（增强项，可延期）

### 目标

- 防止 SOUL 漂移失控，提升可控性与可解释性。

### 候选改动文件

- `agent-diva-agent/src/agent_loop.rs`
- `agent-diva-agent/src/context.rs`
- `agent-diva-core/src/config/schema.rs`（如需新增开关）

### 执行项

- [ ] 增加“高风险改动确认”策略（如边界类条目先提议再落盘）。
- [ ] 增加“短周期频繁改 soul 限流”策略。
- [ ] 增加审计痕迹（可选：轻量日志，不必引入重型存储）。

### 验收标准

- [ ] 人格可持续演化但不过度漂移。
- [ ] 用户可干预关键人格边界调整。

---

## 4. 验证清单（每阶段结束执行）

- [ ] `just fmt-check`
- [ ] `just check`
- [ ] `just test`
- [ ] 最小烟测（用户可见变更至少一项）：
  - [ ] `just run -- agent --message "用一句话介绍你自己"`（验证身份/风格是否符合文件驱动）
  - [ ] `just run -- agent --message "你这轮是否更新了 soul 文件"`（验证透明通知）

---

## 5. 风险与回滚

- 主要风险：
  - Prompt 膨胀导致回答质量波动。
  - 文件驱动身份与历史行为不一致造成“人格跳变”。
  - 透明提示过长影响回复可读性。
- 回滚策略：
  - 保留硬编码身份兜底路径（Phase 1 必须保留）。
  - 支持通过配置关闭 soul 注入（`agents.soul.enabled=false`）。
  - 支持关闭透明提示（`agents.soul.notify_on_change=false`）。

---

## 6. 推荐执行顺序

- 推荐先做：Phase 1 -> Phase 3 -> Phase 4（收益高、风险低）。
- 然后做：Phase 2（体验升级）。
- 最后做：Phase 5（治理增强，可分多次迭代）。

---

## 7. 完成定义（Definition of Done）

- [ ] 代码改动通过格式化、静态检查、测试。
- [ ] 用户可见行为变化有对应 smoke 记录。
- [ ] 文档与实现一致，未出现“文档说有但代码没有”的能力描述。
- [ ] 每个阶段均可单独发布与回滚。
