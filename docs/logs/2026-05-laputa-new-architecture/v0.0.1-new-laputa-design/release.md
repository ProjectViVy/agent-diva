# 发布记录：新 Laputa 架构设计

## 版本信息
- 版本号: v0.0.1-new-laputa-design
- 发布日期: 2026-05-28

---

## 交付内容

本版本为架构设计文档，不含代码变更。

### 交付物清单

| 文件 | 说明 |
|---|---|
| `summary.md` | 完整设计：7 文件骨架、mentle 简化、三轴主体性、进阶心跳、疲劳值、上下文注入视图、实施优先级 |
| `verification.md` | 25 项验证检查（全部通过） |
| `release.md` | 本文件 |
| `acceptance.md` | 验收清单 |

### 影响范围

- **无代码变更**：纯设计文档
- **决策影响**：定义新 Laputa 架构方向，取代 laputa-next 的复杂度
- **与已有设计的关系**：吸收 GenericAgent 公理 + Laputa-next 人格薄层 + memtle 存储引擎，统一为极简 7 文件 + autodream 模型

### 后续发布计划

| 阶段 | 内容 | 前置条件 |
|---|---|---|
| P0 | 7 文件骨架 + 目录结构 | 本轮验收通过 |
| P1 | MEMORY.md + index.md + expectations.md | P0 |
| P2 | autodream 日报生成 + MEMORY.md 压缩 | P1 |
| P3 | SOP 继承 + SoulSignal 分类（自指） | P2 |

---

## 不含变更声明

- 不涉及 cargo build / cargo test
- 不涉及 just ci
- 不涉及配置文件变更
- 不涉及 secret 或 token
