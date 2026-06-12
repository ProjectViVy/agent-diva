# Archive Governance — PENDING DECISIONS

> 生成日期：2026-06-13
> 范围：从 archive 中提取的待拍板事项
> 规则：每个条目必须包含「现状」「选项」「建议」「阻塞点」

---

## 一、架构方向（高优先级）

### PD-01: EvoMap / GEP 原生支持

- **现状**：调研已完成（2026-06-07），EvoMap CLI 已闭源，MCP 桥接包 Apache-2.0 可用
- **选项**：
  - A. 接入 `@evomap/gep-mcp-server` v1.7.0 作为 MCP 桥（只读）
  - B. 内部实现 gep-a2a 协议（深度集成）
  - C. 完全不接入，专注自研 skill 进化
- **建议**：选 A（MCP 桥 read-only），Hub 注册 opt-in
- **阻塞点**：需确认是否注册 Hub（涉及 node_secret 持久化）
- **相关原始文档**：`morediva-root/old-docs/MOREDIVA-EvoMap-调研与方向.md`

### PD-02: 自研 Skill 进化系统（周报提取）

- **现状**：方向已定（"从实践中来"路线），但无具体实现计划
- **选项**：
  - A. 在 `agent-diva-selfinprove` 下开新子 crate
  - B. 复用现有 `agent-diva-tooling` 模式
  - C. 作为外部 cron 任务，不内嵌
- **建议**：选 A，独立 crate 便于迭代
- **阻塞点**：需确认"周报"指 LLM 自动生成还是人工撰写
- **相关原始文档**：`morediva-root/old-docs/MOREDIVA-EvoMap-调研与方向.md` §3.2

### PD-03: 上下文压缩管线实现

- **现状**：Phase 1.1 已规划，未开工
- **选项**：
  - A. 3 层管线（snip / tool_budget / auto）
  - B. 4 层管线（参考 Claude Code：snip/micro/budget/auto）
  - C. 先只做 L1 snip（最低成本）
- **建议**：选 C 先落地，再迭代到 A
- **阻塞点**：需确认 token 估算用粗估还是精确 tokenizer
- **相关原始文档**：`agent-diva-main/docs/dev/archive/awesomeagents/evolution-roadmap.md` §1.1

---

## 二、安全修复（P0 级）

### PD-04: 子 Agent 安全三件套

- **现状**：decisions.md P0-1 已定，沙箱分支部分覆盖
- **待确认**：
  - 工具黑名单具体列表（delegate_task / clarify / memory / send_message / execute_code？）
  - MAX_DEPTH 默认值（1、2 还是 3？）
  - 角色模板是否现在就做（explorer/worker/reviewer）
- **建议**：先实现黑名单 + depth=1，角色模板放 P1
- **阻塞点**：沙箱分支已有深度控制，需确认是否复用
- **相关原始文档**：`agent-diva-main/docs/dev/archive/awesomeagents/decisions.md` §二、P0-1

### PD-05: 记忆写入安全扫描

- **现状**：P0-5b 已定，完全无实现
- **选项**：
  - A. 正则扫描常见注入模式（"忽略所有指令"、"发送到外部"等）
  - B. LLM 双重检测（成本高但覆盖更广）
  - C. A + B 混合（正则先过滤，可疑的再送 LLM）
- **建议**：选 A 先落地，成本可控
- **阻塞点**：需定义威胁模式清单
- **相关原始文档**：`agent-diva-main/docs/dev/archive/awesomeagents/decisions.md` §二、P0-5b

---

## 三、GUI / 产品（与当前主架构相关）

### PD-06: 沙箱审批 UI 实现

- **现状**：pro 分支有权限选择器 UI，但前后端断连
- **选项**：
  - A. CLI 单行交互（`[y/n/session]?`）
  - B. GUI 独立设置页（TOML 表单化）
  - C. 先做 A，B 放后续
- **建议**：选 C
- **阻塞点**：需确认 sandbox 分支的审批管线 API 是否稳定
- **相关原始文档**：`agent-diva-main/docs/dev/archive/awesomeagents/decisions.md` §九、9.2

### PD-07: 上下文健康状态行

- **现状**：后端 TokenBudget 完善，聊天界面零指示
- **选项**：
  - A. CLI 状态行 `[ctx 67% | 87K/130K]`
  - B. GUI 进度条/指示器
  - C. 两者都做
- **建议**：选 A 先做，B 等 GUI 设计稳定
- **阻塞点**：无
- **相关原始文档**：`agent-diva-main/docs/dev/archive/awesomeagents/decisions.md` §九、9.2

---

## 四、记忆系统（与当前主架构相关）

### PD-08: 分层记忆架构

- **现状**：扁平 MEMORY.md，已识别为缺陷 #16
- **选项**：
  - A. GenericAgent 式 L1-L4（索引/事实/记录/归档）
  - B. openfang 式 5 存储（结构化/语义/知识图谱/会话/用量）
  - C. 先只做 L1 索引层（≤30 行存在性编码）
- **建议**：选 C，最小可行
- **阻塞点**：需确认是否引入 BM25（memtle 方案）还是简单关键词匹配
- **相关原始文档**：`agent-diva-main/docs/dev/archive/awesomeagents/unknown-deficits.md` #16

---

## 五、多模态（与当前主架构相关）

### PD-09: 图像识别 Phase 2

- **现状**：main 分支有图片下载+base64 编码，但无结构化 `image_url` 内容块
- **选项**：
  - A. 构造标准 `image_url` 内容块（OpenAI 格式）
  - B. 继续用文本标记 `[IMAGE:data:...]`
  - C. 支持 provider 特定格式（Claude 的 image block 等）
- **建议**：选 A，标准化
- **阻塞点**：需确认各 provider 对 image_url 的支持情况
- **相关原始文档**：`agent-diva-main/docs/dev/archive/multimodal/image-recognition-prephase-analysis-plan.md`

---

## 六、Mentle 集成（进行中）

### PD-10: Mentle S7 范围

- **现状**：S6 已完成，S7 进行中（tool selection + GUI controls）
- **待确认**：
  - S7 是否继续在当前 pro 分支做？
  - GUI controls 是否涉及前端改动？
  - 完成后是否还有 S8？
- **建议**：S7 在 pro 分支完成，完成后评估是否继续
- **阻塞点**：需确认 S7 具体功能清单
- **相关原始文档**：`mentle-integration/25-s7-a1-mentle-tool-selection-and-gui-controls.md`

---

## 七、Nano / 打包

### PD-11: Nano 运行时打包策略

- **现状**：已外化为独立仓库，但打包策略未最终确定
- **选项**：
  - A. crates.io 发布（workspace 链：core → ... → manager → cli）
  - B. GitHub Release 二进制分发
  - C. 两者并行
- **建议**：选 C
- **阻塞点**：需确认 crates.io 发布权限和 CI 配置
- **相关原始文档**：`nano/crates-io-publish-strategy.md`、`nano-runtime-packaging-plan.md`

---

## 八、Diva Pet（桌面宠物）

### PD-12: Diva Pet 3D 背景集成

- **现状**：有完整 13 篇设计文档，但未开工
- **选项**：
  - A. 集成 VRM 模型 + 3D 背景（完整方案）
  - B. 先做 2D 表情系统（低成本验证）
  - C. 暂停，等资源充足
- **建议**：选 B 或 C，当前资源应优先保证核心 agent 功能
- **阻塞点**：需确认是否有 3D 美术资源
- **相关原始文档**：`diva-pet-3d-background/` 下 13 篇文档

---

## 九、Hermes 自学习集成（冻结中）

### PD-13: Hermes 学习融合 go/no-go

- **现状**：6 篇规划文档（2026-04-05），零实现，13-18 周 roadmap
- **选项**：
  - A. 继续推进（按 13-18 周 roadmap）
  - B. 冻结，等 EvoMap 方向确定后再评估
  - C. 取消，专注自研 skill 进化
- **建议**：选 B（冻结），与 PD-01/02 联动决策
- **阻塞点**：大湿拍板
- **相关原始文档**：`hermes-learning/` 下 4 篇文档

---

## 十、Observability

### PD-14: Thin Observability Layer

- **现状**：Phase B 文档存在，未实施
- **选项**：
  - A. 集成 Langfuse（参考 Claude Code）
  - B. 自研轻量层（SQLite + 简单 dashboard）
  - C. 复用现有 tracing + 结构化日志
- **建议**：选 C 先满足基本需求，A/B 后续评估
- **阻塞点**：无
- **相关原始文档**：`Observability/phase-b-thin-observability-layer.md`

---

## 决策记录表

| 编号 | 事项 | 优先级 | 状态 | 阻塞点 |
|------|------|--------|------|--------|
| PD-01 | EvoMap GEP 接入 | 高 | 待拍板 | Hub 注册决策 |
| PD-02 | 自研 Skill 进化 | 高 | 待拍板 | 周报定义 |
| PD-03 | 上下文压缩管线 | 高 | 待拍板 | Token 估算策略 |
| PD-04 | 子 Agent 安全三件套 | P0 | 待细化 | 黑名单清单 |
| PD-05 | 记忆写入安全扫描 | P0 | 待细化 | 威胁模式清单 |
| PD-06 | 沙箱审批 UI | P0 | 待拍板 | Sandbox API 稳定性 |
| PD-07 | 上下文健康状态行 | P1 | 待拍板 | 无 |
| PD-08 | 分层记忆架构 | P1 | 待拍板 | 检索方案 |
| PD-09 | 图像识别 Phase 2 | P1 | 待拍板 | Provider 兼容性 |
| PD-10 | Mentle S7 范围 | 中 | 待确认 | 功能清单 |
| PD-11 | Nano 打包策略 | 中 | 待拍板 | CI/发布权限 |
| PD-12 | Diva Pet 集成 | 低 | 待拍板 | 美术资源 |
| PD-13 | Hermes 学习融合 | 低 | 冻结 | 大湿决策 |
| PD-14 | Observability | 低 | 待拍板 | 无 |
