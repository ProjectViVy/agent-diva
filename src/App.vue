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
const state=runtime.state;
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
watch(()=>[state.history.length,state.pending.length,state.live?.text,state.snapshot?.questions.length],()=>{if(pinned.value)void latest();else newMessages.value=true;});
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
    <div v-for="message in state.history" :key="message.id" class="message-row" :class="message.role"><div class="bubble"><MarkdownText :text="message.text"/><small>{{status(message.status)}}</small><small v-if="message.truncated">长内容预览已截断；完整记录保留在核心会话中。</small></div></div>
    <div v-for="message in state.pending" :key="message.request_id" class="message-row user"><div class="bubble"><MarkdownText :text="message.text"/><small>发送中 · 等待持久化确认</small></div></div>
    <div v-if="state.live?.text" class="message-row assistant"><div class="bubble"><MarkdownText :text="state.live.text"/><small>正在生成</small></div></div>
    <div v-if="state.interrupted" class="message-row assistant"><div class="bubble"><MarkdownText :text="state.interrupted"/><small>连接中断 · 此片段未确认为最终回复</small></div></div>
    <details v-for="tool in state.snapshot?.tools" :key="tool.id" class="tool"><summary>{{tool.name}} · {{tool.phase}}</summary><pre>{{tool.output??'尚无已提交结果'}}</pre><small v-if="tool.truncated">长工具结果预览已截断；完整记录保留在核心会话中。</small></details>
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
.message-row.assistant{justify-content:flex-start}.assistant .bubble{border-radius:16px 16px 16px 4px;background:#fff5;max-width:min(90%,820px)}.bubble{white-space:normal;min-width:0}.bubble small{display:block;margin-top:8px;color:#987b7a;font-size:10px}.connection-note,.error{margin-left:0}.connection-note button{margin-left:12px;border:0;background:transparent;color:#854d60;cursor:pointer}.tool{font-size:12px;border-bottom:1px solid #a96f7c30;padding:8px 0;color:#805b65}.tool pre{max-height:260px;overflow:auto;white-space:pre-wrap;overflow-wrap:anywhere}.new-message{align-self:center;border:0;border-radius:12px;padding:6px 14px;background:#fff9;color:#80545e;cursor:pointer}.send:disabled{opacity:.5;cursor:default}
</style>
