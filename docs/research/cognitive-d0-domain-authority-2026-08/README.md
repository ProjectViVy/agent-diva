# D0 总体认知领域与权威图

- 状态：`Design Draft / A-B-C Frozen / Remainder Awaiting Review`
- 日期：2026-08-14
- 性质：跨域架构合同。**不是** Architecture Gate，**不改**生产代码。
- 正文：[domain-authority.md](./domain-authority.md)
- 依据：R0–R4、P1–P21、S1–S9、Evolution D1–D6、现行 [Laputa 汇总](../../architecture/laputa/architecture.md)

用户授权：Research Gate 分域通过；「先拍 D0，先调研，再想」。

## 本包只回答

每个认知域：单一权威、scope、生命周期、写入者、历史、Prompt 投影、禁止依赖。

## 本包不回答

目录绝对路径字符串、Diff 引擎、Pulse/胶囊 schema、SOP 与 Skill 文件关系、删除切片、保护分支。

## 读法

1. 先读 `domain-authority.md` §1 调研结论和 §2 领域卡。
2. 再读 §5 禁止依赖、§11 已拍的 A/B/C。
3. 已冻产品以 P/S/D 原文为准。A/B/C 已拍（P22 / S1 修订 / D7）。其余 D0 仍待整体点头。
