<script setup lang="ts">
import { onMounted, ref } from "vue";
import { invoke, isTauri } from "@tauri-apps/api/core";

const status = ref("Connecting to the desktop host...");
const busy = ref(false);

async function checkHost() {
  if (busy.value) return;
  if (!isTauri()) {
    status.value = "Browser preview. Open the desktop app to connect to the native host.";
    return;
  }
  busy.value = true;
  try {
    const info = await invoke<{ name: string; version: string }>("app_info");
    status.value = `${info.name} ${info.version} · Desktop host connected`;
  } catch {
    status.value = "Unable to reach the desktop host. Please try again.";
  } finally {
    busy.value = false;
  }
}

onMounted(checkHost);
</script>

<template>
  <main>
    <header>PROJECT VIVY <span>DESKTOP</span></header>
    <section aria-labelledby="title">
      <p class="eyebrow">A new beginning</p>
      <h1 id="title">Agent Diva</h1>
      <p class="intro">A new home for your agent.</p>
      <p class="note">The desktop foundation is ready. Agent capabilities will be connected in the next stage.</p>
      <div class="host">
        <p role="status">{{ status }}</p>
        <button :disabled="busy" @click="checkHost">{{ busy ? "Checking..." : "Check connection" }}</button>
      </div>
    </section>
    <footer>Tauri + Vue <span>Foundation / 01</span></footer>
  </main>
</template>

<style>
:root {
  font-family: "Segoe UI", sans-serif;
  color: #24332e;
  background: #f5f5ef;
  font-synthesis: none;
  text-rendering: optimizeLegibility;
  -webkit-font-smoothing: antialiased;
}
* { box-sizing: border-box; }
body { margin: 0; }
main { min-height: 100vh; padding: clamp(24px, 6vw, 72px); display: flex; flex-direction: column; }
header, footer { display: flex; justify-content: space-between; gap: 16px; font-size: 12px; letter-spacing: .12em; }
header span, footer { color: #64726a; }
section { width: min(100%, 680px); margin: auto 0; padding: 64px 0; }
.eyebrow { color: #506b5d; font-size: 14px; }
h1 { font-size: clamp(48px, 8vw, 88px); font-weight: 500; letter-spacing: -.06em; margin: 12px 0; line-height: 1.1; }
.intro { font-size: 24px; }
.note { color: #64726a; max-width: 460px; line-height: 1.7; }
.host { border-top: 1px solid #d5dcd3; margin-top: 40px; padding-top: 16px; }
.host p { font-size: 13px; line-height: 1.6; }
button { font: inherit; font-size: 14px; padding: 10px 16px; border: 1px solid #314d3e; border-radius: 6px; color: #fff; background: #314d3e; cursor: pointer; }
button:hover { background: #243d30; }
button:focus-visible { outline: 3px solid #819c89; outline-offset: 3px; }
button:disabled { opacity: .6; cursor: wait; }
</style>
