# 配置与依赖

## 1. 配置草案

```json
{
  "memory": {
    "authority_mode": "typed"
  },
  "autodream": {
    "enabled": true,
    "auto_trigger": false,
    "max_evidence_items": 64,
    "max_input_tokens": 12000,
    "max_candidates": 8,
    "daily_token_budget": 50000,
    "suppression_days": 30
  }
}
```

具体默认值需在 E0 characterization 后冻结。未知 mode、负数预算、越界容量均拒绝启动相应功能，不静默修正。

## 2. Provider

Reflection 通过现有 provider resolver 选择模型，不新增独立密钥文件。native provider 必须保留原始 model ID，不能自动加 gateway prefix。

## 3. 依赖原则

- 优先零新增依赖。
- 新 crate 必须支持 Rust 1.80，并在 root workspace 统一声明。
- 不引入向量数据库、外部队列、Mentle 或 LLVM。
- fake provider 与 fixtures 不依赖网络。
