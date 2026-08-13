# 验收步骤

1. **关闭 LLM（默认）**  
   - Notebook 触发日/周/月报成功。  
   - 打开报告：正文为中文结构化降级版（摘要/主题/下一步/数据口径），frontmatter 含 `generation_mode: deterministic_fallback`。  
   - 正文不逐条罗列 session id。

2. **开启 LLM**  
   - 配置 `reports.llm_curation.enabled=true` 与可用模型。  
   - 触发日报：成功时 `generation_mode: llm_curated`，GUI 显示「LLM 归纳」徽章；失败时降级且非空。

3. **兼容**  
   - 旧报告列表/预览/固化仍可用。  
   - 月报仍在 `reports/monthly/`，日/周仍在 `.agent-diva/autodream/reports/`。

4. **双实现**  
   - GUI 不再本地生成月报；仅触发 manager autodream。
