# S5 验证记录（2026-08-16）

全量门在收尾态（`1ea54d08`）执行，全部通过（仅余既有基线失败）：

| 门 | 命令 | 结果 |
| --- | --- | --- |
| 格式 | `just fmt-check` | PASS |
| Lint | `just check`（clippy -D warnings） | PASS |
| 全量测试 | `just test`（`cargo test --all`） | 仅 6 例既有 `CLI-WIREMOCK-502-PREEXISTING` 失败（CLI lib 4 passed / 6 failed，Windows wiremock 502，S5 未触碰 agent-diva-cli wiremock 用例） |
| 工作区测试 | `cargo test --workspace --exclude agent-diva-cli --exclude agent-diva-gui` | PASS（58 个测试目标 ok） |
| GUI 单测 | GUI `npm test`（vitest） | PASS（59 文件 / 427 tests） |
| Tauri | `cargo test -p agent-diva-gui`（src-tauri） | PASS（lib 43 + 集成 11） |
| CLI smoke | `cargo run -q -p agent-diva-cli -- --help` | PASS（usage 正常输出） |

## 证伪/收缩测试（防回归）

- `agent-diva-manager` 路由测试：12 个旧 laputa 治理 URI 断言 404；
  `.laputa/proposals` 目录空断言。
- `agent-diva-gui` locale 测试重写：现役 key 必在、`removedKeys`（旧 evolution/laputa
  治理 key）必为 undefined。
- `world_claim_routing` 集成测试翻转：world-claim 形内容走 `memory_add`，
  `.laputa/cognitive` 不被创建。
- `agent-diva-agent::memory_boundary`：默认 provider = `MemoryHome`，不产生
  `MEMORY.md`/`.laputa/memory.sqlite3`（workspace 级）/memory 目录。
- `agent-diva-core` 配置测试：`authority_mode` 键解析为无效噪音，不再选 provider。
- Manager autodream/e2e 测试注入 `with_skill_reflection_engine(None)`，避免回退到
  真实 provider 配置发起 LLM 调用（该问题在拆线过程中暴露并修复）。

## D4 §3.1 扫描族 sweep

对每个扫描族执行 `git grep` 全仓扫描，生产路径零命中；残留命中逐条定性为
豁免项（migration 导入词汇、CSS 类名、S4 语义方法名、BML 记录种类、证伪测试
字面量、docs/research）。幸存模块文档已随 `1ea54d08` 脱敏，S6 可直接落严格扫描门。
