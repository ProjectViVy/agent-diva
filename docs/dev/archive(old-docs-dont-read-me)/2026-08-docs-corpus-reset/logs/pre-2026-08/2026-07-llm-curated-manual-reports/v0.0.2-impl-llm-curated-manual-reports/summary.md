# 迭代总结

实现手动日报、周报、月报的 LLM 归纳改造（P1–P3 + 基础回归）。

## 变更

- **core**：`reports` 模块拆分为事实包、叙事契约、校验、确定性渲染；配置新增 `reports.llm_curation`（默认 `enabled=false`）。
- **providers**：`LlmReportNarrativeGenerator`（无工具 chat、JSON 解析、evidence 校验、超时、raw model id 透传）。
- **autodream**：日/周/月生成统一走 `ReportFactBundle` → 可选 LLM → fallback；frontmatter 增加 `generation_mode` 等；月报补 Evidence References。
- **manager**：注入 curation generator；报告触发/定时月报改为 async。
- **gui**：删除重复月报生产逻辑；DTO/Notebook 展示 LLM 归纳或确定性降级。

## 影响范围

`agent-diva-core`、`agent-diva-providers`、`agent-diva-autodream`、`agent-diva-manager`、`agent-diva-gui`。

计划文档：`docs/plan/llm-curated-manual-reports.md`。
