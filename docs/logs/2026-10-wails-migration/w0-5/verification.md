# W0-5 verification — UI vs RPC payloads

Ground truth captured with `turnprobe.exe` (embedded host, same dispatch
table) into `rpc-dump.log`: `initialize`, `settings/get`,
`settings/providers`, `tools/list`, `session/*`, `review/list`,
`approval/list`, `question/list`, `run/subscribe`. UI state captured via
WebView2 CDP DOM dumps (`cdp-grants-dom.log`) and screenshots.

## W5-T2-GRANTS-CATALOG

| UI surface | Observed | RPC payload match |
|---|---|---|
| Providers list (设置→供应商) | 302.AI, AiHubMix, AIOnly, Anthropic, Baidu Qianfan, Baichuan, BurnCloud, Cephalon, Cerebras, DashScope, DeepSeek (openai-completions + anthropic-messages), Doubao (Ark), Fireworks AI, Gemini, GitHub Models, Groq, Hunyuan, Hyperbolic, Infini/Maas, Jina AI, Lanyun, LongCat, Mimo, MiniMax, Mistral AI, ModelScope, Moonshot, NVIDIA NIM, OcoolAI, OpenAI ×2, OpenRouter, PH8, PPIO, Perplexity, Poe, Qiniu, SenseNova, SiliconFlow, StepFun, Together AI, TokenFlux, vLLM/Local, Voyage AI, Yi, xAI, Zhipu AI + adapter-family labels | `settings/providers` catalog vendors + endpoints adapter — 1:1 (providers-catalog.png, cdp-grants-dom.log) |
| Endpoint card | API BASE URL `https://api.302.ai/v1`, models deepseek-chat/deepseek-reasoner/chatgpt-4o-latest/o3/claude-sonnet-4-20250514, 缺失字段 api_key | matches the `302ai` catalog entry exactly |
| Active provider | 当前供应商 deepseek / deepseek-flash | `settings/get` `config_provider:"deepseek"`, `config_model:"deepseek-flash"` |
| Permission preset chip | dropdown 谨慎/智能/信任 with 智能 armed (preset-dropdown.png) | `session.permission_preset:"smart"`, `sandbox.default_preset:"smart"` |
| Sandbox view | 拒绝私有 IP on, 允许的域名 [], 审批超时 300s, 工作区根目录 `C:\Users\Administrator\.vivy\workspace`, 允许执行的命令 `go, git, rg` (sandbox-runtime.png) | `deny_private_ips:true`, `allowed_domains:[]`, `approval_timeout_seconds:300`, `workspace_root`, `execute_allowed_commands:["go","git","rg"]` |
| Network tools | 网络 card → 配置网页搜索与抓取工具 | `tools/list` `network_search`/`web_fetch`/`http_request` active flags + `settings/get.network_search` provider roster (bing/google/duckduckgo/searxng/wikipedia, keyless flags) |
| Module-action catalog | sealed `generation.json` in the artifact lists every `std/control-action@v1` provides entry (20 `diva.cognitive.*`, 7 `vivy.masks.*`) | governance allowlist in the shell-written `vivy.yaml` grants exactly the face-owned writes; everything else flows through approval/deny by design |
| Denied/approval boundaries | `tools/list` declares it per tool — `write_note`/`write_file`/`patch`/`multiedit`/`execute`/`commandline`/`bash`/`download`/`skill_manage` are `readonly:false` (approval-gated; bash additionally deny-lists destructive patterns) | payload verbatim in rpc-dump.log |

No hardcoded provider/tool list was found in the UI — every item traced to a
live RPC payload.

## W5-T3-COGNITIVE

- **人格 (persona view, persona-view.png):** `人格状态: ready` —
  `diva.cognitive.status`; doc tabs 身份/关系/红线/用户/世界/梦境/暗面/使命
  with revisions (身份 r1…, IDENTITY.MD rev 1 content rendered) —
  `persona.read` projection; 评审 `暂无变更请求` — `persona.reviews.list`;
  冻结核心 (本会话) `not_captured` — `frozen.read` per-session state (the
  frozen_core_sessions row for the earlier GUI session was written by the
  lazy capture proven in w0-4); ACTMEM `暂无条目` — `actmem.read`;
  所有者文档 `laputa.actmem/v2 rev 0` — `actmem.owner.read`.
- **记忆 (memory view, memory-view.png):** scope badge
  `{"subject_id":"diva","kind":"personal","workspace_id":""}` → mentle;
  检索 with limit → `暂无卡片` (`memory.search`, empty explicit);
  变更 JSON editor + 提交变更 (`memory.mutate`); 回执查询 (`memory.receipt`).
- **演化 (evolution view, evolution-view.png):** 认知状态 已启用 false,
  来源 `diva/scope/v1:personal:…/mentle`, 水位线 0 — `diva.cognitive.status`;
  controls 立即触发 (trigger), 取消活动运行 (cancel), 触发策略 启用认知 +
  最小间隔 + 保存策略 (`policy.set`); 结果 `暂无结果` — `results.list`.
- **Explicit outcomes (not silent):**
  - empty mutate → `invalid_request: invalid_schema: mutation record_id required`
  - `{"operation":"put"}` → `invalid_request: invalid_schema: unknown mutation operation`
  - valid create mutation → `unavailable: selected memory backend unavailable:
    backend_unavailable: no memory backend for scope` (memory-mutate-unavailable.png)
    — the business-envelope status reaches the UI verbatim. The sealed build
    has no mentle write backend bound for this scope; the surface reports it
    explicitly rather than hanging or fabricating success.
- **Turn-side link:** first GUI turn writes `frozen_core_sessions`
  (sess_4d13a567743cd881 @ 08:36:22Z, w0-4); turns complete and persist —
  the cognitive gate is open end-to-end.

## Notes

- turnprobe runs against an embedded host print `control actions disabled:
  sealed Generation identity unavailable` — module-action RPC probing requires
  the sealed exe; cognitive evidence here is therefore UI-driven against the
  real sealed host plus DB verification from w0-4.
