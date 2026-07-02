# GenericAgent 与 OpenFang 图片/Vision 实现分析报告

> 分析日期: 2026-06-01
> 分析范围: `.workspace/GenericAgent/` 和 `.workspace/openfang/`
> 作者: Hermes Agent (deepseek-v4-pro)

---

## 目录

1. [GenericAgent put_task 接口设计](#1-genericagent-put_task-接口设计)
2. [图片在不同前端的流转路径](#2-图片在不同前端的流转路径)
   - [2.1 QT 桌面端](#21-qt-桌面端-qtapppy)
   - [2.2 Telegram Bot](#22-telegram-bot-tgapppy)
   - [2.3 Desktop Bridge (Web2)](#23-desktop-bridge-web2-desktop_bridgepy)
   - [2.4 Conductor (调度器)](#24-conductor-调度器-conductorpy)
3. [Vision API 集成分析](#3-vision-api-集成分析)
4. [LLM 核心层的图片转换](#4-llm-核心层的图片转换-llmcorepy)
5. [图像处理工具模块](#5-图像处理工具模块)
6. [OpenFang 项目 Vision 实现](#6-openfang-项目-vision-实现)
7. [与 Codex / ClaudeCode / Hermes 的设计对比](#7-与-codex--claudecode--hermes-的设计对比)
8. [关键发现与改进建议](#8-关键发现与改进建议)

---

## 1. GenericAgent put_task 接口设计

### 核心接口定义

**文件**: `agentmain.py`, 行 107-110

```python
def put_task(self, query, source="user", images=None):
    display_queue = queue.Queue()
    self.task_queue.put({
        "query": query,
        "source": source,
        "images": images or [],
        "output": display_queue
    })
    return display_queue
```

### 接口分析

- `query`: 字符串类型的用户输入（必须）
- `source`: 来源标识，如 `"user"`, `"telegram"`, `"subagent:xxx"`, `"conductor"`（默认 `"user"`）
- `images`: 可选列表参数，设计意图是传递图片数据
- 返回: `queue.Queue`，调用方通过该队列轮询获取流式输出

### 任务消费端（agentmain.py 行 128-176）

```python
def run(self):
    while True:
        task = self.task_queue.get()
        raw_query, source, display_queue = task["query"], task["source"], task["output"]
        # ...
```

**关键发现**: 在 `run()` 方法中，`task["images"]` **从未被提取或使用**。`agent_runner_loop()` 的调用只传递了 `raw_query`（字符串），没有传递图片数据。

```python
# agentmain.py 行 149 - 图片参数未被传递
gen = agent_runner_loop(self.llmclient, sys_prompt, raw_query, handler, TOOLS_SCHEMA,
                         max_turns=80, verbose=self.verbose, yield_info=True)
```

`agent_runner_loop()` 函数签名（`agent_loop.py` 行 42-43）:
```python
def agent_runner_loop(client, system_prompt, user_input, handler, tools_schema,
                       max_turns=40, verbose=True, initial_user_content=None, yield_info=False):
```

`user_input` 仅作为纯文本传入第一个 user message（`agent_loop.py` 行 45-46）:
```python
messages = [
    {"role": "system", "content": system_prompt},
    {"role": "user", "content": initial_user_content if initial_user_content is not None else user_input}
]
```

### 结论

**`put_task` 的 `images` 参数目前是一个预留接口，定义了但实际链路中断**。图片数据被放入任务队列后，在消费端被忽略，从未到达 LLM API 层。

---

## 2. 图片在不同前端的流转路径

### 2.1 QT 桌面端 (qtapp.py)

**文件**: `frontends/qtapp.py`

#### 图片选择流程

**`_attach_files()`** (行 1903-1928):
1. 用户点击附件按钮 → 调用 `QFileDialog.getOpenFileNames()`
2. 支持的过滤器: `Images (*.png *.jpg *.jpeg *.gif *.webp *.bmp)` 和文本文件
3. 读取文件原始字节 → 检查大小限制（`MAX_UPLOAD_BYTES = 10MB`）
4. 根据扩展名推断 MIME 类型（`image/png`, `image/jpeg` 等）
5. 存入 `self._pending_files` 列表: `[{"name": str, "type": str, "raw": bytes}]`

**`_refresh_chips()`** (行 1930-1947):
- 在输入框上方显示待发送文件的 chip 标签

#### 图片嵌入 Prompt

**`_build_prompt_with_uploads()`** (行 460-517):
1. 遍历 `files` 列表
2. 保存文件到 `temp/uploaded/{timestamp}_{safe_name}`
3. **图片特殊处理** (行 488-495):
   - 对图片进行 base64 编码
   - 将 base64 数据 URI 嵌入 prompt 文本中:
     ```
     \n- [图片附件] {name} ({size} bytes)
       磁盘路径: {saved}
       data:{mime};base64,{b64}
     ```
   - 标记为 `img_count += 1`
4. 文本文件则内联内容（截断至 6000 字符）

#### 发送调用

**`_handle_send()`** (行 1981-2028):
```python
self._display_queue = self.agent.put_task(
    f"{FILE_HINT}\n\n{full_prompt}",
    source="user"
)
```

**关键发现**: QT 前端**没有使用 `put_task` 的 `images` 参数**。图片数据被直接以 base64 文本形式嵌入 prompt 字符串中。LLM 是否能"看到"图片，完全取决于 LLM 后端是否能解析 prompt 文本中的 `data:image/...;base64,...` 数据。

这种方式存在严重问题:
1. base64 图片数据极长（一张 500KB 的图片 base64 后约 667KB 文本），会大量消耗 token
2. `agentmain.py` 中有 `rquery = smart_format(raw_query.replace('\n', ' '), max_str_len=200)` (行 136)，将 query 压缩并截断，base64 数据可能在此被损坏
3. 没有格式化的 vision content block，LLM 可能不知道那里是图片

---

### 2.2 Telegram Bot (tgapp.py)

**文件**: `frontends/tgapp.py`

#### 图片接收

**`handle_photo()`** (行 1040-1060):
1. 检测 `update.message.photo`（Telegram 压缩图）或 `update.message.document`（原图/文件）
2. 取最高分辨率版本: `photo = update.message.photo[-1]`
3. 下载到本地: `temp/tg_{file_unique_id}.jpg`
4. 拼接 prompt 字符串:
   ```python
   prompt = f"[TIPS] 收到{kind}temp/{fpath}，请等待下一步指令"
   # 或带 caption 时: f"[TIPS] 收到{kind}temp/{fpath}\n{caption}"
   ```
5. 调用: `agent.put_task(prompt, source="telegram")`

**关键发现**: Telegram 前端的图片处理是**最原始的方式**——仅下载保存到磁盘，在 prompt 文本中告知文件路径。**完全没有将图片内容传递给 LLM 进行视觉理解**。Agent 需要通过工具 `file_read` 读取文件来间接接触图片，但 `file_read` 是文本读取工具，无法处理二进制图片。

#### 发送响应中的图片

**`_send_files()`** (行 187-200):
- 通过 `reply_photo()` 发送生成的 `.png/.jpg/.gif/.webp` 文件
- 通过 `reply_document()` 发送其他文件

这是输出端的功能，与 vision 输入无关。

---

### 2.3 Desktop Bridge (Web2) (desktop_bridge.py)

**文件**: `frontends/desktop_bridge.py`

#### HTTP API 图片接收

**`prompt_handler()`** (行 536-541):
```python
async def prompt_handler(request):
    data = await read_json(request)
    prompt = data.get("prompt", ...)
    images = data.get("images") or []
    return json_ok(manager.submit_prompt(sid, prompt, images))
```

HTTP POST `/session/{sid}/prompt` 接受 JSON body:
```json
{
  "prompt": "描述这张图",
  "images": [{"id": "img-xxx", "dataUrl": "data:image/png;base64,..."}]
}
```

#### 图片规范化

**`normalize_prompt()`** (行 336-380):
1. 支持 OpenAI Chat Completion 格式的 content array
2. 提取 `type: "image"` 或 `type: "input_image"` 块中的 URL
3. **`_save_image_data()`** (行 316-333): 解析 data URL，提取 base64，解码后写入 `temp/ga_web2_uploads/{img_id}.{ext}`
4. 在 prompt 末尾追加 `[image:path]` 标记:
   ```python
   final_prompt = final_prompt + "\n" + "\n".join(image_tags)
   ```

#### Agent 调用

**`run_agent_turn()`** (行 205-272):
```python
display_q = agent.put_task(prompt, images=images or [])
```

Desktop Bridge **是唯一实际传递 `images` 参数的前端**，但由于 agentmain.py 的 `run()` 方法不消费该参数，此传递仍然是无效的。

**`submit_prompt()`** (行 182-203) 额外将 `image_ids` 存入消息的 `extra` 字段:
```python
if image_ids:
    extra["image_ids"] = image_ids
user_msg = self.add_message(sess, "user", prompt, **extra)
```

---

### 2.4 Conductor (调度器) (conductor.py)

**文件**: `frontends/conductor.py`

Conductor 使用 `ChatIn` 模型 (行 31-33)，只有 `msg: str` 和 `role: str` 两个字段，**完全没有图片支持**。

`start_subagent()` (行 175-190):
```python
dq = agent.put_task(prompt, source=f"subagent:{sid}")
```

不传递 images 参数。

---

## 3. Vision API 集成分析

### vision_api.template.py

**文件**: `memory/vision_api.template.py` (113 行)

这是一个**独立的、模板性质**的 Vision API 封装模块，**未被集成到主 Agent 循环中**。

#### 支持的三种后端

| 后端 | API 格式 | 模型 |
|------|----------|------|
| Claude | Anthropic Messages API (`/v1/messages`) | 从 mykey.py 配置 |
| OpenAI | OpenAI Chat Completions (`/v1/chat/completions`) | 从 mykey.py 配置 |
| ModelScope | OpenAI 兼容 (`api-inference.modelscope.cn`) | `Qwen/Qwen3-VL-235B-A22B-Instruct` |

#### 图片预处理 (`_prepare_image()`, 行 49-72)

1. 支持 PIL Image 对象或文件路径输入
2. 缩放: 像素总数超过 `max_pixels`(默认 1440000 = 1200×1200) 时等比缩放
3. 格式转换: RGBA/LA/P → RGB
4. 编码: JPEG, quality=80, optimize=True
5. Base64 编码

#### Claude 调用 (`_call_claude()`, 行 78-94)

```python
{
    "type": "image",
    "source": {"type": "base64", "media_type": "image/jpeg", "data": b64}
}
```

使用 Anthropic 原生 content block 格式，正确的 `/v1/messages` 端点。

#### OpenAI 调用 (`_call_openai_compat()`, 行 96-111)

```python
{
    "type": "image_url",
    "image_url": {"url": f"data:image/jpeg;base64,{b64}"}
}
```

使用标准的 OpenAI Vision API 格式。

#### 与 Agent 主循环的关系

**完全没有关系**。该文件需要在 mykey.py 同级目录被手动导入使用，属于工具性质而非集成性质。

---

## 4. LLM 核心层的图片转换 (llmcore.py)

**文件**: `llmcore.py`

LLM 核心层具备图片 message block 的处理能力，但这能力**从未被 Agent 循环触发**。

### Claude → OpenAI 格式转换 (`_msgs_claude2oai()`, 行 472-515)

```python
# 行 507-510: 处理 Claude 格式的 image block
elif b.get("type") == "image":
    src = b.get("source") or {}
    if src.get("type") == "base64" and src.get("data"):
        text_parts.append({
            "type": "image_url",
            "image_url": {
                "url": f"data:{src.get('media_type', 'image/png')};base64,{src.get('data', '')}"
            }
        })
elif b.get("type") == "image_url":
    text_parts.append(b)  # 行 511: 透传已有的 image_url
```

这表明 llmcore 已经准备好在 message history 中携带图片块，正确地在 Claude 和 OpenAI 格式间转换。

### Responses API 中的图片 (`_to_responses_input()`, 行 436-469)

```python
# 行 458-460: 处理 image_url 类型的内容块
elif ptype == "image_url":
    url = (part.get("image_url") or {}).get("url", "")
    if url and role != "assistant":
        parts.append({"type": "input_image", "image_url": url})
```

正确地将 OpenAI 的 `image_url` 映射为 Responses API 的 `input_image`。

### 结论

LLM 核心层在 message 格式层面已经支持图片，但上游（Agent 循环）从未向 messages 中注入图片块。

---

## 5. 图像处理工具模块

### ui_detect.py (UI 元素检测)

**文件**: `memory/ui_detect.py` (119 行)

- 基于 OmniParser YOLO + RapidOCR 的 UI 元素检测
- `detect(image_path, mode='crop'|'match')` → 返回 bbox + label 列表
- 两种模式: `crop`（YOLO + 逐块 OCR）和 `match`（全图 OCR 空间匹配）
- 依赖: `ultralytics`, `rapidocr-onnxruntime`, `pillow`, `numpy`
- **非 LLM Vision**，属于传统 CV + OCR 方案

### ocr_utils.py (OCR 工具)

**文件**: `memory/ocr_utils.py` (100 行)

- 基于 `rapidocr-onnxruntime` 的本地 OCR
- `ocr_image()`: PIL Image 或路径输入
- `ocr_screen()`: 屏幕截图 + OCR
- `ocr_window()`: Windows 窗口截图 + OCR（使用 PrintWindow API，支持 RDP 断开场景）
- 返回: `{'text': 全文, 'lines': [行], 'details': [bbox + conf]}`
- **非 LLM Vision**，属于传统 OCR 方案

---

## 6. OpenFang 项目 Vision 实现

### 项目概述

OpenFang 是用 Rust 编写的 Agent 框架（15 个 crate），原生支持 Multi-Agent 协作、SQLite 记忆、WASM 沙箱、41+ 内置工具。

### ContentBlock::Image 类型定义

**文件**: `crates/openfang-types/src/message.rs`, 行 98-105

```rust
#[serde(rename = "image")]
Image {
    /// MIME type (e.g. "image/png", "image/jpeg").
    media_type: String,
    /// Base64-encoded image data.
    data: String,
},
```

作为 `ContentBlock` 枚举的变体之一（与 `Text`, `ToolUse`, `ToolResult`, `Thinking`, `RedactedThinking` 并列）。

### 图片验证 (`validate_image()`, 行 176-195)

```rust
const ALLOWED_IMAGE_TYPES: &[&str] = &["image/png", "image/jpeg", "image/gif", "image/webp"];
const MAX_IMAGE_BYTES: usize = 5 * 1024 * 1024;  // 5 MB
```

- 媒体类型白名单: png, jpeg, gif, webp
- 5MB 大小限制（解码后）
- 严格的输入验证

### 三个原生 LLM 驱动的图片映射

根据 `docs/architecture.md` (行 361-365):

1. **AnthropicDriver**: 原生映射 `ContentBlock::Image` → Anthropic Messages API 的 image content block，支持媒体类型验证和 5MB 限制
2. **GeminiDriver**: 映射至 Google Gemini API (`v1beta`)
3. **OpenAiCompatDriver**: 映射至 OpenAI-compatible `image_url` content block，覆盖 18+ 提供商

### API 使用示例

**文件**: `docs/api-reference.md`, (行 2146-2151)

```json
{
  "role": "user",
  "content": [
    {"type": "text", "text": "Describe this image"},
    {"type": "image_url", "image_url": {"url": "data:image/png;base64,iVBOR..."}}
  ]
}
```

通过 `/v1/chat/completions` 端点（OpenAI 兼容 API），使用标准 `image_url` 格式。

### Changelog 记录

**文件**: `CHANGELOG.md`, 行 155-160

```
#### Image Support
- `ContentBlock::Image` with base64 inline data
- Media type validation (png, jpeg, gif, webp only)
- 5MB size limit enforcement
- Mapped to all 3 native LLM drivers
```

### 多模态消息合并

**文件**: `CHANGELOG.md`, 行 18

```
- Multimodal user messages now combine text and image blocks
  into a single message so the LLM sees both. Fixes #1043.
```

这表明 OpenFang 已经修复了多模态消息中文本和图片分离的问题，确保 LLM 同时收到两者。

### 工具输出中的图片

**文件**: `docs/launch-roadmap.md`, 行 108-112

- `browser_screenshot` 工具将截图保存到 uploads temp 目录
- 返回 JSON 包含 `image_urls`
- `chat.js` 检测 `browser_screenshot` 工具结果并填充 `_imageUrls` 用于内联显示

---

## 7. 与 Codex / ClaudeCode / Hermes 的设计对比

### 对比总览

| 特性 | GenericAgent | OpenFang | Claude Code | Codex (OpenAI) | Hermes Agent |
|------|-------------|----------|-------------|----------------|--------------|
| **图片作为一等类型** | ❌ (仅有预留参数) | ✅ ContentBlock::Image | ✅ tool_result image | ✅ image_url | ✅ vision_analyze 工具 |
| **多模态 API 原生支持** | ❌ (未连接) | ✅ 3 个原生驱动 | ✅ Anthropic Messages | ✅ Responses API | ✅ 多种后端 |
| **图片大小限制** | 无 (前端 10MB) | 5MB 解码后 | ~5MB (API 限制) | ~20MB (API 限制) | 可配置 max_pixels |
| **图片格式验证** | 仅前端扩展名检测 | 白名单 (png/jpeg/gif/webp) | 客户端验证 | 服务端验证 | PIL 预处理 |
| **图片预处理** | 无 | 无 | 自动缩放 | 自动缩放 | PIL 缩放+格式转换 |
| **图片流转方式** | base64 嵌入 prompt 文本 (QT) / 仅存磁盘 (TG) | ContentBlock 附加到 message | Tool 输出 image 块 | Content part | 函数调用传参 |
| **前端图片上传** | QT: QFileDialog / TG: Telegram Photo API / Web: data URL | 桌面/Web/CLI 均支持 | CLI 粘贴/拖拽 | CLI/IDE | CLI + 多种前端 |
| **图片与文本合并** | 文本中嵌入 base64 (QT) | ✅ 同一消息中的多个 block | ✅ 同一消息 | ✅ 同一消息 | ✅ 函数参数 |

### 详细对比

#### Claude Code (Anthropic 官方 CLI Agent)

- 使用 Anthropic Messages API 的原生 `content: [{type: "image", source: {...}}, {type: "text", ...}]` 格式
- 工具（如 bash、browser）可将截图作为 tool_result 的 image content block 返回
- 图片处理全在 LLM 服务端完成，客户端只负责编码和传输
- 设计哲学: 图片是消息的**内容块**（content block），与文本、tool_use/tool_result 同级

#### Codex (OpenAI)

- 使用 OpenAI Responses API 的 `input_image` 类型
- Codex CLI 通过工具输出返回图片（如 `screenshot` 工具）
- 用户可通过 CLI 直接拖入/粘贴图片
- 设计哲学: 图片是消息的**部分**（content part），与文本并列

#### Hermes Agent

- Vision 通过 `vision_analyze` 工具函数实现，而非内联 content block
- 支持多种后端（Claude/OpenAI/ModelScope）的切换
- 有独立的图片预处理流程（缩放、格式转换、base64 编码）
- 设计哲学: Vision 是**工具调用**，结果作为文本返回给 Agent

#### GenericAgent

- `put_task` 定义了 `images` 参数但未使用 — **接口存在但链路断裂**
- QT 前端将 base64 嵌入 prompt 文本 — **最脆弱的方案**
- Telegram 前端仅保存文件路径 — **完全没有 vision**
- Desktop Bridge 有 normalized 但同样没有实际传递给 LLM
- 提供了出色的工具模块（ui_detect、ocr_utils）但它们是独立的本地工具，非 LLM vision
- 设计哲学: 图片处理是**外挂工具**，未与 LLM 多模态能力打通

#### OpenFang

- `ContentBlock::Image` 是 Rust 类型系统的一等公民
- 三个原生驱动全部正确映射 image block
- 有严格的输入验证（类型白名单 + 大小限制）
- 支持多模态消息合并（文本 + 图片在同一消息中）
- 设计哲学: 图片是**原生 content block**，与 Claude Code 对齐

---

## 8. 关键发现与改进建议

### 核心问题

**GenericAgent 的图片/vision 链路存在系统性断裂**:

1. `put_task(images=...)` 参数在 `run()` 方法中被忽略（agentmain.py:130）
2. `agent_runner_loop()` 的 `user_input` 参数只接受纯文本（agent_loop.py:42-46）
3. 各个前端的图片处理各自为政，没有一个统一的图片传递机制

### 现状总结

```
前端图片处理:
  QT:   图片 → base64 嵌入 prompt 文本 → put_task(prompt)          [❌ 未使用 images 参数]
  TG:   图片 → 保存磁盘 → 路径写入 prompt → put_task(prompt)        [❌ 完全没有 vision]
  Web:  图片 → data URL → 保存磁盘 → [image:path] 标签 → put_task(prompt, images=[...])  [❌ images 参数未被消费]

LLM 层能力:
  llmcore.py:  支持 image_url / input_image / Claude image block   [✅ 格式转换就绪]
  实际调用:    agent_runner_loop 只传纯文本                        [❌ 图片未注入 messages]

Vision 工具:
  vision_api.template.py:  独立模块，支持 Claude/OpenAI/ModelScope [✅ 可用但未集成]
  ui_detect.py:            本地 YOLO+OCR                            [✅ 传统 CV]
  ocr_utils.py:            本地 RapidOCR                            [✅ 传统 OCR]
```

### 改进建议

1. **修复 agent_runner_loop**: 增加对图片参数的支持，在构建第一个 user message 时，若 `images` 非空，使用 `content: [{type: "text", text: prompt}, {type: "image", source: {type: "base64", ...}}]` 格式

2. **统一前端图片传递**: 各前端（QT/TG/Web）统一调用 `put_task(query, images=[...])`，其中 images 为标准化的图片 dict 列表: `[{"media_type": "image/png", "data": "<base64>"}]`

3. **集成 vision_api**: 将 `vision_api.template.py` 的逻辑集成到 agent_runner_loop 中，或作为 Agent 可调用的工具，而非独立模块

4. **图片预处理策略**: 参考 vision_api.template.py 的 `_prepare_image()` 实现，在大图场景下自动缩放（限制 max_pixels），避免 token 浪费

5. **对齐 OpenFang 设计**: OpenFang 的 `ContentBlock::Image` 设计值得借鉴——让图片成为消息内容的一等公民，而非嵌入文本或单独处理

6. **TG 前端的 Vision 支持**: 在 `handle_photo()` 中，将下载的图片进行 base64 编码并通过 images 参数传递给 put_task，而非仅保存路径

---

## 附录：涉及文件清单

| 文件 | 行数 | 关键行号 | 角色 |
|------|------|----------|------|
| `agentmain.py` | 290 | 107-110, 128-176 | put_task 接口定义 + 任务消费（images 未使用） |
| `agent_loop.py` | - | 42-46 | agent_runner_loop 只接受纯文本 user_input |
| `llmcore.py` | 1040 | 452-460, 472-515 | image_url/Claude image block 格式转换（就绪但未触发） |
| `frontends/qtapp.py` | 2478 | 460-517, 1903-1928, 1981-2028 | QT 图片选择 + base64 嵌入 prompt |
| `frontends/tgapp.py` | 1138 | 1040-1061 | TG 图片下载到磁盘（无 vision） |
| `frontends/desktop_bridge.py` | 613 | 182-203, 316-380, 536-541 | Web 图片保存 + [image:path] 标签 |
| `frontends/conductor.py` | 427 | 31-33, 175-190 | 无图片支持 |
| `frontends/chatapp_common.py` | 352 | 319-345 | AgentChatMixin.run_agent() 不传 images |
| `memory/vision_api.template.py` | 113 | 19-111 | 独立 Vision API 封装（3 后端） |
| `memory/ui_detect.py` | 119 | 63-99 | YOLO+OCR UI 检测 |
| `memory/ocr_utils.py` | 100 | 42-92 | 本地 OCR 工具 |
| `openfang/crates/openfang-types/src/message.rs` | 617 | 84-105, 170-195 | ContentBlock::Image 定义 + 验证 |
| `openfang/CHANGELOG.md` | - | 18, 155-160 | 多模态消息合并 + Image Support 记录 |
| `openfang/docs/architecture.md` | - | 361-365 | 3 驱动图片映射 |
| `openfang/docs/api-reference.md` | - | 2146-2151 | 多模态 API 示例 |
| `openfang/docs/troubleshooting.md` | - | 472 | image_url 使用说明 |
