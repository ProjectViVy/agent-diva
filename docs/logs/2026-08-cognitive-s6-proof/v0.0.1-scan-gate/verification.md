# 验证记录

| 门 | 命令 | 结果 |
| --- | --- | --- |
| 自测 | `python scripts/ci/check_cognitive_clean_break.py --self-test` | PASS（插入 MemoryMd / formatJson / MemoryGovernance 必红；证伪行不误伤） |
| 实扫 | `python scripts/ci/check_cognitive_clean_break.py` | PASS |
| 配方 | `just cognitive-clean-break-check` | PASS |
| CLI smoke | `cargo run -q -p agent-diva-cli -- --help` | PASS；无 `persona-retire` |
| 全量测试 | `just test` | 未跑（本片只加扫描脚本/just 配方 + 一处注释脱敏，无生产行为变化） |

第一次实扫命中的均为证伪字面量或已删符号注释；证伪规则收紧后转绿。唯一源码改动是 `consolidation.rs` 文档不再点名已删 `WorldGovernance`。
