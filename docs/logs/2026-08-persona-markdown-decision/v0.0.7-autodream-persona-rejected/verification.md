# Verification

对照代码：`inputs.rs` 默认读 Identity+MemoryMd；`candidates.rs` 只拒 SopCreate；
`outputs.rs` 按 proposal_type 建提案；`reflection.rs` 默认 MemoryPatch；
`laputa_propose_section_write` 可对四段 Frozen Core 建提案；autodream 生产
`tracing` 仅 worker 两处 warn。决策与 TODOLIST 已写。无 `just ci`。
