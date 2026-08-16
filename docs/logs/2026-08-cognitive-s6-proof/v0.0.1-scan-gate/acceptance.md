# 验收

## 机器门（本片完成）

```
just cognitive-clean-break-check
```

- 自测插入 `LaputaSectionName::MemoryMd` / `formatJson` / `domain=memory` 必须失败。
- 生产 crate / GUI / justfile 对 D4 §3.1 族零未豁免命中。
- `just ci` 含此门。`just laputa-clean-break-check` 仍独立存在。

## D4 §7 真机八条（本片未做）

- [ ] 新 profile：五文件全缺 → 引导 → 第一次聊天 Frozen Core 是 Markdown
- [ ] 中央编辑器是 MD，不可能 `[object Object]`
- [ ] Memory / ACTMEM / MEMRULES 都不开 Approval
- [ ] 发言后能读 Pulse；本轮结束立刻有 Recap；另一 session 可读同一份
- [ ] Evolution 只有 Skill + 待审；接受后 `{config_dir}/skills/<slug>/SKILL.md` 出现
- [ ] 危险工具 Approval 仍走得通
- [ ] 旧工作区副本：不自动导入；Persona 不读 JSON section
- [ ] 子代理不读不写人格 / ACTMEM / BML / Skill

仍由 UI-S2/S3/S4 `*-DESKTOP-SMOKE` 跟踪。

## D4 §8 恢复演练（本片未做）

用 `protect/cognitive-pre-clean-break-20260815` @ `2aab18cc` 二进制打开旧工作区**拷贝**，确认旧 GUI 仍能开；新 tip 打开同一拷贝不写回旧 JSON。记录进 `docs/logs/`，不进产品。
