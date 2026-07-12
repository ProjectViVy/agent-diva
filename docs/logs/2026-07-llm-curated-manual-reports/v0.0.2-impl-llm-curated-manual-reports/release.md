# 发布说明

本迭代为运行时功能交付，不单独打版本号（workspace 仍为 `0.5.0`）。

## 配置

```json
{
  "reports": {
    "llm_curation": {
      "enabled": false,
      "provider": null,
      "model": null,
      "language": "zh-CN",
      "max_input_tokens": 8000,
      "max_output_tokens": 1500,
      "timeout_secs": 60,
      "fallback": "deterministic"
    }
  }
}
```

启用 LLM 归纳：将 `enabled` 设为 `true`，可选覆盖 `provider`/`model`（否则继承 `agents.defaults`）。native endpoint 使用原始 model id。

## 部署

与常规 workspace 发布相同：构建/发布 `agent-diva-cli` 与 GUI 即可；无需迁移脚本。旧报告缺少新 frontmatter 字段时 UI 不显示归纳标签。
