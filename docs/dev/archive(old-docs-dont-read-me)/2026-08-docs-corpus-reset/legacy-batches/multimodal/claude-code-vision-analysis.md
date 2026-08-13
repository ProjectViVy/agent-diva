# Claude Code 图片/Vision 消息处理分析报告

> 基于逆向工程代码分析，目标目录: `.workspace/claude-code/src/`

---

## 一、用户消息 content 的类型体系

用户消息（UserMessage）的 `message.content` 字段支持两种类型：

```
string | ContentBlockParam[]
```

### 定义位置
- `src/types/message.ts` — `MessageContent` 类型
- `src/utils/messages.ts:463-533` — `createUserMessage({ content })` 签名，接受 `string | ContentBlockParam[]`
- `src/utils/config.ts:54-62` — `PastedContent` 类型定义

### 运行时处理
`getUserMessageText()` (messages.ts:3193-3203) 和 `getContentText()` (messages.ts:3235-3245) 这两处展示了 string 和 ContentBlockParam[] 的切换逻辑：

```typescript
// messages.ts:3235-3245
export function getContentText(
  content: string | DeepImmutable<Array<ContentBlockParam>>,
): string | null {
  if (typeof content === 'string') {
    return content
  }
  if (Array.isArray(content)) {
    return extractTextContent(content, '\n').trim() || null
  }
  return null
}
```

`extractTextContent()` (messages.ts:3225-3233) 负责从 ContentBlockParam[] 中过滤出 type='text' 的块并拼接。

---

## 二、ContentBlock 类型定义

### 来源
所有 ContentBlock 类型从 `@anthropic-ai/sdk` 包导入。关键导入点：

### 2.1 `src/utils/messages.ts:3-14` (主要类型导入)
```typescript
import type {
  ContentBlock,
  ContentBlockParam,
  RedactedThinkingBlock,
  RedactedThinkingBlockParam,
  TextBlockParam,
  ThinkingBlock,
  ThinkingBlockParam,
  ToolResultBlockParam,
  ToolUseBlock,
  ToolUseBlockParam,
} from '@anthropic-ai/sdk/resources/index.mjs'
```

### 2.2 `src/services/api/claude.ts:1-19` (Beta 系列类型)
```typescript
import type {
  BetaContentBlock,
  BetaContentBlockParam,
  BetaImageBlockParam,       // ← image 类型
  BetaJSONOutputFormat,
  BetaMessage,
  BetaMessageDeltaUsage,
  BetaMessageStreamParams,
  BetaOutputConfig,
  BetaRawMessageStreamEvent,
  BetaRequestDocumentBlock,   // ← document (PDF) 类型
  BetaStopReason,
  BetaToolChoiceAuto,
  BetaToolChoiceTool,
  BetaToolResultBlockParam,
  BetaToolUnion,
  BetaUsage,
  BetaMessageParam as MessageParam,
} from '@anthropic-ai/sdk/resources/beta/messages/messages.mjs'
```

### 2.3 `src/bridge/inboundMessages.ts:1-5`
```typescript
import type {
  Base64ImageSource,
  ContentBlockParam,
  ImageBlockParam,
} from '@anthropic-ai/sdk/resources/messages.mjs'
```

### 2.4 `src/utils/imageResizer.ts:1-4`
```typescript
import type {
  Base64ImageSource,
  ImageBlockParam,
} from '@anthropic-ai/sdk/resources/messages.mjs'
```

### 关键 ContentBlock 子类型结构

| 类型 | type 字段 | 关键字段 |
|------|----------|---------|
| TextBlockParam | `'text'` | `text: string`, `cache_control?` |
| ImageBlockParam | `'image'` | `source: { type: 'base64', media_type: string, data: string }` |
| DocumentBlockParam | `'document'` | `source: { type: 'base64' / 'text' / 'content', ... }` |
| ToolUseBlockParam | `'tool_use'` | `id, name, input` |
| ToolResultBlockParam | `'tool_result'` | `tool_use_id, content, is_error?` |
| ThinkingBlockParam | `'thinking'` | `thinking: string, signature` |
| RedactedThinkingBlockParam | `'redacted_thinking'` | `data: string` |

### ImageBlockParam 结构详解
```typescript
{
  type: 'image',
  source: {
    type: 'base64',
    media_type: 'image/png' | 'image/jpeg' | 'image/gif' | 'image/webp',
    data: '<base64-encoded-string>'
  }
}
```

---

## 三、图片从粘贴/上传到 API 请求的完整流程

### 流程图概览

```
用户操作（Cmd+V 粘贴图片/拖拽文件）
    │
    ▼
┌─────────────────────────────────────────┐
│ 1. 剪贴板检测 & 读取                      │
│    src/utils/imagePaste.ts               │
│    - hasImageInClipboard() :86-124       │
│    - getImageFromClipboard() :126-249    │
│    - tryReadImageFromPath() :358-423     │
│    - isImageFilePath() :331-335          │
└────────────────────┬────────────────────┘
                     │ base64 + mediaType + dimensions
                     ▼
┌─────────────────────────────────────────┐
│ 2. 图片缩放/编码 (API 限制内)             │
│    src/utils/imageResizer.ts             │
│    - maybeResizeAndDownsampleImageBuffer │
│      :179-448                            │
│    - maybeResizeAndDownsampleImageBlock  │
│      :455-496                            │
│    - 限制: max 2000x2000px, target 3.75MB│
│      raw, 5MB base64                     │
└────────────────────┬────────────────────┘
                     │ ResizeResult { buffer, mediaType, dimensions }
                     ▼
┌─────────────────────────────────────────┐
│ 3. 图片存储到磁盘 (供 FileRead 工具访问)    │
│    src/utils/imageStore.ts               │
│    - storeImage() :54-79                 │
│    - storeImages() :84-99                │
│    - 存储路径: ~/.claude/image-cache/     │
│      {sessionId}/{id}.{ext}              │
└────────────────────┬────────────────────┘
                     │ 文件路径 (供元数据文本使用)
                     ▼
┌─────────────────────────────────────────┐
│ 4. 用户输入处理                           │
│    src/utils/processUserInput/           │
│      processUserInput.ts                 │
│    - processUserInputBase() :290-604     │
│    - 遍历 pastedContents 提取图片         │
│    - 创建 ImageBlockParam 并 resize       │
│    - 收集 imageMetadataTexts              │
│    - 传递 imageContentBlocks,             │
│      imagePasteIds 到 processTextPrompt   │
└────────────────────┬────────────────────┘
                     │ imageContentBlocks[], imagePasteIds[]
                     ▼
┌─────────────────────────────────────────┐
│ 5. 组装消息内容                           │
│    src/utils/processUserInput/           │
│      processTextPrompt.ts                │
│    - 合并 text + image blocks :66-87     │
│    - createUserMessage({                 │
│        content: [...text, ...image],      │
│        imagePasteIds                      │
│      })                                  │
└────────────────────┬────────────────────┘
                     │ UserMessage (content: ContentBlockParam[])
                     ▼
┌─────────────────────────────────────────┐
│ 6. 消息规范化 & 验证                      │
│    src/utils/messages.ts                 │
│    - normalizeMessages() :737-843        │
│      (split multi-block → single-block)  │
│    - normalizeMessagesForAPI() :2275-2666│
│      (合并、strip、validate)              │
│    - validateImagesForAPI() :2662-2663   │
│      → imageValidation.ts:65-104         │
│    - stripExcessMediaItems() (claude.ts:  │
│      978-1037) — 上限 100 个 media 项     │
└────────────────────┬────────────────────┘
                     │ validated UserMessage[]
                     ▼
┌─────────────────────────────────────────┐
│ 7. 发送 API 请求                          │
│    src/services/api/claude.ts            │
│    - queryModel() 构建 BetaMessageParam[] │
│    - image blocks 随 content[] 发送       │
│    - SDK 处理 base64 编码传输              │
└─────────────────────────────────────────┘
```

### 各步骤详细说明

#### 步骤 1: 剪贴板检测与读取 (imagePaste.ts)

**`hasImageInClipboard()` (第 86-124 行)**
- 仅在 macOS (darwin) 上检测
- 优先使用原生 NSPasteboard 读取 (`image-processor-napi`)
- 回退到 `osascript -e 'the clipboard as «class PNGf»'`

**`getImageFromClipboard()` (第 126-249 行)**
- macOS: 原生 CoreGraphics 读取，约 5ms 冷启动
- Linux: `xclip` 或 `wl-paste` 读取 PNG/BMP
- Windows: PowerShell `Get-Clipboard -Format Image`
- 支持 BMP → PNG 自动转换 (BMP API 不支持)
- 检测 magic bytes 获取真实图片格式

**`tryReadImageFromPath()` (第 358-423 行)**
- 用户拖拽图片文件到终端
- 支持绝对路径
- 支持 VSCode Terminal 的相对路径 + clipboard 路径匹配

#### 步骤 2: 图片缩放/编码 (imageResizer.ts)

**`maybeResizeAndDownsampleImageBuffer()` (第 179-448 行)**
- 使用 sharp (高性能图片处理库)
- 如果原始文件符合限制(≤3.75MB raw + ≤2000x2000px)，直接返回原图
- 超标时执行渐进式降级：
  1. PNG → PNG 最大压缩 (compressionLevel:9, palette:true)
  2. PNG → JPEG quality 80→60→40→20
  3. resize fit:inside + withoutEnlargement
  4. 二次压缩
  5. 兜底: 1000px 以内 + JPEG quality 20
- 捕获 sharp 错误时检查 magic bytes，如在 5MB base64 限制内则通过
- 错误分类 (classifyImageError): MODULE_LOAD / PROCESSING / PIXEL_LIMIT / MEMORY / TIMEOUT / VIPS / PERMISSION

**`maybeResizeAndDownsampleImageBlock()` (第 455-496 行)**
- 包装器：将 ImageBlockParam 反 base64 → 处理 → 重新 base64 编码
- 只处理 `source.type === 'base64'` 的图片

**`detectImageFormatFromBase64()` (第 834-843 行)**
- 通过 magic bytes 检测真实格式（PNG/JPEG/GIF/WEBP）

#### 步骤 3: 图片存储 (imageStore.ts)

**`storeImage()` (第 54-79 行)**
- 存储路径: `~/.claude/image-cache/{sessionId}/{imageId}.{ext}`
- 文件权限: 0o600
- 最大缓存: 200 个路径（LRU 驱逐，第 115-123 行）

**目的**: 让 Claude 的 FileRead 工具可以直接通过路径引用图片进行处理。

#### 步骤 4-5: 用户输入处理 (processUserInput.ts, processTextPrompt.ts)

`processUserInputBase()` (第 290-604 行) 是关键协调函数：

1. **数组 input 路径** (第 326-355 行): 当 input 是 `ContentBlockParam[]`（来自 SDK/bridge）时，遍历 image blocks 调用 `maybeResizeAndDownsampleImageBlock` 进行缩放
2. **pastedContents 路径** (第 363-430 行): 过滤 `isValidImagePaste` 的 content，并行 resize，收集 metadata
3. **拼接** (第 592-603 行): 调用 `processTextPrompt(normalizedInput, imageContentBlocks, imagePasteIds, ...)`

`processTextPrompt()` (processTextPrompt.ts:19-100):
- 第 67-87 行: 当有 `imageContentBlocks` 时，组装 `[...textContent, ...imageContentBlocks]`
- 第 75-78 行: `createUserMessage({ content: [...text, ...image], imagePasteIds })`
- 第 89-94 行: 无图片时 `createUserMessage({ content: input })`（string 路径）

#### 步骤 6: 消息规范化与验证 (messages.ts)

**`normalizeMessagesForAPI()` (第 2275-2666 行)**:
- 合并连续 UserMessage
- 剥离 error message 前的 document/image blocks (第 2399-2423 行)
- tool_results 必须在 content[] 最前面 (`hoistToolResults`)
- `sanitizeErrorToolResultContent` (第 2170-2193 行): strip is_error tool_result 中的非 text block
- **第 2662-2663 行**: `validateImagesForAPI(sanitized)` — 最终验证

**`validateImagesForAPI()` (imageValidation.ts:65-104)**:
- 遍历所有 user message 的 content blocks
- 检查 base64ImageBlock 的 `source.data.length`
- 超过 `API_IMAGE_MAX_BASE64_SIZE` (5MB) 则 throw `ImageSizeError`

**`stripExcessMediaItems()` (claude.ts:978-1037)**:
- 限制每次 API 请求最多 `API_MAX_MEDIA_PER_REQUEST` (100) 个 media 项
- 从最早的消息开始剥离多余的 images/documents

#### 步骤 7: API 发送 (claude.ts)

- `queryModel()` (第 1039+ 行) 将 UserMessage 转成 `BetaMessageParam`
- Image content blocks 作为 `content[]` 的一部分直接发送
- 通过 Anthropic SDK streaming endpoint

---

## 四、图片引用在 History 中的持久化

### 数据结构

**`PastedContent` 类型** (config.ts:54-62):
```typescript
export type PastedContent = {
  id: number          // 顺序编号
  type: 'text' | 'image'
  content: string     // base64 数据（图片）或文本内容
  mediaType?: string  // 如 'image/png'
  filename?: string
  dimensions?: ImageDimensions
  sourcePath?: string // 原始文件路径
}
```

**`HistoryEntry` 类型** (config.ts:69-72):
```typescript
export interface HistoryEntry {
  display: string
  pastedContents: Record<number, PastedContent>
}
```

### 引用格式 (history.ts:44-75)

- **文本引用**: `[Pasted text #N +X lines]` (第 51-55 行)
- **图片引用**: `[Image #N]` (第 58-59 行)
- **解析正则** (第 65-66 行):
  ```regex
  /\[(Pasted text|Image|\.\.\.Truncated text) #(\d+)(?: \+\d+ lines)?(\.)*\]/g
  ```

### 持久化流程 (history.ts)

1. **`addToHistory()`** (第 411-434 行): 接收 HistoryEntry
2. **`addToPromptHistory()`** (第 355-409 行):
   - 图片类型的 PastedContent **被过滤掉**（第 367 行: `if (content.type === 'image') continue`）
   - 短文本(≤1024字符)内联存储
   - 长文本通过 hash 引用到 pasteStore
3. **写入 `~/.claude/history.jsonl`** (第 116-143, 291-327 行)
4. **读取时恢复** (第 265-279 行): `logEntryToHistoryEntry()` 解析 pasteStore hash 引用
5. **当前会话优先排序** (第 190-217 行): `getHistory()` 先返回当前 session 的条目

### paste store 机制 (pasteStore.ts)

- **目录**: `~/.claude/paste-cache/`
- **`hashPastedText()`** (第 21-23 行): SHA256 前 16 hex 字符做 hash
- **`storePastedText()`** (第 37-53 行): 写 `{hash}.txt` 文件
- **`retrievePastedText()`** (第 59-70 行): 按 hash 读取
- **`cleanupOldPastes()`** (第 76-104 行): 根据 mtime 清理

### 图片的独立存储 (imageStore.ts)

图片不经过 pasteStore，而是直接存储到 `~/.claude/image-cache/{sessionId}/`:
- 存储为实际图片文件（base64 解码后写入）
- 供 Claude 的 FileRead 工具引用
- 按 session 隔离

---

## 五、Bridge 模式的图片处理

### inboundMessages.ts (1-82 行)

**`extractInboundMessageFields()` (第 21-42 行)**:
- 从 SDK 消息中提取 `content: string | ContentBlockParam[]`
- 调用 `normalizeImageBlocks()` 修复 bridge 客户端的字段名问题

**`normalizeImageBlocks()` (第 54-75 行)**:
- 解决 iOS/web 客户端发送 `mediaType`(camelCase) 而非 `media_type`(snake_case) 的问题
- 若无 media_type 字段，通过 `detectImageFormatFromBase64()` 从 magic bytes 检测
- 零分配优化: 不需要修改时直接返回原数组引用

### inboundAttachments.ts (1-175 行)

**`resolveInboundAttachments()` (第 123-134 行)**:
- 处理 web composer 上传的 `file_uuid` 附件
- 通过 OAuth 认证的 `GET /api/oauth/files/{uuid}/content` 下载
- 写入 `~/.claude/uploads/{sessionId}/` 目录
- 返回 `@"path"` 引用字符串

**`prependPathRefs()` (第 142-161 行)**:
- 将下载的 @path 引用添加到 content 的 **最后一个 text block**
- 这样 FileRead 工具可以自动读取这些附件

---

## 六、Token 估计中的图片处理

### `src/services/tokenEstimation.ts`

**`roughTokenCountEstimationForBlock()` (第 458-502 行)**:

图片和文档类型的 token 估计 (第 467-478 行):
```typescript
if (block.type === 'image' || block.type === 'document') {
  // https://platform.claude.com/docs/en/build-with-claude/vision#calculate-image-costs
  // tokens = (width px * height px)/750
  // Images are resized to max 2000x2000 (5333 tokens). Use a conservative
  // estimate that matches microCompact's IMAGE_MAX_TOKEN_SIZE
  return 2000  // ← 固定估算值
}
```

- 每张图片固定估算 **2000 tokens**
- 保守估计，与 microCompact 的 IMAGE_MAX_TOKEN_SIZE 一致
- 文档( PDF )也使用同样的 2000 token 估算
- 注释中的公式 `(width*height)/750` 是官方文档给出的，但实际使用固定值
- 实际 API 计费按 2000x2000 最大 = 5,333,333/750 ≈ 7111 tokens（但代码用 2000）

### 完整 token 计数链

1. `tokenCountWithEstimation()` (tokens.ts:251-292) — 入口
2. → `getTokenCountFromUsage()` (tokens.ts:55-65) — API 返回的用量
3. → `roughTokenCountEstimationForMessages()` (tokenEstimation.ts:371-383) — 新增消息
4. → `roughTokenCountEstimationForMessage()` (tokenEstimation.ts:385-413)
5. → `roughTokenCountEstimationForContent()` (tokenEstimation.ts:415-433)
6. → `roughTokenCountEstimationForBlock()` (tokenEstimation.ts:458-502) — 最终按块估算

---

## 七、API 限制常量

`src/constants/apiLimits.ts` (1-94 行):

| 常量 | 值 | 说明 |
|------|----|-----|
| `API_IMAGE_MAX_BASE64_SIZE` | 5 MB | base64 编码后的最大图片大小 |
| `IMAGE_TARGET_RAW_SIZE` | 3.75 MB | 原始图片目标大小 (5MB * 3/4) |
| `IMAGE_MAX_WIDTH` | 2000 px | 客户端缩放最大宽度 |
| `IMAGE_MAX_HEIGHT` | 2000 px | 客户端缩放最大高度 |
| `API_MAX_MEDIA_PER_REQUEST` | 100 | 每次请求最多 media 项 |
| `PDF_TARGET_RAW_SIZE` | 20 MB | PDF 原始文件限制 |
| `API_PDF_MAX_PAGES` | 100 | PDF 最大页数 |

---

## 八、错误处理与恢复

### query.ts 中的图片错误处理

**第 1215-1223 行**: 捕获 `ImageSizeError` / `ImageResizeError`:
```typescript
if (error instanceof ImageSizeError || error instanceof ImageResizeError) {
  yield createAssistantAPIErrorMessage({ content: error.message })
  return { reason: 'image_error' }
}
```

**第 1346-1447 行**: Media-size rejection 恢复:
- `mediaRecoveryEnabled` (第 809-810 行): 由 `reactiveCompact.isReactiveCompactEnabled()` 控制
- 图片过大导致 API 拒绝时，跳过 collapse drain（不会移除图片）
- 尝试 reactive compact 的 strip-retry 策略

### messages.ts 中的错误 stripping

**第 2289-2318 行**: 构建错误→块类型映射:
```typescript
[getImageTooLargeErrorMessage()]: new Set(['image']),
[getRequestTooLargeErrorMessage()]: new Set(['document', 'image']),
```

**第 2399-2423 行**: 从 isMeta 消息中剥离导致错误的 document/image blocks，防止每次 API 调用重复发送。

---

## 九、文件行号索引

| 功能 | 文件 | 行号 |
|------|------|------|
| PastedContent 类型 | `utils/config.ts` | 54-62 |
| HistoryEntry 类型 | `utils/config.ts` | 69-72 |
| [Image #N] 引用格式 | `history.ts` | 58-59 |
| parseReferences 正则 | `history.ts` | 62-75 |
| addToPromptHistory 图片过滤 | `history.ts` | 366-368 |
| pasteStore 存储/读取 | `utils/pasteStore.ts` | 21-70 |
| 剪贴板图片读取 | `utils/imagePaste.ts` | 126-249 |
| 图片文件路径读取 | `utils/imagePaste.ts` | 358-423 |
| 图片缩放/降采样 | `utils/imageResizer.ts` | 179-448 |
| ImageBlockParam 缩放 | `utils/imageResizer.ts` | 455-496 |
| base64 格式检测 | `utils/imageResizer.ts` | 834-843 |
| 图片元数据文本 | `utils/imageResizer.ts` | 850-895 |
| storeImages | `utils/imageStore.ts` | 84-99 |
| createUserMessage (含 imagePasteIds) | `utils/messages.ts` | 463-533 |
| normalizeMessages (图片拆分) | `utils/messages.ts` | 737-843 |
| normalizeMessagesForAPI | `utils/messages.ts` | 2275-2666 |
| validateImagesForAPI 调用 | `utils/messages.ts` | 2662-2663 |
| validateImagesForAPI 实现 | `utils/imageValidation.ts` | 65-104 |
| getUserMessageText | `utils/messages.ts` | 3193-3203 |
| extractTextContent | `utils/messages.ts` | 3225-3233 |
| getContentText | `utils/messages.ts` | 3235-3245 |
| processUserInput (总入口) | `utils/processUserInput/processUserInput.ts` | 89-279 |
| processUserInputBase (图片处理核心) | `utils/processUserInput/processUserInput.ts` | 290-604 |
| processTextPrompt (组装 content) | `utils/processUserInput/processTextPrompt.ts` | 19-100 |
| stripExcessMediaItems | `services/api/claude.ts` | 978-1037 |
| mediaRecoveryEnabled | `query.ts` | 809-810 |
| 图片错误处理 | `query.ts` | 1215-1223 |
| media recovery 逻辑 | `query.ts` | 1346-1447 |
| normalizeImageBlocks (bridge) | `bridge/inboundMessages.ts` | 54-75 |
| resolveInboundAttachments | `bridge/inboundAttachments.ts` | 123-134 |
| prependPathRefs | `bridge/inboundAttachments.ts` | 142-161 |
| 图片 token 估算 (2000) | `services/tokenEstimation.ts` | 467-478 |
| API 限制常量 | `constants/apiLimits.ts` | 1-94 |

---

## 十、总结

Claude Code 的图片/vision 处理是一个多层的管道：

1. **采集层** (imagePaste.ts): 从剪贴板/文件拖拽获取原始图片 → base64 编码
2. **缩放层** (imageResizer.ts): sharp 渐进式降采样保证在 API 限制内 (max 2000×2000, 5MB base64)
3. **存储层** (imageStore.ts): 写入磁盘供工具访问
4. **组装层** (processUserInput.ts + processTextPrompt.ts): 将图片作为 ContentBlockParam[] 与文本合并
5. **规范化层** (messages.ts): normalizeMessagesForAPI 验证、剥离、合并
6. **验证层** (imageValidation.ts): 最终 API limit check
7. **传输层** (claude.ts): 通过 SDK streaming 发送
8. **持久化层** (history.ts + pasteStore.ts): [Image #N] 引用格式，图片独立存储在 image-cache 目录
9. **桥接层** (inboundMessages.ts + inboundAttachments.ts): 处理 remote bridge 客户端的字段兼容性和文件下载
10. **估算层** (tokenEstimation.ts): 固定 2000 tokens/图片
