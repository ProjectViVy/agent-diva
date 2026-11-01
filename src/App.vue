<script setup lang="ts">
import { computed, nextTick, onUnmounted, ref } from "vue";
import TitleBar from "./TitleBar.vue";
import Atmosphere from "./Atmosphere.vue";

type Message = { id: number; text: string } | { id: number; audio: string };
const messages = ref<Message[]>([]);
const draft = ref("");
const voiceMode = ref(false);
const recording = ref(false);
const requesting = ref(false);
const error = ref("");
const list = ref<HTMLElement | null>(null);
const canSend = computed(() => draft.value.trim().length > 0);
let nextId = 0;
let stream: MediaStream | undefined;
let recorder: MediaRecorder | undefined;
let held = false;
let disposed = false;
let cancelled = false;
let chunks: Blob[] = [];

async function scrollToLatest() {
  await nextTick();
  if (list.value) list.value.scrollTop = list.value.scrollHeight;
}
function send() {
  const text = draft.value.trim();
  if (!text) return;
  messages.value.push({ id: nextId++, text });
  draft.value = "";
  void scrollToLatest();
}
function stopTracks() {
  stream?.getTracks().forEach(track => track.stop());
  stream = undefined;
}
async function startRecording(event: PointerEvent) {
  if (event.button !== 0 || held || recording.value || requesting.value) return;
  held = true;
  cancelled = false;
  error.value = "";
  (event.currentTarget as HTMLElement).setPointerCapture(event.pointerId);
  if (!navigator.mediaDevices?.getUserMedia || typeof MediaRecorder === "undefined") {
    held = false;
    error.value = "当前环境不支持录音。";
    return;
  }
  requesting.value = true;
  try {
    const audio = await navigator.mediaDevices.getUserMedia({ audio: true });
    if (!held || disposed) { audio.getTracks().forEach(track => track.stop()); return; }
    stream = audio;
    chunks = [];
    const active = new MediaRecorder(audio);
    recorder = active;
    active.ondataavailable = e => { if (e.data.size) chunks.push(e.data); };
    active.onerror = () => { cancelled = true; error.value = "录音失败，请重试。"; stopTracks(); recording.value = false; };
    active.onstop = () => {
      if (!disposed && !cancelled && chunks.length) {
        const url = URL.createObjectURL(new Blob(chunks, { type: active.mimeType || "audio/webm" }));
        messages.value.push({ id: nextId++, audio: url });
        void scrollToLatest();
      }
      chunks = [];
      stopTracks();
      recorder = undefined;
      recording.value = false;
    };
    active.start();
    recording.value = true;
  } catch {
    stopTracks();
    error.value = "无法使用麦克风，请检查权限后重试。";
  } finally {
    requesting.value = false;
  }
}
function endRecording(cancel = false) {
  held = false;
  if (cancel) cancelled = true;
  if (recorder && recorder.state !== "inactive") recorder.stop();
  else stopTracks();
}
function toggleMode() {
  endRecording(true);
  voiceMode.value = !voiceMode.value;
  error.value = "";
}
function onKeydown(event: KeyboardEvent) {
  if (event.key === "Enter" && !event.shiftKey && !event.isComposing) {
    event.preventDefault();
    send();
  }
}
onUnmounted(() => {
  disposed = true;
  endRecording(true);
  messages.value.forEach(message => { if ("audio" in message) URL.revokeObjectURL(message.audio); });
});
</script>

<template>
  <div class="app-shell">
    <Atmosphere />
    <TitleBar />
    <main class="chat">
      <div ref="list" class="messages" aria-label="对话内容" aria-live="polite">
        <div v-if="!messages.length" class="empty">在这里开始对话</div>
        <div v-for="message in messages" :key="message.id" class="message-row">
          <div class="bubble">
            <template v-if="'text' in message">{{ message.text }}</template>
            <audio v-else :src="message.audio" controls aria-label="播放语音消息"></audio>
          </div>
        </div>
      </div>
      <div class="composer-wrap">
        <p class="connection-note">尚未连接 · 消息仅在此窗口保留</p>
        <p v-if="error" class="error" role="alert">{{ error }}</p>
        <div class="composer">
          <button type="button" class="icon-button" :aria-label="voiceMode ? '切换到文字输入' : '切换到语音输入'" :title="voiceMode ? '文字输入' : '语音输入'" @click="toggleMode">
            <svg v-if="!voiceMode" viewBox="0 0 24 24" aria-hidden="true"><rect x="9" y="3" width="6" height="11" rx="3"/><path d="M5.5 11a6.5 6.5 0 0 0 13 0M12 17.5V21m-3 0h6"/></svg>
            <svg v-else viewBox="0 0 24 24" aria-hidden="true"><rect x="3" y="6" width="18" height="12" rx="2"/><path d="M6 10h.1M9 10h.1m3 0h.1m3 0h.1m3 0h.1M7 14h10"/></svg>
          </button>
          <button v-if="voiceMode" type="button" class="hold" :class="{ active: recording }" :aria-busy="requesting" @pointerdown.prevent="startRecording" @pointerup="endRecording(false)" @pointercancel="endRecording(true)" @lostpointercapture="held && endRecording(true)" @contextmenu.prevent>{{ requesting ? '正在请求麦克风…' : recording ? '松开发送' : '按住说话' }}</button>
          <textarea v-else v-model="draft" rows="1" placeholder="说点什么…" aria-label="输入消息" @keydown="onKeydown"></textarea>
          <button v-if="!voiceMode && canSend" type="button" class="send" @click="send">发送</button>
          <button v-else type="button" class="icon-button add" aria-label="更多功能，暂未开放" title="更多功能，暂未开放" disabled><svg viewBox="0 0 24 24" aria-hidden="true"><path d="M12 4v16M4 12h16"/></svg></button>
        </div>
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
.messages::-webkit-scrollbar { width: 10px; }
.messages::-webkit-scrollbar-thumb { background: rgba(119, 71, 62, .35); border: 3px solid transparent; border-radius: 999px; background-clip: padding-box; }
.messages::-webkit-scrollbar-thumb:hover { background-color: rgba(119, 71, 62, .6); }
.empty { color: #a98986; font-size: 13px; text-align: center; margin-top: 18vh; }
.message-row { display: flex; justify-content: flex-end; margin: 0 0 14px; }
.bubble { max-width: min(75%, 580px); padding: 11px 15px; border-radius: 16px 16px 4px 16px; background: rgba(255, 250, 246, .82); box-shadow: 0 3px 16px rgba(126, 76, 70, .06); font-size: 14px; line-height: 1.6; white-space: pre-wrap; overflow-wrap: anywhere; }
.bubble audio { display: block; max-width: 100%; height: 36px; }
.connection-note { margin: 0 0 10px 52px; font-size: 11px; color: #896b6c; }
.composer-wrap { padding: 16px clamp(20px, 5vw, 64px) 28px; }
.composer { display: flex; align-items: flex-end; gap: 10px; }
.icon-button { flex: 0 0 42px; width: 42px; height: 42px; border: 1px solid rgba(133, 92, 91, .23); border-radius: 50%; background: rgba(255, 250, 246, .38); color: #78555c; display: grid; place-items: center; cursor: pointer; }
.icon-button svg { width: 21px; height: 21px; fill: none; stroke: currentColor; stroke-width: 1.6; stroke-linecap: round; stroke-linejoin: round; }
.icon-button:not(:disabled):hover { background: rgba(255, 250, 246, .7); }
.icon-button:disabled { opacity: .48; cursor: not-allowed; }
textarea, .hold { flex: 1; min-width: 0; min-height: 42px; max-height: 140px; border: 1px solid rgba(133, 92, 91, .14); border-radius: 12px; background: rgba(255, 252, 249, .78); color: #533941; }
textarea { resize: vertical; padding: 11px 14px; line-height: 20px; }
textarea::placeholder { color: #ae9492; }
.hold { cursor: pointer; touch-action: none; user-select: none; font-size: 14px; }
.hold.active { background: #efd0cc; }
.hold:disabled { opacity: .7; }
.send { height: 42px; min-width: 58px; border: 0; border-radius: 10px; background: #a96f7c; color: #fff; cursor: pointer; }
.send:hover { background: #925969; }
.error { margin: 0 0 10px 52px; color: #8a353b; font-size: 12px; }
@media (max-width: 600px) { .composer-wrap { padding-bottom: 20px; } }
</style>
