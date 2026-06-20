# 调研报告：Claude Code Context Window 管理策略

**调研日期**: 2026-06-07
**调研目标**: 深入分析 Claude Code 的模型上下文窗口管理策略，为 agent-diva-pro 的 context compaction 系统改进提供参考
**调研范围**: `~/Desktop/morediva/agent-diva/.workspace/claude-code/`

---

## 一、核心架构

Claude Code 采用**六层优先级**的 context window 解析链：

```
优先级从高到低：
1. 环境变量覆盖 (ant-only): CLAUDE_CODE_MAX_CONTEXT_TOKENS
2. 显式 [1m] 后缀: model 名包含 "[1m]" → 1,000,000
3. Anthropic API 动态获取 (modelCapabilities.ts):
   - 调用 /v1/models 获取 max_input_tokens
   - 缓存到 ~/.config/claude/cache/model-capabilities.json
4. Beta header 检测: betas 包含 'context-1m-2025-08-07' + modelSupports1M(model)
5. Sonnet 1M 实验特性: getSonnet1mExpTreatmentEnabled(model)
6. ant-only 内部模型表: resolveAntModel(model).contextWindow
7. 默认回退: MODEL_CONTEXT_WINDOW_DEFAULT = 200,000
```

---

## 二、关键文件分析

### 2.1 `src/utils/context.ts` — Context Window 计算核心

**默认常量：**
```typescript
export const MODEL_CONTEXT_WINDOW_DEFAULT = 200_000
export const COMPACT_MAX_OUTPUT_TOKENS = 20_000
const MAX_OUTPUT_TOKENS_DEFAULT = 32_000
const MAX_OUTPUT_TOKENS_UPPER_LIMIT = 64_000
export const CAPPED_DEFAULT_MAX_TOKENS = 8_000
export const ESCALATED_MAX_TOKENS = 64_000
```

**完整优先级实现：**
```typescript
export function getContextWindowForModel(
  model: string,
  betas?: string[],
): number {
  // 1. 环境变量覆盖 (ant-only)
  if (
    process.env.USER_TYPE === 'ant' &&
    process.env.CLAUDE_CODE_MAX_CONTEXT_TOKENS
  ) {
    const override = parseInt(process.env.CLAUDE_CODE_MAX_CONTEXT_TOKENS, 10)
    if (!isNaN(override) && override > 0) {
      return override
    }
  }

  // 2. [1m] 后缀 — 显式客户端启用
  if (has1mContext(model)) {
    return 1_000_000
  }

  // 3. API 动态获取
  const cap = getModelCapability(model)
  if (cap?.max_input_tokens && cap.max_input_tokens >= 100_000) {
    if (
      cap.max_input_tokens > MODEL_CONTEXT_WINDOW_DEFAULT &&
      is1mContextDisabled()
    ) {
      return MODEL_CONTEXT_WINDOW_DEFAULT
    }
    return cap.max_input_tokens
  }

  // 4. Beta header
  if (betas?.includes(CONTEXT_1M_BETA_HEADER) && modelSupports1M(model)) {
    return 1_000_000
  }

  // 5. Sonnet 1M 实验特性
  if (getSonnet1mExpTreatmentEnabled(model)) {
    return 1_000_000
  }

  // 6. ant-only 内部模型表
  if (process.env.USER_TYPE === 'ant') {
    const antModel = resolveAntModel(model)
    if (antModel?.contextWindow) {
      return antModel.contextWindow
    }
  }

  // 7. 默认回退
  return MODEL_CONTEXT_WINDOW_DEFAULT
}
```

**模型 Max Output Tokens 硬编码表：**

| 模型 | Default Max Output | Upper Limit |
|------|-------------------|-------------|
| opus-4-7 | 64,000 | 128,000 |
| opus-4-6 | 64,000 | 128,000 |
| sonnet-4-6 | 32,000 | 128,000 |
| opus-4-5 / sonnet-4 / haiku-4 | 32,000 | 64,000 |
| opus-4-1 / opus-4 | 32,000 | 32,000 |
| claude-3-opus | 4,096 | 4,096 |
| claude-3-sonnet | 8,192 | 8,192 |
| claude-3-haiku | 4,096 | 4,096 |
| 3-5-sonnet / 3-5-haiku | 8,192 | 8,192 |
| 3-7-sonnet | 32,000 | 64,000 |
| 其他 | 32,000 | 64,000 |

**Slot-reservation 优化：**
- `CAPPED_DEFAULT_MAX_TOKENS = 8,000` — 默认 cap，避免过度预留 slot
- `ESCALATED_MAX_TOKENS = 64,000` — 命中 cap 后的重试上限
- BQ p99 output = 4,911 tokens，32k/64k 默认会过度预留 8-16× slot 容量
- Cap 启用后 <1% 请求命中限制，这些请求会 clean retry 到 64k

---

### 2.2 `src/utils/model/modelCapabilities.ts` — 动态模型能力获取

**缓存 Schema：**
```typescript
const ModelCapabilitySchema = z.object({
  id: z.string(),
  max_input_tokens: z.number().optional(),
  max_tokens: z.number().optional(),
}).strip()

const CacheFileSchema = z.object({
  models: z.array(ModelCapabilitySchema()),
  timestamp: z.number(),
})
```

**缓存文件路径：**
```typescript
function getCachePath(): string {
  return join(getClaudeConfigHomeDir(), 'cache', 'model-capabilities.json')
}
```

**获取逻辑：**
```typescript
export function getModelCapability(model: string): ModelCapability | undefined {
  if (!isModelCapabilitiesEligible()) return undefined
  const cached = loadCache(getCachePath())
  if (!cached || cached.length === 0) return undefined
  const m = model.toLowerCase()
  const exact = cached.find(c => c.id.toLowerCase() === m)
  if (exact) return exact
  return cached.find(c => m.includes(c.id.toLowerCase()))
}
```

**刷新逻辑（异步）：**
```typescript
export async function refreshModelCapabilities(): Promise<void> {
  if (!isModelCapabilitiesEligible()) return
  if (isEssentialTrafficOnly()) return

  try {
    const anthropic = await getAnthropicClient({ maxRetries: 1 })
    const betas = isClaudeAISubscriber() ? [OAUTH_BETA_HEADER] : undefined
    const parsed: ModelCapability[] = []
    for await (const entry of anthropic.models.list({ betas })) {
      const result = ModelCapabilitySchema().safeParse(entry)
      if (result.success) parsed.push(result.data)
    }
    if (parsed.length === 0) return

    const path = getCachePath()
    const models = sortForMatching(parsed)
    if (isEqual(loadCache(path), models)) {
      logForDebugging('[modelCapabilities] cache unchanged, skipping write')
      return
    }

    await mkdir(getCacheDir(), { recursive: true })
    await writeFile(path, jsonStringify({ models, timestamp: Date.now() }), {
      encoding: 'utf-8',
      mode: 0o600,
    })
    loadCache.cache.delete(path)
    logForDebugging(`[modelCapabilities] cached ${models.length} models`)
  } catch (error) {
    logForDebugging(
      `[modelCapabilities] fetch failed: ${error instanceof Error ? error.message : 'unknown'}`,
    )
  }
}
```

**关键设计：**
- 仅 firstParty provider + anthropic 官方 URL 才启用
- 缓存使用 `lodash.memoize`，keyed on cache path
- 排序策略：最长 ID 优先（`b.id.length - a.id.length || a.id.localeCompare(b)`）
- 确保子串匹配时选到最具体的模型

---

### 2.3 `src/services/compact/autoCompact.ts` — 自动压缩触发逻辑

**有效窗口计算：**
```typescript
export function getEffectiveContextWindowSize(model: string): number {
  const reservedTokensForSummary = Math.min(
    getMaxOutputTokensForModel(model),
    MAX_OUTPUT_TOKENS_FOR_SUMMARY,  // 20,000
  )
  let contextWindow = getContextWindowForModel(model, getSdkBetas())
  
  // 环境变量覆盖
  const autoCompactWindow = process.env.CLAUDE_CODE_AUTO_COMPACT_WINDOW
  if (autoCompactWindow) {
    contextWindow = Math.min(contextWindow, parsed)
  }
  
  return contextWindow - reservedTokensForSummary
}
```

**自动压缩阈值：**
```typescript
export function getAutoCompactThreshold(model: string): number {
  const effectiveContextWindow = getEffectiveContextWindowSize(model)
  return effectiveContextWindow - getAutocompactBufferTokens(model)
}

export function getAutocompactBufferTokens(model: string): number {
  const effectiveWindow = getEffectiveContextWindowSize(model)
  if (effectiveWindow >= 800_000) return 50_000
  if (effectiveWindow >= 400_000) return 30_000
  return AUTOCOMPACT_BUFFER_TOKENS  // 13,000
}
```

**分档缓冲策略：**

| 有效窗口 | 缓冲 Tokens |
|---------|------------|
| ≥ 800K | 50,000 |
| ≥ 400K | 30,000 |
| < 400K | 13,000 |

---

### 2.4 `src/services/compact/compact.ts` — 压缩核心实现

**关键特性：**
1. **Forked Agent 复用**：压缩使用 forked agent 复用主对话的 prompt cache
2. **Strip & Reinject**：压缩前 strip images 和 reinjected attachments
3. **PTL 重试**：最多 3 次，每次丢弃最旧的 20% 消息组
4. **状态保留**：压缩后保留最近 5 个文件、plan、skills、MCP instructions
5. **Deferred Tools**：压缩后重新注入 deferred tools delta

**Circuit Breaker：**
- 连续 3 次压缩失败后停止尝试
- 避免无效 API 调用

---

### 2.5 `src/QueryEngine.ts` — Compaction 集成

**Compact Boundary 处理：**
```typescript
case 'system': {
  if (msg.subtype === 'compact_boundary' && msg.compactMetadata) {
    // 释放 pre-compaction 消息供 GC
    this.mutableMessages.splice(0, mutableBoundaryIdx)
    messages.splice(0, localBoundaryIdx)
    yield { type: 'system', subtype: 'compact_boundary', ... }
  }
}
```

---

## 三、可借鉴的设计模式

### 3.1 分层优先级解析
```
环境变量 > 显式标记 > API 动态获取 > Beta header > 内部表 > 默认回退
```
**适用场景**：agent-diva-pro 的 `BudgetConfig` 和 `ModelCapabilities`

### 3.2 模型能力缓存
```
API 获取 → 本地 JSON 缓存 → 内存 memoization
```
**关键细节**：
- 最长 ID 优先的子串匹配
- 无变化时跳过写入（`lodash.isEqual`）
- 文件权限 `0o600`

### 3.3 环境变量验证
使用 `validateBoundedIntEnvVar` 统一处理：
- 解析
- 边界检查
- 日志记录

### 3.4 自动压缩策略
```
有效窗口 = contextWindow - 预留输出 tokens - 缓冲 tokens
```
**分档缓冲**：
- 1M → 50K
- 400K → 30K
- 默认 → 13K

### 3.5 PTL 重试机制
- 压缩请求本身也可能 PTL
- 最多 3 次重试
- 每次丢弃最旧 20% 消息组

### 3.6 压缩后状态恢复
保留并重新注入：
- 最近 5 个文件附件
- Plan
- Skills
- MCP instructions
- Deferred tools delta

---

## 四、与 agent-diva-pro 的对比

| 特性 | Claude Code | agent-diva-pro (当前) |
|------|------------|---------------------|
| Context Window 获取 | 六层优先级 | 硬编码 180K |
| 模型能力缓存 | API + JSON + 内存 | 无 |
| 自动压缩阈值 | 动态计算（有效窗口 - 缓冲） | 固定比例 |
| PTL 重试 | 3次，丢弃最旧20% | 无 |
| 压缩后状态恢复 | 保留文件/plan/skills/MCP | 仅保留最近5条消息 |
| Slot 预留优化 | Capped default (8K) | 无 |

---

## 五、推荐改进方案

基于调研，建议 agent-diva-pro 采用**简化版三层策略**：

### 5.1 在 `ModelCapabilities` 中添加 `context_window`
```rust
pub struct ModelCapabilities {
    pub vision: bool,
    pub tools: bool,
    pub reasoning: bool,
    pub context_window: Option<usize>,  // 新增
}
```

### 5.2 在 `model_capabilities_for_model` 中实现硬编码表
```rust
pub fn model_capabilities_for_model(model: &str) -> ModelCapabilities {
    let context_window = match model {
        "deepseek-chat" | "deepseek-coder" => Some(128_000),
        "gpt-4o" | "gpt-4o-mini" => Some(128_000),
        "claude-sonnet-4" | "claude-opus-4" => Some(200_000),
        "claude-sonnet-4-6" | "claude-opus-4-6" => Some(1_000_000),
        _ => None,
    };
    // ...
}
```

### 5.3 修改 `BudgetConfig` 支持模型感知
```rust
impl BudgetConfig {
    pub fn for_model(model: &str) -> Self {
        let max_tokens = model_capabilities_for_model(model)
            .context_window
            .unwrap_or(180_000);
        Self {
            max_tokens,
            keep_recent: 5,
            summary_max_chars: 2000,
        }
    }
}
```

### 5.4 环境变量覆盖（可选）
```rust
pub fn from_env_or_model(model: &str) -> Self {
    if let Ok(override) = std::env::var("AGENT_DIVA_MAX_CONTEXT_TOKENS") {
        if let Ok(tokens) = override.parse::<usize>() {
            return Self {
                max_tokens: tokens,
                ..Self::default()
            };
        }
    }
    Self::for_model(model)
}
```

---

## 六、相关文件清单

### 调研源文件（agent-diva .workspace）
- `claude-code/src/utils/context.ts` — 核心 context window 计算
- `claude-code/src/utils/model/modelCapabilities.ts` — 动态模型能力获取
- `claude-code/src/utils/model/antModels.ts` — 内部模型配置
- `claude-code/src/utils/model/providers.ts` — Provider 选择
- `claude-code/src/services/compact/compact.ts` — 压缩核心
- `claude-code/src/services/compact/autoCompact.ts` — 自动压缩触发
- `claude-code/src/QueryEngine.ts` — QueryEngine 集成
- `claude-code/src/constants/betas.ts` — Beta headers
- `claude-code/src/services/api/claude.ts` — Max output tokens 计算

### agent-diva-pro 待修改文件
- `agent-diva-providers/src/base.rs` — `ModelCapabilities` 添加 `context_window`
- `agent-diva-agent/src/context_budget.rs` — `BudgetConfig` 支持模型感知
- `agent-diva-agent/src/compaction/compaction_exec.rs` — 集成模型感知的预算
- `agent-diva-agent/tests/compaction_real_test.rs` — 更新测试

---

## 七、后续工作

1. [ ] 在 `ModelCapabilities` 中添加 `context_window` 字段
2. [ ] 在 `model_capabilities_for_model` 中实现 tekaapi 常用模型的硬编码表
3. [ ] 修改 `BudgetConfig` 支持 `for_model()` 构造函数
4. [ ] 更新 `test_real_compaction` 验证不同模型的预算正确性
5. [ ] 考虑添加 Provider 动态获取层（如 LiteLLM `/models` 端点）
6. [ ] 实现分档缓冲策略（类似 Claude Code 的 1M→50K, 400K→30K）

---

*调研完成，等待进一步决策。*
