<script setup lang="ts">
import { computed, nextTick, onMounted, onUnmounted, ref, watch } from "vue";
import { isTauri } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import TitleBar from "./TitleBar.vue";
import Atmosphere from "./Atmosphere.vue";
import SettingsView from "./SettingsView.vue";
import MarkdownText from "./MarkdownText.vue";
import QuestionCard from "./QuestionCard.vue";
import { runtime } from "./ipc";
import { describeFailure } from "./failure";
import { debugOutput } from "./debug";
const state=runtime.state;
const failures=computed(()=>Object.fromEntries((state.snapshot?.tasks??[]).flatMap(task=>task.error?[[task.id,describeFailure(task.error)]]:[])));
const draft=ref("");const sending=ref(false);const settingsOpen=ref(false);
const settings=ref<InstanceType<typeof SettingsView>|null>(null);
const titleBar=ref<InstanceType<typeof TitleBar>|null>(null);
const list=ref<HTMLElement|null>(null);const input=ref<HTMLTextAreaElement|null>(null);
const pinned=ref(true);const newMessages=ref(false);
let stableDraft:{id:string,text:string}|null=null;let unlisten:(()=>void)|undefined;
const canSend=computed(()=>!!draft.value.trim()&&state.connected&&state.snapshot?.model.ready&&!sending.value);
const statusNames:Record<string,string>={queued:'排队中',running:'进行中',waiting:'等待唤醒',suspended:'已中断',completed:'已完成',failed:'失败',cancelled:'已停止',interrupted:'已中断'};
const status=(value:string)=>statusNames[value]??value;
async function latest(){await nextTick();if(list.value)list.value.scrollTop=list.value.scrollHeight;newMessages.value=false;}
function scrolled(){if(list.value)pinned.value=list.value.scrollHeight-list.value.scrollTop-list.value.clientHeight<70;}
watch(()=>[state.history.length,state.pending.length,state.live?.text,state.live?.reasoning,state.snapshot?.questions.length],()=>{if(pinned.value)void latest();else newMessages.value=true;});
function firstLine(text:string|null|undefined){if(!text)return '';const line=text.split('\n').map(l=>l.trim()).find(Boolean)??'';return line.length>120?`${line.slice(0,119)}…`:line;}
async function send(){
 if(!canSend.value)return;const text=draft.value.trim();
 if(stableDraft?.text!==text)stableDraft={id:crypto.randomUUID(),text};
 sending.value=true;
 try{await runtime.send(text,stableDraft.id);draft.value='';stableDraft=null;resizeInput();}catch{/* draft and stable retry identity stay visible */}finally{sending.value=false;}
}
async function toggleSettings(){if(settingsOpen.value&&settings.value&&!await settings.value.canLeave())return;settingsOpen.value=!settingsOpen.value;}
async function closeSettings(){if(settings.value&&!await settings.value.canLeave())return;settingsOpen.value=false;void nextTick(()=>titleBar.value?.focusMenu());}
function resizeInput(){if(input.value){input.value.style.height='auto';input.value.style.height=`${Math.min(input.value.scrollHeight,140)}px`;}}
function key(event:KeyboardEvent){if(event.key==='Enter'&&!event.shiftKey&&!event.isComposing){event.preventDefault();void send();}}
onMounted(async()=>{if(isTauri())unlisten=await listen('open-settings',()=>{settingsOpen.value=true;});await runtime.connect();});
onUnmounted(()=>{unlisten?.();runtime.dispose();});
</script>
<template>
 <div class="app-shell"><Atmosphere/><TitleBar ref="titleBar" :settings-open="settingsOpen" @toggle-settings="toggleSettings"/>
  <SettingsView v-if="settingsOpen" ref="settings" @close="closeSettings"/>
  <main v-show="!settingsOpen" class="chat">
   <div ref="list" class="messages" aria-label="对话内容" @scroll="scrolled">
    <div v-if="!state.history.length&&!state.pending.length" class="empty">{{state.snapshot?.model.ready?'在这里开始对话':'在设置中配置网关后开始对话'}}</div>
    <template v-for="message in state.history" :key="message.id">
     <details v-if="debugOutput&&message.reasoning" class="reasoning"><summary><span class="thinking-dot"></span><span class="reasoning-label">推理</span><span class="reasoning-preview">{{firstLine(message.reasoning)}}</span></summary><pre>{{message.reasoning}}</pre></details>
     <div class="message-row" :class="message.role"><div class="bubble"><MarkdownText :text="message.text"/><small>{{status(message.status)}}</small><p v-if="failures[message.task]" class="turn-failure" role="alert"><strong>{{failures[message.task].title}}</strong><span v-if="failures[message.task].detail">：{{failures[message.task].detail}}</span></p><small v-if="message.truncated">长内容预览已截断；完整记录保留在核心会话中。</small></div></div>
    </template>
    <div v-for="message in state.pending" :key="message.request_id" class="message-row user"><div class="bubble"><MarkdownText :text="message.text"/><small>发送中 · 等待持久化确认</small></div></div>
    <details v-if="debugOutput&&state.live?.reasoning" class="reasoning" open><summary><span class="thinking-dot running"></span><span class="reasoning-label">推理中</span><span class="reasoning-preview">{{firstLine(state.live.reasoning)}}</span></summary><pre>{{state.live.reasoning}}</pre></details>
    <div v-if="state.live?.text" class="message-row assistant"><div class="bubble"><MarkdownText :text="state.live.text"/><small>正在生成</small></div></div>
    <div v-if="state.interrupted" class="message-row assistant"><div class="bubble"><MarkdownText :text="state.interrupted"/><small>连接中断 · 此片段未确认为最终回复</small></div></div>
    <section v-if="debugOutput" class="debug-records" aria-label="调试记录">
     <h4>工具调用</h4>
     <details v-for="tool in state.snapshot?.tools" :key="tool.id" class="tool" :class="tool.phase"><summary><span class="tool-state" aria-hidden="true"></span><span class="tool-name">{{tool.name}}</span><span class="tool-phase">{{tool.phase}}</span><span v-if="firstLine(tool.output)" class="tool-line">{{firstLine(tool.output)}}</span></summary><pre>{{tool.output??'尚无已提交结果'}}</pre><small v-if="tool.truncated">长工具结果预览已截断；完整记录保留在核心会话中。</small></details>
     <p v-if="!state.snapshot?.tools.length" class="debug-empty">暂无工具调用记录。</p>
    </section>
    <QuestionCard v-for="question in state.snapshot?.questions" :key="state.instance+question.id" :question="question"/>
   </div>
   <button v-if="newMessages" class="new-message" @click="latest">查看新消息 ↓</button>
   <div class="composer-wrap"><p class="connection-note">{{state.connecting?'正在连接…':state.connected?'已连接':'已断开'}}<button v-if="!state.connected&&!state.connecting" @click="runtime.connect">重新连接</button><button v-if="state.snapshot?.active_task" @click="runtime.stop().catch(e=>state.error=String(e))">停止</button></p>
    <p v-if="state.error" class="error" role="alert">{{state.error}}</p>
    <div class="composer"><textarea ref="input" v-model="draft" rows="1" placeholder="说点什么…" aria-label="输入消息" @input="resizeInput" @keydown="key"></textarea><button class="send" :disabled="!canSend" @click="send">{{sending?'发送中':'发送'}}</button></div>
   </div>
  </main>
 </div>
</template>
<style>
:root { font-family: "Segoe UI", "Microsoft YaHei", sans-serif; color: #533941; background: transparent; font-synthesis: none; -webkit-font-smoothing: antialiased; }
* { box-sizing: border-box; }
body { margin: 0; overflow: hidden; }
button, textarea { font: inherit; }
button:focus-visible, textarea:focus-visible { outline: 2px solid #ad7382; outline-offset: 2px; }
.app-shell { position: relative; isolation: isolate; overflow: hidden; height: 100vh; display: flex; flex-direction: column; background: #fce1d8; }
.app-shell > :not(.atmosphere) { position: relative; }
.chat { flex: 1; min-height: 0; display: flex; flex-direction: column; }
.messages { flex: 1; min-height: 0; overflow-y: auto; padding: 26px clamp(22px, 5vw, 64px); }
::-webkit-scrollbar { width: 10px; height: 10px; }
::-webkit-scrollbar-track { background: transparent; }
::-webkit-scrollbar-thumb { background: rgba(119, 71, 62, .35); border: 3px solid transparent; border-radius: 999px; background-clip: padding-box; }
::-webkit-scrollbar-thumb:hover { background-color: rgba(119, 71, 62, .6); }
.empty { color: #a98986; font-size: 13px; text-align: center; margin-top: 18vh; }
.message-row { display: flex; justify-content: flex-end; margin: 0 0 14px; }
.bubble { max-width: min(75%, 580px); padding: 11px 15px; border-radius: 16px 16px 4px 16px; background: rgba(255, 250, 246, .82); box-shadow: 0 3px 16px rgba(126, 76, 70, .06); font-size: 14px; line-height: 1.6; white-space: pre-wrap; overflow-wrap: anywhere; }

.connection-note { margin: 0 0 10px 52px; font-size: 11px; color: #896b6c; }
.composer-wrap { padding: 16px clamp(20px, 5vw, 64px) 28px; }
.composer { display: flex; align-items: flex-end; gap: 10px; }
.icon-button { flex: 0 0 42px; width: 42px; height: 42px; border: 1px solid rgba(133, 92, 91, .23); border-radius: 50%; background: rgba(255, 250, 246, .38); color: #78555c; display: grid; place-items: center; cursor: pointer; }
.icon-button svg { width: 21px; height: 21px; fill: none; stroke: currentColor; stroke-width: 1.6; stroke-linecap: round; stroke-linejoin: round; }
.icon-button:not(:disabled):hover { background: rgba(255, 250, 246, .7); }
.icon-button:disabled { opacity: .48; cursor: not-allowed; }
textarea, .hold { flex: 1; min-width: 0; min-height: 42px; max-height: 140px; border: 1px solid rgba(133, 92, 91, .14); border-radius: 12px; background: rgba(255, 252, 249, .78); color: #533941; }
textarea { resize: none; overflow-y: auto; padding: 11px 14px; line-height: 20px; }
textarea::placeholder { color: #ae9492; }
.hold { cursor: pointer; touch-action: none; user-select: none; font-size: 14px; }
.hold.active { background: #efd0cc; }
.hold:disabled { opacity: .7; }
.send { height: 42px; min-width: 58px; border: 0; border-radius: 10px; background: #a96f7c; color: #fff; cursor: pointer; }
.send:hover { background: #925969; }
.error { margin: 0 0 10px 52px; color: #8a353b; font-size: 12px; }
@media (max-width: 600px) { .composer-wrap { padding-bottom: 20px; } }
</style>

<style scoped>
.message-row.assistant{justify-content:flex-start}.assistant .bubble{border-radius:16px 16px 16px 4px;background:#fff5;max-width:min(90%,820px)}.bubble{white-space:normal;min-width:0}.bubble small{display:block;margin-top:8px;color:#987b7a;font-size:10px}.turn-failure{margin:8px 0 0;padding:7px 10px;border-radius:7px;background:#f7dcd8;color:#8a353b;font-size:12px;line-height:1.5;white-space:pre-wrap;overflow-wrap:anywhere}.turn-failure strong{font-weight:600}.connection-note,.error{margin-left:0}.connection-note button{margin-left:12px;border:0;background:transparent;color:#854d60;cursor:pointer}.new-message{align-self:center;border:0;border-radius:12px;padding:6px 14px;background:#fff9;color:#80545e;cursor:pointer}.send:disabled{opacity:.5;cursor:default}
.debug-records{margin:18px 0;padding:12px 14px;border:1px dashed #a96f7c45;border-radius:10px;background:rgba(255,247,244,.55)}.debug-records h4{margin:0 0 6px;font-size:10px;font-weight:600;letter-spacing:.14em;text-transform:uppercase;color:#a77a7b}.debug-empty{margin:6px 0 0;font-size:12px;color:#a77a7b}.tool{font-size:12px;border-bottom:1px solid #a96f7c30;color:#805b65}.tool:last-of-type{border-bottom:0}.tool summary{display:flex;align-items:center;gap:8px;min-height:24px;padding:4px 0;overflow:hidden;list-style:none;cursor:pointer}.tool summary::-webkit-details-marker{display:none}.tool summary::before{content:'';flex:0 0 auto;width:5px;height:5px;border-right:1.4px solid currentColor;border-bottom:1.4px solid currentColor;transform:rotate(-45deg);opacity:.7;transition:transform .15s ease}.tool[open] summary::before{transform:rotate(45deg)}.tool-state{flex:0 0 auto;width:6px;height:6px;border-radius:50%;background:#c9a59e}.tool.settled .tool-state{background:#9fbfa8}.tool.prepared .tool-state,.tool.dispatched .tool-state{background:#d8a657}.tool-name{flex:0 0 auto;color:#6d3f4c}.tool-phase{flex:0 0 auto;font-size:11px;color:#a77a7b}.tool-line{min-width:0;overflow:hidden;text-overflow:ellipsis;white-space:nowrap;padding-left:8px;border-left:1px solid #a96f7c40;color:#987b7a}.tool pre{margin:6px 0 4px;padding:9px 11px;border-radius:8px;background:rgba(255,250,247,.7);max-height:260px;overflow:auto;white-space:pre-wrap;overflow-wrap:anywhere}
.reasoning{max-width:min(90%,820px);margin:0 0 8px;font-size:12px;color:#8a6a70}.reasoning summary{display:flex;align-items:center;gap:8px;width:max-content;max-width:100%;padding:5px 10px;border:1px solid #a96f7c35;border-radius:999px;background:rgba(255,246,242,.6);color:#805b65;list-style:none;cursor:pointer}.reasoning summary::-webkit-details-marker{display:none}.reasoning-label{flex:0 0 auto;color:#7a4f5b}.reasoning-preview{min-width:0;overflow:hidden;text-overflow:ellipsis;white-space:nowrap;color:#a77a7b}.thinking-dot{flex:0 0 auto;width:7px;height:7px;border-radius:50%;background:#c58f99;box-shadow:0 0 0 2px rgba(197,143,153,.18)}.thinking-dot.running{animation:thinking 1.1s ease-in-out infinite}.reasoning pre{margin:6px 0 0;padding:9px 12px;border-left:2px solid #d6a5ad;border-radius:0 8px 8px 0;background:rgba(255,247,244,.6);max-height:260px;overflow:auto;white-space:pre-wrap;overflow-wrap:anywhere;font:inherit;line-height:1.65;color:#7e5f66}@keyframes thinking{0%,100%{opacity:.45;transform:scale(.9)}50%{opacity:1;transform:scale(1)}}@media (prefers-reduced-motion: reduce){.thinking-dot.running{animation:none}}
</style>
