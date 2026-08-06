# Release — GA-MEM-PARITY Wave 4（AutoDream 去重 — G4）

## 发布方式

代码变更随常规 crate 构建发布（laputa / autodream），无独立部署物。

## 行为变更提示

- **AutoDream 候选去重接通 typed authority**（G4）：worker `reflect()`
  构造 `BoundedReflectionInput.existing_memory_digests` 时，在既有 laputa
  section digest（legacy 路径）之外，**新增**对
  `LaputaService::applied_authority_digests()` 的调用，把当前 workspace
  所有活跃 AppliedAuthority 长时记录的 content digest 一并合并（HashSet
  并集）。下次 AutoDream run 不再重新提出与 `memory_add` 已 apply 内容
  完全相同的候选——`CandidateGate` 会按 `Duplicate` 拒绝。
- **失败降级**：若 typed authority store 读取失败（IO / schema / 其他
  非 "库不存在" 错误），AutoDream **不中断**——降级为 `tracing::warn!`
  + 空 digest 数组（去重变弱但 run 继续）。首次 run 场景（typed DB 文件
  尚未存在）gracefully 返回空数组，无告警。
- **新公开 API**：`LaputaService::applied_authority_digests()`
  （`pub async fn`）。其他 crate / Tauri command / Manager route 可直接
  调用获取当前 workspace 的"已 apply authority content digest 全集"，
  用于与外部候选源去重。
- **新错误变体**：`LaputaError::InvalidState(String)`，stable API code
  `invalid_state`，用于 service-layer wrapper 暴露的 typed-store 内部
  不变量违规。任何对 `LaputaError::code()` 做穷举匹配的下游需补 arm
  （wildcard 已覆盖则无影响）。

## GUI/CLI 影响

- 无 wire/SSE/Tauri 协议变更。
- GUI Evolution 页面：下次 AutoDream run 的 candidate 列表不再出现与已
  apply authority 同内容的候选（前提是 typed authority 读成功；失败场景
  见上方"失败降级"）。
- CLI `agent-diva autodream` / Manager 自动触发路径：行为一致，只是候选
  集合更紧。
