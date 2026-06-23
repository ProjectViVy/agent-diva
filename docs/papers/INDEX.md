# 论文参考集(diva 自主活动方向)

**创建**: 2026-06-18
**解析工具**: [Nebutra/MinerU-Skill](https://github.com/Nebutra/MinerU-Skill) `scripts/mineru.py`
**存放**: `/Users/mastwet/Desktop/morediva/papers/`

## 7 篇全部齐 ✅

| arXiv | 标题 | 文件 |
|-------|------|------|
| 2304.03442 | Generative Agents (Park et al., Stanford 2023) | `2304_03442.{md,pdf}` |
| 2305.02750 | A Survey on Proactive Dialogue Systems (Deng et al., IJCAI 2023) | `2305_02750.{md,pdf}` |
| 2410.12361 | Proactive Agent (Lu et al., THUNLP, ICLR 2025) | `2410_12361.{md,pdf}` |
| 2412.14352 | A Survey on LLM Inference-Time Self-Improvement (Dong et al., 2024) | `2412_14352.{md,pdf}` |
| 2210.03629 | ReAct (Yao et al., 2022) | `2210_03629.{md,pdf}` |
| 2303.11366 | **Reflexion** (Shinn et al., NeurIPS 2023) | `2303_11366.{md,pdf}` |
| 2212.08073 | Constitutional AI (Bai et al., Anthropic 2022) | `2212_08073.{md,pdf}` |

## 解析踩到的坑(下次避坑)

1. **Agent API 免费版硬限**: 10MB 文件 OR 20 页数,任一超标拒。
2. **Standard API 配额**: 即便 token 通过,server-side 配额/限流可能让后续请求 "Invalid token"。本次经验:首批成功后,后续请求被服务器拒。
3. **网络重定向**: `https://arxiv.org/pdf/2212.08073` 偶尔返回 HTML 错误页,需加 `User-Agent: Mozilla/5.0` 重试。
4. **arXiv ID 撞车**: `2303.11366` (Reflexion) vs `2303.11381` (MM-REACT) —— 差 15 ID 号容易搞错。**核心教训**: 下完论文一定要 grep 标题验证,不能只信 arxiv ID。

## 解析踩坑 → 修正事件

`2303.11381` 第一次下到的是 MM-REACT(Microsoft, Yang et al.)不是 Reflexion。原因可能是 MinerU 任务 ID 与上传 PDF 不匹配,或上传时拿错文件。

处理:
- 已下载正确的 Reflexion PDF → `papers/2303_11366.pdf` + `papers/2303_11366.md`
- MM-REACT markdown 挪到 `papers/_wrong/mm-react_misidentified.md`(留作参考)
- `papers/2303_11381.pdf`(MM-REACT PDF)保留未删 —— 也是个有效参考论文(ReAct 的多模态扩展)

## 用法(后续复用)

```bash
# 默认走 cloud Agent API(免费,小文件)
uv run /Users/mastwet/Desktop/morediva/.workspace/_notes/MinerU-Skill/scripts/mineru.py FILE.pdf --output out/

# 大文件分块
uv run .../mineru.py BIG.pdf --pages 1-20 --output out/

# 完全离线(pymupdf4llm,无云)
uv run .../mineru.py FILE.pdf --engine local --output out/

# Standard API(高质量,需 MINERU_TOKEN)
export MINERU_TOKEN=*** run .../mineru.py FILE.pdf --output out/
```

## 与 diva 自主活动的相关性(本研究的目标)

**T1 直接相关(必读)**:
- **2304 Park 2023 Generative Agents** — diva 自主活动的理论蓝图(Memory Stream / Retrieval / Reflection / Planning 四柱)
- **2410 Lu 2024 Proactive Agent** — 主动出击的形式化定义 + 评测基准
- **2305 Deng 2023 Proactive Survey** — Initiative / Anticipation / Planning 三要素框架
- **2412 Dong 2024 Self-Improvement Survey** — 自我升级的分类与挑战

**T2 相关架构**:
- **2210 ReAct** — Reasoning + Acting 交错,diva 的 agent loop 范式
- **2303 Reflexion** — 言语强化学习,self-improve 思路
- **2212 Constitutional AI** — 原则驱动的 AI 行为约束,与 diva Authority Spine 同源思想
