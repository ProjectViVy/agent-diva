# GenericAgent 鍗囩骇杩涘寲璋冪爺鎶ュ憡

## 鐗堟湰淇℃伅
- 鐗堟湰鍙? v0.0.1-initial-research
- 璋冪爺鏃ユ湡: 2026-04-24
- 璋冪爺鐩爣: 鍒嗘瀽 .workspace/GenericAgent 鏋舵瀯锛岃瘎浼板 agent-diva 涓?agent 鐨勫崌绾у€熼壌浠峰€?

---

## 涓€銆丟enericAgent 鏋舵瀯姒傝

### 1.1 鏍稿績璁捐鍝插

GenericAgent 鐨勬牳蹇冪悊蹇垫槸锛?*涓嶉璁炬妧鑳斤紝闈犺繘鍖栬幏寰楄兘鍔?*銆傛瘡瑙ｅ喅涓€涓柊浠诲姟锛岀郴缁熻嚜鍔ㄥ皢鎵ц璺緞鍥哄寲涓?Skill锛屼緵鍚庣画鐩存帴璋冪敤銆備娇鐢ㄦ椂闂磋秺闀匡紝娌夋穩鐨勬妧鑳借秺澶氾紝褰㈡垚涓撳睘鎶€鑳芥爲銆?

### 1.2 浠ｇ爜缁撴瀯

GenericAgent 閲囩敤鏋佺畝鏋舵瀯锛屾牳蹇冧粎绾?**~3K 琛屼唬鐮?*锛?

| 鏂囦欢 | 琛屾暟浼扮畻 | 鍔熻兘 |
|------|----------|------|
| `agentmain.py` | ~260 琛?| Agent 涓诲叆鍙ｃ€丩LM Session 绠＄悊銆佷换鍔￠槦鍒?|
| `agent_loop.py` | ~122 琛?| 鏍稿績 Agent Loop锛堢害鐧捐锛?|
| `ga.py` | ~559 琛?| Handler 瀹炵幇 + 9 涓師瀛愬伐鍏峰疄鐜?|
| `llmcore.py` | ~919 琛?| LLM Session 鎶借薄灞傦紙澶?Provider 鏀寔锛?|
| `simphtml.py` | ~850 琛?| HTML 绠€鍖栧鐞嗭紙娴忚鍣ㄥ唴瀹规彁鍙栵級 |
| `TMWebDriver.py` | ~350 琛?| 娴忚鍣?CDP 鎺у埗灞?|

### 1.3 鏍稿績缁勪欢

```
鈹屸攢鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹?
鈹?                   GenericAgent Architecture                 鈹?
鈹溾攢鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹?
鈹?                                                             鈹?
鈹? 鈹屸攢鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹?    鈹屸攢鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹?    鈹屸攢鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹?鈹?
鈹? 鈹? Frontends   鈹傗攢鈹€鈹€鈹€鈻垛攤  agentmain   鈹傗攢鈹€鈹€鈹€鈻垛攤  agent_loop  鈹?鈹?
鈹? 鈹?(澶氭笭閬撴帴鍏? 鈹?    鈹? (浠诲姟璋冨害)  鈹?    鈹? (鏍稿績寰幆)  鈹?鈹?
鈹? 鈹斺攢鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹?    鈹斺攢鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹?    鈹斺攢鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹?鈹?
鈹?                             鈹?                     鈹?       鈹?
鈹?                             鈻?                     鈻?       鈹?
鈹?                      鈹屸攢鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹?    鈹屸攢鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹?鈹?
鈹?                      鈹? llmcore     鈹?    鈹? ga.py       鈹?鈹?
鈹?                      鈹?(LLM Session)鈹?    鈹?(Handler+Tools)鈹?
鈹?                      鈹斺攢鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹?    鈹斺攢鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹?鈹?
鈹?                             鈹?                     鈹?       鈹?
鈹?                             鈻?                     鈻?       鈹?
鈹?                      鈹屸攢鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹?    鈹屸攢鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹?鈹?
鈹?                      鈹? Memory      鈹?    鈹? TMWebDriver 鈹?鈹?
鈹?                      鈹?(鍒嗗眰璁板繂)   鈹?    鈹?(娴忚鍣ㄦ帶鍒? 鈹?鈹?
鈹?                      鈹斺攢鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹?    鈹斺攢鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹?鈹?
鈹?                                                             鈹?
鈹斺攢鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹?
```

---

## 浜屻€佸垎灞傝蹇嗙郴缁燂紙鏍稿績浜偣锛?

### 2.1 璁板繂灞傜骇鏋舵瀯

GenericAgent 鐨勫垎灞傝蹇嗙郴缁熸槸鍏舵渶鍏抽敭鐨勮璁★細

```
L1: global_mem_insight.txt (鏋佺畝绱㈠紩灞?- 鈮?0 琛?
    鈫?瀵艰埅鎸囧悜
L2: global_mem.txt (浜嬪疄搴撳眰 - 鐜浜嬪疄)
    鈫?璇︾粏寮曠敤
L3: ../memory/*.md/*.py (浠诲姟绾ц褰曞簱 - SOP + 宸ュ叿鑴氭湰)
    鈫?闀跨▼鍙洖
L4: ../memory/L4_raw_sessions/ (浼氳瘽褰掓。灞?- 鍘嗗彶浼氳瘽)
```

### 2.2 鍚勫眰鑱岃矗璇﹁В

| 灞傜骇 | 鑱岃矗 | 鐗瑰緛 | 鏇存柊鏃舵満 |
|------|------|------|----------|
| L0 | 鍏冭鍒欙紙闅愬惈锛?| 鏍稿績琛屼负瑙勫垯銆佺郴缁熺害鏉?| 闈欐€?|
| L1 | 璁板繂绱㈠紩 | 鈮?0琛岋紝<1k tokens锛屽満鏅叧閿瘝鈫掕蹇嗗畾浣?| L2/L3 鍙樺寲鏃跺悓姝?|
| L2 | 鍏ㄥ眬浜嬪疄 | 鐜鐗瑰紓鎬т簨瀹烇紙璺緞銆佸嚟璇併€侀厤缃級 | 鍙戠幇鏂颁簨瀹炴椂 |
| L3 | 浠诲姟 Skills/SOPs | 鐗瑰畾浠诲姟鍙鐢ㄦ祦绋?| 浠诲姟瀹屾垚鍚庢矇娣€ |
| L4 | 浼氳瘽褰掓。 | 宸插畬鎴愪换鍔＄殑鎻愮偧璁板綍 | 浠诲姟缁撴潫鍚庡綊妗?|

### 2.3 璁板繂鍐欏叆鍏悊

**鏍稿績鍘熷垯**锛?
1. **琛屽姩楠岃瘉鍘熷垯**: 鍙啓鍏ョ粡杩囧伐鍏疯皟鐢ㄩ獙璇佺殑淇℃伅
2. **绁炲湥涓嶅彲鍒犳敼**: 楠岃瘉杩囩殑鏈夋晥閰嶇疆/閬垮潙鎸囧崡涓嶅彲涓㈠純
3. **绂佹鏄撳彉鐘舵€?*: 涓嶅瓨鏃堕棿鎴炽€丼ession ID銆丳ID 绛夋槗鍙樻暟鎹?
4. **鏈€灏忓厖鍒嗘寚閽?*: 涓婂眰鍙暀瀹氫綅涓嬪眰鐨勬渶鐭爣璇?

**淇℃伅鍒嗙被鍐崇瓥鏍?*锛?
```
鏄€庣幆澧冪壒寮傛€т簨瀹炪€? 鈫?L2 鈫?鎸夐鐜囧綊鍏?L1
鈫?鍚?
鏄€庨€氱敤鎿嶄綔瑙勫緥銆? 鈫?L1 RULES (1鍙ュ帇缂?
鈫?鍚?
鏄€庣壒瀹氫换鍔℃妧鏈€? 鈫?L3 (SOP/鑴氭湰)
鈫?鍚?
鍒ゅ畾涓恒€庨€氱敤甯歌瘑銆? 涓ョ瀛樺偍
```

---

## 涓夈€佽嚜涓绘墽琛屽惊鐜紙Agent Loop锛?

### 3.1 鏍稿績寰幆浠ｇ爜缁撴瀯

`agent_loop.py` 绾?100 琛屾牳蹇冧唬鐮佸疄鐜板畬鏁寸殑 Agent 寰幆锛?

```python
def agent_runner_loop(client, system_prompt, user_input, handler, tools_schema, max_turns=40):
    messages = [
        {"role": "system", "content": system_prompt},
        {"role": "user", "content": user_input}
    ]
    turn = 0
    while turn < handler.max_turns:
        turn += 1
        # 1. LLM 璋冪敤
        response = yield from client.chat(messages=messages, tools=tools_schema)
        # 2. 宸ュ叿璋冪敤瑙ｆ瀽
        tool_calls = parse_tool_calls(response)
        # 3. 宸ュ叿鎵ц
        for tc in tool_calls:
            outcome = yield from handler.dispatch(tool_name, args, response)
            if outcome.should_exit: break
        # 4. 鏋勫缓涓嬩竴杞?prompt
        messages = [{"role": "user", "content": next_prompt, "tool_results": tool_results}]
    return exit_reason
```

### 3.2 Handler 妯″紡

`BaseHandler` 瀹氫箟宸ュ叿鍥炶皟鎺ュ彛锛?

```python
class BaseHandler:
    def tool_before_callback(self, tool_name, args, response): pass
    def tool_after_callback(self, tool_name, args, response, ret): pass
    def turn_end_callback(self, response, tool_calls, tool_results, turn, next_prompt, exit_reason): ...
    def dispatch(self, tool_name, args, response):
        # 鑷姩璺敱鍒?do_{tool_name} 鏂规硶
        method_name = f"do_{tool_name}"
        if hasattr(self, method_name):
            ret = yield from getattr(self, method_name)(args, response)
            return ret
```

### 3.3 浜や簰鍗忚娉ㄥ叆

姣忚疆 prompt 鑷姩娉ㄥ叆锛?
- `<history>`: 鏈€杩戝璇濆巻鍙叉憳瑕?
- `<key_info>`: 宸ヤ綔璁板繂鍐呭
- `<summary>` 鍗忚瑕佹眰: 寮哄埗 LLM 杈撳嚭鍗曡鐗╃悊蹇収

---

## 鍥涖€佹渶灏忓伐鍏烽泦

### 4.1 9 涓師瀛愬伐鍏?

| 宸ュ叿 | 鍔熻兘 | 璁捐鍘熷垯 |
|------|------|----------|
| `code_run` | 鎵ц浠绘剰浠ｇ爜 | 浼樺厛 python锛宼imeout 鎺у埗 |
| `file_read` | 璇诲彇鏂囦欢 | 鏀寔鍏抽敭璇嶆悳绱€佽鍙峰畾浣?|
| `file_write` | 鍐欏叆鏂囦欢 | 浠呯敤浜庡ぇ鏂囦欢锛屼緷璧?`<file_content>` 鏍囩 |
| `file_patch` | 灞€閮ㄤ慨鏀?| 鍞竴鍖归厤鍘熷垯锛屽け璐ユ椂寤鸿鍏?read |
| `web_scan` | 鑾峰彇椤甸潰鍐呭 | 绠€鍖?HTML锛岃繃婊ら潪涓讳綋鍐呭 |
| `web_execute_js` | 鎵ц JS | 瀹屽叏鎺у埗娴忚鍣紝鏀寔缁撴灉淇濆瓨 |
| `ask_user` | 浜烘満鍗忎綔 | 涓柇寮忕‘璁?|
| `update_working_checkpoint` | 鏇存柊宸ヤ綔璁板繂 | 鐭湡渚跨锛岄槻淇℃伅涓㈠け |
| `start_long_term_update` | 鍚姩闀挎湡璁板繂娌夋穩 | 浠诲姟瀹屾垚鍚庤皟鐢?|

### 4.2 宸ュ叿璁捐鐗圭偣

1. **鍘熷瓙鎬?*: 姣忎釜宸ュ叿鍙仛涓€浠朵簨
2. **鍙粍鍚?*: 閫氳繃缁勫悎瀹屾垚澶嶆潅浠诲姟
3. **鍔ㄦ€佹墿灞?*: `code_run` 鍙姩鎬佸畨瑁呭寘銆佸啓鏂拌剼鏈€佽皟鐢?API

---

## 浜斻€侀珮绾фā寮?

### 5.1 Plan Mode锛堣鍒掓ā寮忥級

瑙﹀彂鏉′欢锛? 姝ヤ互涓娿€佹湁渚濊禆銆佸鏂囦欢鍗忓悓

娴佺▼锛?
1. **鎺㈢储鎬?*: 鍒涘缓鐩綍 鈫?鍚姩鎺㈢储 subagent 鈫?鐩戝療绛夊緟 鈫?鏀跺彇鍙戠幇
2. **瑙勫垝鎬?*: 璇?SOP 鈫?鍐?plan.md 鈫?鑷娓呭崟 鈫?鐢ㄦ埛纭
3. **鎵ц鎬?*: 寰幆鎵ц `[ ]` 鈫?Mini 楠岃瘉 鈫?鏍囪瀹屾垚
4. **楠岃瘉鎬?*: 鍚姩鐙珛 subagent 瀵规姉鎬ч獙璇?

### 5.2 Subagent 妯″紡

閫氫俊鍗忚锛?
- 鐩綍: `temp/{task_name}/`
- 杈撳叆: `input.txt` (鐩爣+绾︽潫锛岀鍐欐楠?
- 杈撳嚭: `output*.txt` (append锛宍[ROUND END]` = 杞畬鎴?
- 骞查: `_stop` | `_keyinfo` | `_intervene`

鍦烘櫙锛?
- **娴嬭瘯妯″紡**: 瑙傚療鐪熷疄琛屼负锛屼慨姝?RULES/SOP
- **Map 妯″紡**: 骞惰澶勭悊鐙珛鍚屾瀯瀛愪换鍔?
- **楠岃瘉妯″紡**: 鐙珛 subagent 瀵规姉鎬ч獙璇?

### 5.3 Reflect 妯″紡锛堝弽灏勬ā寮忥級

鐩戞帶鑴氭湰瀹氭椂瑙﹀彂浠诲姟锛?
- 鍔犺浇鐩戞帶鑴氭湰锛宍check()` 瑙﹀彂鏃跺彂浠诲姟
- 鏀寔 `on_done` 鍥炶皟

---

## 鍏€乤gent-diva 褰撳墠鏋舵瀯瀵规瘮

### 6.1 缁撴瀯瀵规瘮琛?

| 鐗规€?| GenericAgent | agent-diva |
|------|-------------|------------|
| 璇█ | Python | Rust (workspace) |
| 鏍稿績浠ｇ爜閲?| ~3K 琛?| ~鏁颁竾琛?|
| Agent Loop | ~100 琛?generator | 鍒嗘暎鍦ㄥ涓ā鍧?|
| 璁板繂绯荤粺 | 5 灞傚垎灞傝蹇嗭紙L0-L4锛?| 2 灞傦紙MEMORY.md + HISTORY.md锛?|
| 宸ュ叿闆?| 9 涓師瀛愬伐鍏?| 娉ㄥ唽寮?Tool trait |
| Skill 绯荤粺 | 鑷姩杩涘寲娌夋穩 | SkillsLoader + SKILL.md |
| Subagent | 鏂囦欢 IO 鍗忚 | SubagentManager trait |
| LLM Session | 澶?Provider 鍏煎灞?| Provider trait |
| 娓犻亾鎺ュ叆 | 澶?frontend 鑴氭湰 | Channel trait + MessageBus |
| 娴忚鍣ㄦ帶鍒?| TMWebDriver (CDP) | 鏃犲唴缃?|

### 6.2 agent-diva 鐜版湁浼樺娍

1. **绫诲瀷瀹夊叏**: Rust 缂栬瘧鏃舵鏌?
2. **妯″潡鍖?*: 娓呮櫚鐨?crate 鍒嗙
3. **寮傛鍘熺敓**: Tokio runtime
4. **瀹夊叏鎬?*: SecurityPolicy 闄愬埗
5. **鍙墿灞?*: Provider/Channel/Tool trait
6. **浼氳瘽鎸佷箙鍖?*: JSONL SessionManager

### 6.3 GenericAgent 鍙€熼壌鐐?

1. **鍒嗗眰璁板繂绯荤粺**: L1-L4 鏋佺畝绱㈠紩 鈫?浜嬪疄搴?鈫?SOP 鈫?浼氳瘽褰掓。
2. **鏈€灏忓伐鍏烽泦鍝插**: 鍘熷瓙宸ュ叿缁勫悎鑰岄潪棰勮澶嶆潅宸ュ叿
3. **鑷垜杩涘寲鏈哄埗**: 浠诲姟瀹屾垚鑷姩娌夋穩 Skill
4. **浜や簰鍗忚**: `<thinking>`, `<summary>`, `<tool_use>` 缁撴瀯鍖栬緭鍑?
5. **Plan Mode**: 鎺㈢储鈫掕鍒掆啋鎵ц鈫掗獙璇佺殑瀹屾暣娴佺▼
6. **Subagent 鏂囦欢 IO 鍗忚**: 绠€娲佺殑 subagent 閫氫俊鏂瑰紡
7. **娴忚鍣ㄦ帶鍒?*: TMWebDriver CDP 娉ㄥ叆鏂规

---

## 涓冦€佸崌绾у缓璁?

### 7.1 楂樹紭鍏堢骇鍗囩骇椤?

| 椤圭洰 | 褰撳墠鐘舵€?| 鍗囩骇鏂瑰悜 | 鏀剁泭 |
|------|----------|----------|------|
| 璁板繂绯荤粺 | 2 灞?| 鎵╁睍涓?5 灞傚垎灞傝蹇?| Token 鏁堢巼銆侀暱绋嬪彫鍥?|
| 宸ュ叿鍗忚 | 鑷敱鏍煎紡 | 寮曞叆 `<tool_use>` 鍗忚 | 杈撳嚭瑙勮寖鍖?|
| Skill 娌夋穩 | 鎵嬪姩缂栧啓 | 鑷姩杩涘寲鏈哄埗 | 鑳藉姏绱Н |
| Plan Mode | 鏃?| 鏂板瀹屾暣瑙勫垝妯″紡 | 澶嶆潅浠诲姟鍙潬鎬?|
| 楠岃瘉鏈哄埗 | 鏃?| Subagent 瀵规姉鎬ч獙璇?| 缁撴灉鍑嗙‘鎬?|

### 7.2 涓紭鍏堢骇鍗囩骇椤?

| 椤圭洰 | 褰撳墠鐘舵€?| 鍗囩骇鏂瑰悜 | 鏀剁泭 |
|------|----------|----------|------|
| Subagent 閫氫俊 | Rust trait | 鍙傝€冩枃浠?IO 鍗忚绠€鍖?| 璋冭瘯渚垮埄 |
| 宸ヤ綔璁板繂 | 鏃?| 鏂板 `working_checkpoint` | 闀夸换鍔′笂涓嬫枃 |
| 娴忚鍣ㄦ帶鍒?| 鏃?| 闆嗘垚 CDP 鏂规 | Web 鑷姩鍖?|

### 7.3 浣庝紭鍏堢骇鍗囩骇椤?

| 椤圭洰 | 褰撳墠鐘舵€?| 鍗囩骇鏂瑰悜 | 鏀剁泭 |
|------|----------|----------|------|
| Reflect 妯″紡 | CronService | 瀹氭椂瑙﹀彂浠诲姟 | 鑷姩鐩戞帶 |
| LLM Session | Provider trait | 鍙傝€冨鏍煎紡鍏煎灞?| Provider 鎵╁睍 |

---

## 鍏€佸疄鐜拌矾寰勫缓璁?

### Phase 1: 璁板繂绯荤粺鍗囩骇

1. 鏂板 `L1InsightIndex` 妯″潡锛堟瀬绠€绱㈠紩锛?
2. 鏂板 `L3SopLibrary` 妯″潡锛堜换鍔＄骇 SOP锛?
3. 鏂板 `L4SessionArchive` 妯″潡锛堜細璇濆綊妗ｏ級
4. 瀹氫箟璁板繂鍐欏叆鍏悊鍜屽垎绫诲喅绛栨爲

### Phase 2: 浜や簰鍗忚鍗囩骇

1. 瀹氫箟 `<thinking>`, `<summary>`, `<tool_use>` 鍗忚
2. 淇敼 ContextBuilder 娉ㄥ叆鍗忚璇存槑
3. 淇敼 loop_turn 瑙ｆ瀽鍗忚杈撳嚭

### Phase 3: Plan Mode 瀹炵幇

1. 鏂板 `PlanMode` 妯″潡
2. 瀹炵幇 `exploration_findings.md` 杈撳嚭
3. 瀹炵幇 `plan.md` 鎵ц寰幆
4. 瀹炵幇楠岃瘉 subagent 鏈哄埗

### Phase 4: Skill 鑷姩杩涘寲

1. 鏂板 `SkillEvolution` 妯″潡
2. 瀹炵幇浠诲姟瀹屾垚鏃剁殑 Skill 娌夋穩
3. 瀹炵幇 Skill 鎼滅储鍜屽尮閰?

---

## 涔濄€侀闄╀笌娉ㄦ剰浜嬮」

1. **Rust 瀹炵幇澶嶆潅鎬?*: 鍒嗗眰璁板繂闇€璋ㄦ厧璁捐绫诲瀷绯荤粺
2. **Token 娑堣€?*: L1 绱㈠紩闇€涓ユ牸鎺у埗澶у皬
3. **楠岃瘉寮€閿€**: Subagent 楠岃瘉澧炲姞涓€杞?LLM 璋冪敤
4. **鍏煎鎬?*: 鍗忚鍗囩骇闇€鑰冭檻鐜版湁 Skill 鏍煎紡
5. **娴嬭瘯瑕嗙洊**: Plan Mode 闇€澶ч噺鍦烘櫙娴嬭瘯

---

## 鍗併€丩LM Session 澶?Provider 鍏煎鏈哄埗

### 10.1 Session 绫诲瀷鏄犲皠

GenericAgent 閫氳繃鍙橀噺鍛藉悕瑙勫垯鑷姩璺敱鍒颁笉鍚岀殑 LLM Session 绫诲瀷锛?

| 鍙橀噺鍚嶅寘鍚?| Session 绫诲瀷 | API 鏍煎紡 |
|-----------|-------------|----------|
| `oai` (涓嶅惈 `native`) | `LLMSession` | OpenAI 鍏煎 `/v1/chat/completions` |
| `claude` (涓嶅惈 `native`) | `ClaudeSession` | Claude 鍏煎 `/messages` |
| `native` + `claude` | `NativeClaudeSession` | Claude 鍘熺敓宸ュ叿璋冪敤 |
| `native` + `oai` | `NativeOAISession` | OpenAI 鍘熺敓宸ュ叿璋冪敤 |
| `mixin` | `MixinSession` | 澶?Session 鐑垏鎹?fallback |

### 10.2 Native Session 鐗圭偣

`NativeClaudeSession` 鍜?`NativeOAISession` 浣跨敤鍘熺敓宸ュ叿璋冪敤鏍煎紡锛?
- Claude: `tool_use` content blocks锛屾敮鎸?`thinking` blocks
- OpenAI: `tool_calls` 鍝嶅簲瀛楁锛屾敮鎸?`reasoning_effort`

### 10.3 MixinSession 鐑垏鎹?

```python
class MixinSession:
    """Multi-session fallback with spring-back to primary."""
    def __init__(self, all_sessions, cfg):
        self._sessions = [...]  # 澶氫釜 session
        self._spring_sec = cfg.get('spring_back', 300)  # 5鍒嗛挓鍚庡垏鍥炰富

    def _raw_ask(self, *args, **kwargs):
        # 閬囬敊鑷姩鍒囨崲涓嬩竴涓?session
        # 鎴愬姛鍚庤褰曞垏鎹㈡椂闂达紝瓒呰繃 spring_sec 鍚庡垏鍥炰富
```

**搴旂敤鍦烘櫙**: 涓?Provider 鏁呴殰鏃惰嚜鍔ㄥ垏鎹㈠鐢?Provider锛岀ǔ瀹氬悗鍒囧洖銆?

---

## 鍗佷竴銆丗rontend 娓犻亾鎺ュ叆

### 11.1 宸叉敮鎸佹笭閬?

| 娓犻亾 | 鏂囦欢 | 鎺ュ叆鏂瑰紡 |
|------|------|----------|
| **Streamlit Web** | `launch.pyw` | 榛樿 Web UI |
| **Qt Desktop** | `frontends/qtapp.py` | PyQt 妗岄潰搴旂敤 |
| **妗岄潰瀹犵墿** | `frontends/desktop_pet_v2.pyw` | 妗岄潰鎮诞瀹犵墿 |
| **Telegram** | `frontends/tgapp.py` | Bot API |
| **寰俊涓汉鍙?* | `frontends/wechatapp.py` | 鎵爜鐧诲綍 |
| **QQ** | `frontends/qqapp.py` | WebSocket 闀胯繛鎺?|
| **椋炰功 Lark** | `frontends/fsapp.py` | 寮€鏀惧钩鍙?API |
| **浼佷笟寰俊** | `frontends/wecomapp.py` | AI Bot SDK |
| **閽夐拤** | `frontends/dingtalkapp.py` | Stream 鍗忚 |

### 11.2 AgentChatMixin 鍏叡閫昏緫

```python
class AgentChatMixin:
    """娓犻亾鎺ュ叆鍏叡閫昏緫"""
    async def handle_command(self, chat_id, cmd, **ctx):
        # /help /status /stop /new /restore /llm

    async def run_agent(self, chat_id, text, **ctx):
        # 璋冪敤 agent.put_task() 骞惰疆璇?output
        # ping_interval = 20s 闃查暱浠诲姟瓒呮椂
```

---

## 鍗佷簩銆佸叧閿?SOP 绀轰緥鎽樿

### 12.1 memory_management_sop.md

**鏍稿績鍏悊**:
- 琛屽姩楠岃瘉鍘熷垯: 鏃犺鍔ㄤ笉璁板繂
- 绁炲湥涓嶅彲鍒犳敼: 楠岃瘉杩囩殑閰嶇疆涓嶅彲涓㈠純
- 绂佹鏄撳彉鐘舵€? 涓嶅瓨鏃堕棿鎴?PID/Session ID
- 鏈€灏忓厖鍒嗘寚閽? 涓婂眰鍙暀瀹氫綅鏍囪瘑

**鍚屾瑙勫垯**: L1 鈫?L2/L3 鍙樺寲鏃跺悓姝ョ储寮?

### 12.2 plan_sop.md

**瑙﹀彂鏉′欢**: 3姝ヤ互涓娿€佹湁渚濊禆銆佸鏂囦欢鍗忓悓

**鍥涢樁娈垫祦绋?*:
1. **鎺㈢储鎬?*: 涓籥gent绂佹帰娴嬶紝濮旀墭subagent
2. **瑙勫垝鎬?*: `[D]` 濮旀墭鏍囨敞锛岀敤鎴风‘璁ら棬
3. **鎵ц鎬?*: 杩炵画鎵ц涓嶅仠椤匡紝绂佸嚟璁板繂
4. **楠岃瘉鎬?*: `[VERIFY]` 寮哄埗瀵规姉楠岃瘉

**缁堟妫€鏌?*: file_read plan.md 纭 0 涓?`[ ]` 娈嬬暀

### 12.3 subagent.md

**鏂囦欢 IO 鍗忚**:
```
temp/{task_name}/
鈹溾攢鈹€ input.txt      # 鐩爣+绾︽潫锛堢鍐欐楠わ級
鈹溾攢鈹€ output.txt     # 涓昏緭鍑猴紙append锛?
鈹溾攢鈹€ output1.txt    # 澶氳疆杈撳嚭
鈹溾攢鈹€ reply.txt      # 缁х画鎸囦护
鈹溾攢鈹€ _stop          # 缁堟淇″彿
鈹溾攢鈹€ _keyinfo       # 娉ㄥ叆宸ヤ綔璁板繂
鈹斺攢鈹€ _intervene     # 杩藉姞鎸囦护
```

**Map 妯″紡**: 骞惰澶勭悊鐙珛鍚屾瀯瀛愪换鍔★紝涓籥gent鏀堕泦姹囨€?

### 12.4 verify_sop.md

**楠岃瘉绛栫暐**: 鎸?task_type 閫夋嫨
- code: 缂栬瘧銆佹祴璇曘€侀潤鎬佹鏌?
- data: schema 楠岃瘉銆佹娊鏍锋鏌?
- browser: 鎴浘璇佹嵁
- file: 鍐呭闈炵┖銆佹牸寮忔纭?

**VERDICT 杈撳嚭**: `PASS / FAIL / PARTIAL`

---

## 鍗佷笁銆佺粨璁?

GenericAgent 鎻愪緵浜嗕竴濂楁瀬绠€浣嗛珮搴︽湁鏁堢殑 Agent 鏋舵瀯鑼冨紡锛屽叾鍒嗗眰璁板繂绯荤粺鍜岃嚜鎴戣繘鍖栨満鍒舵槸鏍稿績浜偣銆俛gent-diva 褰撳墠鏋舵瀯鍦ㄧ被鍨嬪畨鍏ㄥ拰妯″潡鍖栦笂鏈変紭鍔匡紝浣嗗湪璁板繂鏁堢巼銆佸鏉備换鍔¤鍒掑拰鑳藉姏绱Н鏂归潰鏈夎緝澶ф彁鍗囩┖闂淬€?

寤鸿鍒嗛樁娈靛疄鏂藉崌绾э紝浼樺厛寮曞叆鍒嗗眰璁板繂绯荤粺鍜屼氦浜掑崗璁紝閫愭鎵╁睍 Plan Mode 鍜?Skill 鑷姩杩涘寲鑳藉姏銆?

---

## 闄勫綍 A锛氭枃浠剁储寮?

| 鏂囦欢璺緞 | 鏍稿績鍐呭 |
|----------|----------|
| `agentmain.py` | 涓诲叆鍙ｃ€丩LM Session 鍒濆鍖栥€佷换鍔￠槦鍒?|
| `agent_loop.py` | 鏍稿績 Agent Loop (~100琛? |
| `ga.py` | GenericAgentHandler + 宸ュ叿瀹炵幇 |
| `llmcore.py` | 澶?Provider Session 鎶借薄 |
| `simphtml.py` | HTML 绠€鍖栨彁鍙?|
| `TMWebDriver.py` | CDP 娴忚鍣ㄦ帶鍒?|
| `memory/global_mem_insight.txt` | L1 绱㈠紩灞?|
| `memory/global_mem.txt` | L2 浜嬪疄搴?|
| `memory/*_sop.md` | L3 SOP 搴?|
| `memory/L4_raw_sessions/` | L4 浼氳瘽褰掓。 |
| `assets/tools_schema.json` | 宸ュ叿瀹氫箟 JSON |
| `assets/sys_prompt.txt` | 绯荤粺鎻愮ず妯℃澘 |
| `frontends/*.py` | 澶氭笭閬撴帴鍏ュ疄鐜?|
