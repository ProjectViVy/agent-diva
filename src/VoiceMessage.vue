<script setup lang="ts">
import { computed, ref } from "vue";

defineProps<{ src: string }>();
const audio = ref<HTMLAudioElement | null>(null);
const playing = ref(false);
const time = ref(0);
const duration = ref(0);
const failed = ref(false);
const progress = computed(() => duration.value > 0 ? (time.value / duration.value) * 100 : 0);
function sync() {
  const player = audio.value;
  if (!player) return;
  time.value = Number.isFinite(player.currentTime) ? player.currentTime : 0;
  duration.value = Number.isFinite(player.duration) ? player.duration : 0;
}
function format(seconds: number) {
  const value = Math.max(0, Math.floor(seconds));
  return `${Math.floor(value / 60)}:${String(value % 60).padStart(2, "0")}`;
}
async function toggle() {
  const player = audio.value;
  if (!player) return;
  if (!player.paused) { player.pause(); return; }
  try { await player.play(); failed.value = false; } catch { failed.value = true; }
}
function seek(event: Event) {
  const player = audio.value;
  if (player && duration.value) player.currentTime = Number((event.target as HTMLInputElement).value);
}
</script>

<template>
  <div class="voice-player">
    <audio ref="audio" :src="src" preload="metadata" @loadedmetadata="sync" @durationchange="sync" @timeupdate="sync" @play="playing = true" @pause="playing = false" @ended="playing = false" @error="failed = true"></audio>
    <button class="play" type="button" :aria-label="playing ? '暂停语音' : '播放语音'" @click="toggle">
      <svg viewBox="0 0 20 20" aria-hidden="true"><path v-if="playing" d="M6 4h3v12H6zm5 0h3v12h-3z"/><path v-else d="M6 3.5 16 10 6 16.5z"/></svg>
    </button>
    <input class="seek" type="range" min="0" :max="duration || 1" step="0.1" :value="time" :disabled="!duration" :style="{ '--progress': `${progress}%` }" aria-label="语音播放进度" @input="seek" />
    <span class="time">{{ format(time) }}<template v-if="duration"> / {{ format(duration) }}</template></span>
    <span v-if="failed" class="failure" role="alert">无法播放语音</span>
  </div>
</template>

<style scoped>
.voice-player { display: flex; align-items: center; gap: 10px; width: min(270px, 100%); min-width: 0; flex-wrap: wrap; }
.play { flex: 0 0 32px; width: 32px; height: 32px; border: 0; border-radius: 50%; display: grid; place-items: center; background: #a96f7c; color: white; cursor: pointer; }
.play:hover { background: #925969; }
.play svg { width: 16px; height: 16px; fill: currentColor; }
.seek { appearance: none; -webkit-appearance: none; flex: 1 1 65px; min-width: 0; height: 20px; margin: 0; background: transparent; cursor: pointer; }
.seek::-webkit-slider-runnable-track { height: 4px; border-radius: 99px; background: linear-gradient(to right, #a96f7c var(--progress), rgba(137, 92, 95, .2) var(--progress)); }
.seek::-webkit-slider-thumb { -webkit-appearance: none; width: 12px; height: 12px; margin-top: -4px; border-radius: 50%; background: #a96f7c; }
.seek:disabled { opacity: .45; cursor: default; }
.time { color: #896b6c; font-size: 11px; font-variant-numeric: tabular-nums; white-space: nowrap; }
.failure { flex-basis: 100%; color: #8a353b; font-size: 12px; }
.play:focus-visible, .seek:focus-visible { outline: 2px solid #ad7382; outline-offset: 2px; }
</style>
