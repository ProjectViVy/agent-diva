# Hermes Agent 图片/Vision 实现方式分析报告

> 分析日期: 2026-06-01
> 源码目录: `.workspace/hermes-agent/`

---

## 1. 消息 Content 类型体系

### 1.1 OpenAI 兼容的多模态 Content 格式

Hermes 内部统一使用 OpenAI 风格的 `content` 数组来表示多模态消息：

```json
{
  "role": "user",
  "content": [
    {"type": "text", "text": "用户文本"},
    {"type": "image_url", "image_url": {"url": "data:image/png;base64,..."}},
    {"type": "image_url", "image_url": {"url": "https://example.com/img.png"}}
  ]
}
```

**文件:** `agent/image_routing.py:418-504` — `build_native_content_parts()` 函数构建此格式。

### 1.2 两种消息格式并存

Hermes 支持两种 content 形式：
- **纯文本 (str)**: 简单的 `"content": "text"` 字符串
- **内容数组 (list)**: `"content": [{"type": "text", ...}, {"type": "image_url", ...}]`

CLI 和 Gateway 入口根据 `decide_image_input_mode()` 决定使用哪种格式。

**文件:** `cli.py:11838-11870`, `gateway/run.py:8190-8209`

### 1.3 Provider 层面的翻译

不同 provider 适配器将 OpenAI 格式翻译为各自的原生格式：

| Provider | 适配器文件 | 关键行号 |
|----------|-----------|---------|
| Anthropic | `agent/anthropic_adapter.py:1510-1523` | `image_url` → `{"type": "image", "source": {...}}` |
| OpenAI Chat | 原生支持 | 无需翻译 |
| Codex Responses | `agent/chat_completion_helpers.py:186-197` | `image_url` → `input_image` |
| Gemini Native | `agent/gemini_native_adapter.py` | 自有翻译逻辑 |
| Bedrock Converse | `agent/chat_completion_helpers.py:198-222` | 自有翻译逻辑 |

**关键代码路径:**
- `agent/anthropic_adapter.py:1514-1517` — Anthropic 的 `image_url` → `image` block 翻译

### 1.4 Tool Result 中的多模态支持

Vision tool 可以在 tool result 中返回 `_multimodal` 信封：

```json
{
  "_multimodal": true,
  "content": [
    {"type": "text", "text": "Image loaded..."},
    {"type": "image_url", "image_url": {"url": "data:image/..."}}
  ],
  "text_summary": "..."
}
```

**文件:** `tools/vision_tools.py:552-604` — `_build_native_vision_tool_result()`

`_supports_media_in_tool_results()` (`tools/vision_tools.py:462-519`) 白名单列出支持 tool result 中携带图片的 provider。

---

## 2. 图片附件的处理流程

### 2.1 整体架构图

```
平台消息 → Gateway/CLI 入口 → 图片路由决策 → 模型/API 发送
                  │
                  ├── native 模式: build_native_content_parts() → content 数组 → provider 适配器
                  │
                  └── text 模式: _enrich_message_with_vision() → vision_analyze → 文本描述
```

### 2.2 Gateway 端（平台消息 → Agent）

**步骤 1: 下载用户图片**

平台适配器（Telegram, Discord, WhatsApp 等）从各自 API 下载用户发送的图片到本地缓存：

- `gateway/platforms/base.py:614-670` — `cache_image_from_url()` 通用图片下载（含 SSRF 防护）
- `gateway/platforms/telegram.py:3907-3998` — Telegram 的 `send_image_file()` 实现
- `gateway/platforms/telegram.py:4098-4190` — Telegram 的 `send_image()` 实现（URL 和文件上传回退）
- `gateway/platforms/weixin.py:1544` — 微信的 `_download_image()` 实现
- `gateway/platforms/feishu.py:3640` — 飞书的 `_download_feishu_image()` 实现
- `gateway/platforms/bluebubbles.py:704` — BlueBubbles 的 `_download_attachment()` 实现

**步骤 2: 图片路由决策**

Gateway 在处理用户消息时，调用 `_decide_image_input_mode()` 决定路由：

**文件:** `gateway/run.py:15088-15109`
```
1. 读取 agent.image_input_mode 配置 (auto/native/text)
2. 若 native 或模型支持 vision → native 模式
3. 否则 → text 模式 (vision_analyze 预处理)
```

**步骤 3a: native 模式（直接附加图片 pixels）**

**文件:** `gateway/run.py:8191-8197`
- 将图片路径存储在 `_pending_native_image_paths_by_session` 中
- 后续 `run_conversation()` 调用 `build_native_content_parts()` 构建 content 数组
- 图片以 base64 data URL 或 HTTP URL 形式嵌入

**步骤 3b: text 模式（vision_analyze 预处理为文本）**

**文件:** `gateway/run.py:15111-15178` — `_enrich_message_with_vision()`
- 对每张图片调用 `vision_analyze_tool()` 进行 AI 描述
- 将描述文本 prepend 到用户消息之前
- 模型看不到像素，只看到文本描述

### 2.3 CLI 端（命令行 → Agent）

**文件:** `cli.py:11820-11870`

CLI 的处理逻辑与 Gateway 一致：
1. 调用 `decide_image_input_mode()` 决定模式
2. native 模式: 调用 `build_native_content_parts()` → message 变为 content 数组
3. text 模式: 调用 `_preprocess_images_with_vision()` 预处理

**TUI Gateway** 同样遵循此逻辑：`tui_gateway/server.py:3549-3573`

### 2.4 图片本地路径提取

用户消息中的本地图片路径会被自动识别并附加：

**文件:** `agent/image_routing.py:56-144` — `extract_image_refs()`
- 支持 `~/path/to/image.png` 和 `/absolute/path/to/image.jpg` 路径
- 支持 `http(s)://example.com/image.png` URL
- 跳过代码块内的路径（避免示例代码误触发）
- 文件系统验证（本地路径必须真实存在）

**文件:** `cli.py:15446-15493` — CLI 入口调用 `extract_image_refs()` 扫描用户输入

### 2.5 图片大小处理（Reactive 策略）

Hermes 采用**响应式**而非主动式的图片大小处理：

**策略说明:** `agent/image_routing.py:320-333`
- 不会预先按 provider 限制调整图片大小
- 先发送原图，若 provider 拒绝（如 Anthropic 的 5MB 限制返回 400），再缩小后重试

**文件:** `run_agent.py:3912-3989`
- `_try_shrink_image_parts_in_messages()` → `agent/conversation_compression.py:614-755`
- 将超过 4MB 的 base64 图片缩小（使用 Pillow）
- `_try_strip_image_parts_from_tool_messages()` 降级 list 型 tool message（不支持多模态的 provider）

**文件:** `tools/vision_tools.py:344-447` — `_resize_image_for_vision()`
- 使用 Pillow 逐步 halve 尺寸
- JPEG 额外尝试降低质量 (85, 70, 50)
- 最大 4 轮缩小

**大小限制常量:**
- `tools/vision_tools.py:316` — `_MAX_BASE64_BYTES = 20*1024*1024` (硬上限 20MB)
- `tools/vision_tools.py:327` — `_EMBED_TARGET_BYTES = 4*1024*1024` (嵌入历史目标 4MB)
- `tools/vision_tools.py:331` — `_RESIZE_TARGET_BYTES = 5*1024*1024` (重试目标 5MB)
- `tools/vision_tools.py:74` — `_VISION_MAX_DOWNLOAD_BYTES = 50*1024*1024` (下载硬上限 50MB)

---

## 3. 图片路由逻辑

### 3.1 核心决策函数

**文件:** `agent/image_routing.py:287-317` — `decide_image_input_mode(provider, model, cfg)`

决策流程：
```
1. 读取 agent.image_input_mode 配置 (默认 "auto")
2. 若 "native" → 返回 "native"
3. 若 "text" → 返回 "text"
4. 若 "auto":
   a. 检查 auxiliary.vision 是否显式配置 → 是 → "text"
   b. 查询模型 supports_vision 能力:
      - 先检查 config.yaml 中的 supports_vision override
      - 再查询 models.dev 数据库
      - 若 True → "native"
      - 否则 → "text"
```

### 3.2 配置项

**文件:** `hermes_cli/config.py:777` — 默认配置
```yaml
agent:
  image_input_mode: "auto"  # "auto" | "native" | "text"
```

### 3.3 supports_vision 查询

**文件:** `agent/image_routing.py:260-284` — `_lookup_supports_vision()`

查询优先级：
1. `model.supports_vision` 顶层快捷键
2. `providers.<provider>.models.<model>.supports_vision` 按模型配置
3. `agent/models_dev.py:450-503` — `get_model_capabilities()` → models.dev 数据库

**文件:** `agent/models_dev.py:401-408` — `ModelCapabilities` dataclass
```python
@dataclass
class ModelCapabilities:
    supports_tools: bool = True
    supports_vision: bool = False    # <-- 关键字段
    supports_reasoning: bool = False
    context_window: int = 200000
    max_output_tokens: int = 8192
    model_family: str = ""
```

### 3.4 auxiliary.vision 的显式配置检测

**文件:** `agent/image_routing.py:235-257` — `_explicit_aux_vision_override()`
- 若 `auxiliary.vision.provider` 非空且非 "auto"，或有 `model`/`base_url` → 视为显式配置
- 显式配置意味着用户想要 text pipeline（用专门的 vision 模型）
- 此时即使主模型支持 vision，也使用 text 模式

### 3.5 Computer Use 的视觉路由

**文件:** `tools/computer_use/vision_routing.py:118-147` — `should_route_capture_to_aux_vision()`
- 专门处理 computer_use 截图的路由
- 与 user-attached image 的路由逻辑平行但独立
- 同样检查 `auxiliary.vision` 显式配置和主模型能力

---

## 4. 图片生成的 Provider 抽象

### 4.1 ImageGenProvider ABC

**文件:** `agent/image_gen_provider.py:51-143`

```python
class ImageGenProvider(abc.ABC):
    @property
    @abc.abstractmethod
    def name(self) -> str: ...

    @property
    def display_name(self) -> str: ...    # 默认 name.title()

    def is_available(self) -> bool: ...   # 默认 True

    def list_models(self) -> List[Dict]: ...  # 默认 []

    def get_setup_schema(self) -> Dict: ...    # hermes tools 选择器元数据

    def default_model(self) -> Optional[str]: ...

    @abc.abstractmethod
    def generate(self, prompt: str, aspect_ratio: str, **kwargs) -> Dict: ...
```

### 4.2 辅助函数

**文件:** `agent/image_gen_provider.py:146-324`

| 函数 | 行号 | 功能 |
|------|------|------|
| `resolve_aspect_ratio()` | 151-162 | 规范化宽高比 (landscape/square/portrait) |
| `save_b64_image()` | 174-191 | 将 base64 图片保存到 `$HERMES_HOME/cache/images/` |
| `save_url_image()` | 207-273 | 下载 URL 图片并缓存 (处理临时 URL 过期问题) |
| `success_response()` | 276-302 | 构建统一的成功响应 dict |
| `error_response()` | 305-324 | 构建统一的错误响应 dict |

### 4.3 Provider 注册表

**文件:** `agent/image_gen_registry.py`

| 函数 | 行号 | 功能 |
|------|------|------|
| `register_provider()` | 36-57 | 注册 provider，支持热重载 |
| `list_providers()` | 60-64 | 列出所有已注册 provider |
| `get_provider()` | 67-72 | 按名称查找 provider |
| `get_active_provider()` | 75-139 | 解析当前活跃的 provider，fallback 逻辑 |

**活跃 Provider 选择策略** (`agent/image_gen_registry.py:75-139`):
1. `image_gen.provider` 在 config.yaml 中显式配置 → 使用它（即使 unavailable）
2. 仅有一个已注册 provider 且 available → 使用它
3. `fal` provider 已注册且 available → 向后兼容 legacy 默认
4. 否则返回 None → 工具提示用户配置

### 4.4 内置 Provider 插件

**目录:** `plugins/image_gen/`

| Provider | 文件 | 说明 |
|----------|------|------|
| OpenAI | `plugins/image_gen/openai/__init__.py` | gpt-image-2 (low/medium/high 三档) |
| xAI | `plugins/image_gen/xai/__init__.py` | grok-imagine-image |
| FAL | `plugins/image_gen/fal/__init__.py` | 传统 FAL.ai 后端 |
| Codex | `plugins/image_gen/openai-codex/__init__.py` | Codex Responses API 图片生成 |
| Krea | `plugins/image_gen/krea/__init__.py` | Krea 2 图片生成 |

### 4.5 图片生成工具

**文件:** `tools/image_generation_tool.py`

通过 FAL.ai 提供多个模型。FAL 模型目录 (`tools/image_generation_tool.py:97-369`):

| 模型 ID | 显示名 | 速度 |
|---------|--------|------|
| `fal-ai/flux-2/klein/9b` | FLUX 2 Klein 9B | <1s |
| `fal-ai/flux-2-pro` | FLUX 2 Pro | ~6s |
| `fal-ai/z-image/turbo` | Z-Image Turbo | ~2s |
| `fal-ai/nano-banana-pro` | Nano Banana Pro (Gemini) | ~8s |
| `fal-ai/gpt-image-1.5` | GPT Image 1.5 | ~15s |
| `fal-ai/gpt-image-2` | GPT Image 2 | ~20s |
| `fal-ai/ideogram/v3` | Ideogram V3 | ~5s |
| `fal-ai/recraft/v4/pro/...` | Recraft V4 Pro | ~8s |
| `fal-ai/qwen-image` | Qwen Image | ~12s |
| `fal-ai/krea/v2/medium/...` | Krea 2 Medium | ~15-25s |
| `fal-ai/krea/v2/large/...` | Krea 2 Large | ~25-60s |

三种尺寸规格风格 (`tools/image_generation_tool.py:83-89`):
- `image_size_preset` — FLUX 系列 (square_hd, landscape_16_9, ...)
- `aspect_ratio` — Gemini via FAL (1:1, 16:9, ...)
- `gpt_literal` — GPT Image 1.5 (1024x1024, 1536x1024, ...)

---

## 5. 各平台 (Channel) 的图片处理差异

### 5.1 BaseAdapter 通用接口

**文件:** `gateway/platforms/base.py`

| 方法 | 行号 | 功能 |
|------|------|------|
| `send_image()` | 2357 | 通过 URL 发送图片 |
| `send_image_file()` | 2530 | 通过本地文件路径发送图片 |
| `send_multiple_images()` | 2300 | 批量发送多张图片 |
| `send_animation()` | 2389 | 发送动画/GIF（默认 fallback 到 send_image） |
| `extract_local_files()` | 2645 | 从响应文本中提取本地文件路径（含图片） |

### 5.2 平台实现概览

| 平台 | 文件 | send_image 行号 | send_image_file 行号 | 特点 |
|------|------|-----------------|---------------------|------|
| Telegram | `gateway/platforms/telegram.py` | 4098 | 3907 | 使用 Bot API send_photo，支持 URL+文件上传双路径回退，支持 media group 多图 |
| Discord | `gateway/platforms/discord.py` | (通道消息附件机制) | — | 通过 attachment 机制处理 |
| WhatsApp | `gateway/platforms/whatsapp.py` | 1048 | 1062 | delegate 到 super().send_image() |
| WeChat | `gateway/platforms/weixin.py` | 1839 | 1864 | 微信服务器上传+Media ID 机制 |
| WeCom | `gateway/platforms/wecom.py` | 1399 | 1422 | 企业微信图片消息 |
| DingTalk | `gateway/platforms/dingtalk.py` | 933 | 957 | 钉钉图片方法 |
| Feishu | `gateway/platforms/feishu.py` | 2162 | 2097 | 飞书图片+远程图片下载 `_download_remote_image` |
| Signal | `gateway/platforms/signal.py` | 1240 | 1336 | Signal 图片消息 |
| Slack | `gateway/platforms/slack.py` | 1412 | 1386 | Slack files.upload 机制 |
| Matrix | `gateway/platforms/matrix.py` | 1151 | 1206 | Matrix 协议 mxc URI |
| BlueBubbles | `gateway/platforms/bluebubbles.py` | 536 | 552 | iMessage 图片 |
| Email | `gateway/platforms/email.py` | 568 | 580 | 邮件附件方式 |
| QQ Bot | `gateway/platforms/qqbot/adapter.py` | 2745 | 2771 | QQ 机器人图片消息 |
| Yuanbao | `gateway/platforms/yuanbao.py` | 4823 | 4839 | 元宝平台 |
| API Server | `gateway/platforms/api_server.py` | — | — | 通过 HTTP multipart 处理图片 |

### 5.3 图片下载流程（各平台 → Hermes 缓存）

通用模式：平台适配器下载用户图片 → 缓存到 `$HERMES_HOME/cache/images/`

**文件:** `gateway/platforms/base.py:614-670` — `cache_image_from_url()` (通用下载)

各平台的下载函数：
- Telegram: `telegram.py` (通过 python-telegram-bot 获取 file_path)
- WeChat: `weixin.py:1544` — `_download_image()`
- Feishu: `feishu.py:3640` — `_download_feishu_image()`
- BlueBubbles: `bluebubbles.py:704` — `_download_attachment()`

### 5.4 图片发送流程（Hermes → 各平台）

响应文本中的本地文件路径被自动识别并发送：

**文件:** `gateway/platforms/base.py:2645-2716` — `extract_local_files()`
- 正则匹配 `~/path/to/file.png` 和 `/absolute/path/to/file.jpg`
- 跳过代码块内的路径
- 文件系统验证
- 自动从响应文本中移除路径字符串

**文件:** `gateway/platforms/base.py:2300-2357` — `send_multiple_images()`
- 自动将图片路径分发到 `send_image_file()` 或 `send_image()`

---

## 6. Vision 分析 Tool（vision_analyze）

### 6.1 两种模式

**文件:** `tools/vision_tools.py:716-997` — `vision_analyze_tool()`

1. **传统 auxiliary LLM 模式**: 下载图片 → base64 → 调用辅助 vision 模型 → 返回文本描述
2. **Native fast path**: 直接将图片附加到 tool result → 主模型"看到"像素

### 6.2 Native Fast Path

**文件:** `tools/vision_tools.py:462-549` — `_should_use_native_vision_fast_path()` 和辅助函数

触发条件：
1. `decide_image_input_mode()` 返回 "native"
2. 且 provider 在 `_supports_media_in_tool_results()` 白名单中或用户显式声明 `supports_vision`

白名单 provider (`tools/vision_tools.py:462-519`):
- **Aggregators**: openrouter, nous, vertex, bedrock, anthropic-vertex, google-vertex
- **直接 API**: anthropic, claude, anthropic-direct
- **OpenAI**: openai, openai-chat, openai-codex, azure-openai
- **Gemini**: google, gemini, google-gemini, google-vertex-gemini (仅 gemini-3 系列)

### 6.3 Vision 分析下载

**文件:** `tools/vision_tools.py:154-262` — `_download_image()`
- 异步下载，最多 3 次重试
- SSRF 防护（预检 + redirect guard）
- 大小限制: 50MB
- 支持本地文件路径和 HTTP/HTTPS URL

---

## 7. 关键文件索引

### 核心图片路由
| 文件 | 关键内容 | 行号 |
|------|---------|------|
| `agent/image_routing.py` | `decide_image_input_mode()` | 287-317 |
| `agent/image_routing.py` | `build_native_content_parts()` | 418-504 |
| `agent/image_routing.py` | `extract_image_refs()` | 78-144 |
| `agent/image_routing.py` | `_lookup_supports_vision()` | 260-284 |
| `agent/image_routing.py` | `_explicit_aux_vision_override()` | 235-257 |
| `agent/image_routing.py` | `_supports_vision_override()` | 176-222 |
| `agent/image_routing.py` | `_sniff_mime_from_bytes()` | 335-367 |
| `agent/image_routing.py` | `_file_to_data_url()` | 396-415 |

### 图片生成
| 文件 | 关键内容 | 行号 |
|------|---------|------|
| `agent/image_gen_provider.py` | `ImageGenProvider` ABC | 51-143 |
| `agent/image_gen_provider.py` | `save_b64_image()` | 174-191 |
| `agent/image_gen_provider.py` | `save_url_image()` | 207-273 |
| `agent/image_gen_provider.py` | `success_response()` / `error_response()` | 276-324 |
| `agent/image_gen_registry.py` | `get_active_provider()` | 75-139 |
| `agent/image_gen_registry.py` | `register_provider()` | 36-57 |
| `tools/image_generation_tool.py` | FAL_MODELS catalog | 97-369 |
| `tools/image_generation_tool.py` | `_submit_fal_request()` | 433-473 |

### Vision 工具
| 文件 | 关键内容 | 行号 |
|------|---------|------|
| `tools/vision_tools.py` | `vision_analyze_tool()` | 716-997 |
| `tools/vision_tools.py` | `_should_use_native_vision_fast_path()` | 522-549 |
| `tools/vision_tools.py` | `_build_native_vision_tool_result()` | 552-604 |
| `tools/vision_tools.py` | `_supports_media_in_tool_results()` | 462-519 |
| `tools/vision_tools.py` | `_resize_image_for_vision()` | 344-447 |
| `tools/vision_tools.py` | `_download_image()` | 154-262 |

### Provider 适配器（图片翻译）
| 文件 | 关键内容 | 行号 |
|------|---------|------|
| `agent/anthropic_adapter.py` | `image_url` → `image` block | 1510-1523 |
| `agent/anthropic_adapter.py` | tool_result 过滤 | 1607 |

### 图片大小处理
| 文件 | 关键内容 | 行号 |
|------|---------|------|
| `agent/conversation_compression.py` | `try_shrink_image_parts_in_messages()` | 614-755 |
| `run_agent.py` | `_try_shrink_image_parts_in_messages()` (forwarder) | 3912-3915 |
| `run_agent.py` | `_try_strip_image_parts_from_tool_messages()` | 3917-3989 |

### Gateway 图片处理
| 文件 | 关键内容 | 行号 |
|------|---------|------|
| `gateway/run.py` | `_decide_image_input_mode()` | 15088-15109 |
| `gateway/run.py` | `_enrich_message_with_vision()` | 15111-15178 |
| `gateway/run.py` | 图片路由分发 | 8189-8209 |
| `gateway/platforms/base.py` | `cache_image_from_url()` | 614-670 |
| `gateway/platforms/base.py` | `extract_local_files()` | 2645-2716 |
| `gateway/platforms/base.py` | `send_image()` / `send_image_file()` | 2357 / 2530 |

### CLI 图片处理
| 文件 | 关键内容 | 行号 |
|------|---------|------|
| `cli.py` | 图片路由决策 + native 附加 | 11820-11870 |
| `cli.py` | `extract_image_refs()` 调用 | 15446-15493 |

### Computer Use 视觉路由
| 文件 | 关键内容 | 行号 |
|------|---------|------|
| `tools/computer_use/vision_routing.py` | `should_route_capture_to_aux_vision()` | 118-147 |

### 模型能力
| 文件 | 关键内容 | 行号 |
|------|---------|------|
| `agent/models_dev.py` | `ModelCapabilities` dataclass | 401-408 |
| `agent/models_dev.py` | `get_model_capabilities()` | 450-503 |
| `agent/models_dev.py` | `supports_vision` 推导逻辑 | 459-485 |

### 配置
| 文件 | 关键内容 | 行号 |
|------|---------|------|
| `hermes_cli/config.py` | `image_input_mode: "auto"` 默认值 | 777 |

---

## 8. 总结

Hermes Agent 的图片/vision 实现是一个**双模式**系统：

1. **Native 模式**: 图片以 base64 data URL 或 HTTP URL 直接附加在 OpenAI-style content 数组的 `image_url` part 中。Provider 适配器自主翻译为各自的原生格式（Anthropic image block、Codex input_image 等）。图片大小采用**响应式策略**——先发送原图，被拒绝后缩小重试。

2. **Text 模式**: 图片先通过 `vision_analyze_tool` 用辅助 vision 模型分析成文本描述，再 prepend 到用户消息前。模型永远看不到像素，只看到文本摘要。

3. **路由决策** 由 `decide_image_input_mode()` 统一管理，基于 config.yaml 的 `agent.image_input_mode` + 模型 `supports_vision` 能力 + `auxiliary.vision` 显式配置三者综合判断。

4. **图片生成** 通过 `ImageGenProvider` ABC 抽象，使用注册表模式，支持多个后端（FAL、OpenAI、xAI、Krea、Codex）。FAL 通过 `tools/image_generation_tool.py` 提供 11 种模型的统一接入。

5. **平台适配** 各 gateway 平台适配器各自实现 `send_image()` / `send_image_file()`，处理平台特定的图片上传/下载 API。
