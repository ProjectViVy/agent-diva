# Verification

## Commands

```bash
cargo check -p agent-diva-providers    # clean, no warnings
cargo fmt                              # formatting applied
cargo test -p agent-diva-providers     # 60 tests passed
```

## Test Coverage

新增测试 `reasoning_capabilities_are_conservative` 验证:
- 未知模型返回 false
- DeepSeek/Anthropic/OpenAI/Gemini/Qwen/Doubao 模型返回 true
- 有前缀的模型名 (deepseek/deepseek-r1) 正确匹配

## Known Issues

- `cargo clippy -D warnings` 在 `agent-diva-core/src/config/schema.rs` 有 3 个已有 `derivable_impls` 错误，非本次引入
