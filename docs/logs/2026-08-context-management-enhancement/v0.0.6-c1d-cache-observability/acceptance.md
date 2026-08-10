# C1d Acceptance

## 自动验收

1. T7：Anthropic cache read/create 字段进入 response usage、observer 与向后兼容 ledger。
2. T8：多 system 仅第一条 stable block 带 cache-control；混合工具仅 CORE 末项带 control。
3. 同 hash 无告警；声明变化为 expected break；provider/model/policy/TTL 变化为 policy
   break；无声明结构变化立即 warn。
4. 稳定前缀达到阈值后跳过暖缓存样本，连续两次显著 read miss 才 warn；命中会重置计数。
5. DeepSeek/Ollama/未知 mock 等未启用 provider 不收到 cache-control，原始 model ID 不变。

## 完成定义

C1d 在受影响 crate 测试、严格 Clippy、工作区三门和 CLI smoke 通过，并完成研究文档、
TODOLIST、迭代日志与 focused commit 后完成。
