# S08–S11 开发收口检查点

记录时间：2026-10-10。性质：开发验证检查点，不是正式验收、候选封版或发布。

## 结论

S08 的实际 App 召回链路继续通过针对性检查：记忆在独立 App 进程重启后被新会话实际检索并进入模型请求；三种召回 profile、六项负对照，以及纠正/删除后的模型输入检查已有通过证据。S09 新增了删除后重投已处理旧 run 的检查：ObserverHost 经夹具控制通道重放三次，原 canonical 来源、capture sequence、record、revision 与数量保持不变；删除卡片不再出现在公开搜索，新会话模型输入也不含旧事实或已删除 record。

S11 的竞争条件已定位到 race instrumentation 下的 Mentle hybrid card search。普通构建两次查询耗时 168ms 和 140ms，低于 ContextHost 的 750ms 默认源预算。race 构建的原生搜索耗时约 3.0s 和 2.0s；使用产品默认预算时请求超时并按安全降级处理，不产生候选。为了让独立的注入边界测试也能验证完整实际模型输入，race-only 测试夹具使用 5s 源预算：实际 hostile memory 到达模型的用户上下文，Mission、策略和 effect receipts 均未变化。该 seam 仅从测试夹具设置，生产默认仍为 750ms。

Garden `materialError` 现在保留未导出的 cause 链，供调用方和测试用 `errors.Is` 识别 deadline/cancel；公开错误文字和 JSON envelope 仍为原来的归一化值，没有泄露底层原因。这样可将 S11 的 race 失败精确归因为 deadline，而不是改变 wire contract。

## 验证结果

- S08/S09：最后一次针对性整组测试结果见 `raw/s08-s09-final-suite.jsonl`；S09 的独立过程输出见 `raw/s09-replay-final.jsonl`。命令使用 package mode、DIVA 生成程序集成 overlay 和本地 loopback 合成模型。
- S11 普通注入：PASS；原生查询分别为 168ms、140ms，见 `raw/s11-injection-normal.jsonl`。
- S11 race 注入：初始默认预算诊断失败，source cause 确认是 deadline；卡片搜索约 2.3s、证据展开约 11ms，见 `raw/s11-injection-race-default-timeout-diagnostic.jsonl`。race-only 5s 测试预算下实际注入路径 PASS，查询约 3.0s、2.0s，见 `raw/s11-injection-race-test-budget.jsonl`。
- S11 默认预算退化：race 用例 PASS；`material read failed` 保持无候选的安全降级，见 `raw/s11-degrade-default-timeout-race.jsonl`。
- Laputa Garden `agentapi`：`go test -count=1 ./agentapi` PASS，见 `raw/laputa-agentapi.txt`。
- 计划包静态检查：5 Epics、13 Stories、28 场景、无环依赖、10.5 人日、43 个源码路径检查 PASS。此检查不运行产品测试。

代码提交：VIVY `c58732e3d832078a1154c402b302fa59e4650b22`；Laputa `2aa49e74e2c51b44cc091f34979e985abd811ab4`。验证输入源码基线为 DIVA `0d0cadf9567120505ea7364c91fc7fd848cc7581`。逐项原始输出和 SHA-256 见 [checkpoint.json](checkpoint.json) 与 `raw/`。

## 未关闭的验收边界

所有 Story 仍保持 Planned/Blocked 原状态。S08 的 V19 WORLD/ACTMEM 自动上下文隔离与显式工具读取审计仍缺证据；S09 的其余全矩阵仍待正式前置及候选验收；S11 的后台故障/恢复矩阵、workspace A/B 授权与产品 profile/workspace 边界仍未验证。S01–S03 的正式前置 gate 仍阻止把这些开发证据计作 Done。没有合并、推送、发布或真实模型评估。
