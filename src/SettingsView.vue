<script setup lang="ts">
import { nextTick, ref } from "vue";

const emit = defineEmits<{ close: [] }>();
const sections = [
  { id: "providers", label: "供应商", subtitle: "Providers", description: "模型供应商配置尚未接入。" },
  { id: "mcp", label: "MCP", subtitle: "Model Context Protocol", description: "MCP 连接配置尚未接入。" },
  { id: "skills", label: "Skill", subtitle: "Skills", description: "Skill 配置尚未接入。" },
] as const;
const selected = ref<(typeof sections)[number]>(sections[0]);
const showDetail = ref(false);
const detailHeading = ref<HTMLElement | null>(null);
const navHeading = ref<HTMLElement | null>(null);
async function select(section: (typeof sections)[number]) {
  selected.value = section;
  showDetail.value = true;
  if (window.matchMedia("(max-width: 680px)").matches) {
    await nextTick();
    detailHeading.value?.focus();
  }
}
async function backToList() {
  showDetail.value = false;
  await nextTick();
  navHeading.value?.focus();
}
</script>

<template>
  <section id="settings-panel" class="settings" :class="{ 'show-detail': showDetail }" aria-label="设置">
    <div class="settings-top">
      <span class="overline">AGENT DIVA / SETTINGS</span>
      <button type="button" class="return" @click="emit('close')">← 返回对话</button>
    </div>
    <div class="settings-body">
      <nav class="settings-nav" aria-label="设置分类">
        <h1 ref="navHeading" tabindex="-1">设置<span>.</span></h1>
        <p class="nav-caption">选择一个类别</p>
        <div class="nav-items">
          <button v-for="section in sections" :key="section.id" type="button" :class="{ selected: selected.id === section.id }" :aria-current="selected.id === section.id ? 'page' : undefined" @click="select(section)">{{ section.label }}<span aria-hidden="true">↗</span></button>
        </div>
      </nav>
      <article class="settings-detail" :aria-labelledby="'settings-detail-title'">
        <button type="button" class="back" @click="backToList">← 返回分类</button>
        <div :key="selected.id" class="detail-content">
        <span class="detail-watermark" aria-hidden="true">{{ selected.id === "providers" ? "01" : selected.id === "mcp" ? "02" : "03" }}</span>
        <p class="detail-kicker">{{ selected.subtitle }}</p>
        <h2 id="settings-detail-title" ref="detailHeading" tabindex="-1">{{ selected.label }}</h2>
        <div class="detail-rule"></div>
        <p class="detail-message">{{ selected.description }}</p>
        <p class="detail-note">相关功能将在后续版本提供。</p>
        </div>
      </article>
    </div>
  </section>
</template>

<style scoped>
.settings { animation: reveal .38s ease-out both; flex: 1; min-height: 0; display: flex; flex-direction: column; color: #563b41; }
.settings-top { display: flex; align-items: center; justify-content: space-between; gap: 16px; padding: 20px clamp(24px, 5vw, 64px); }
.overline { font-size: 10px; letter-spacing: .17em; font-weight: 700; color: #9d6f73; }
.return, .back { border: 0; padding: 8px 0; background: transparent; color: #80545e; font-size: 13px; cursor: pointer; }
.return:hover, .back:hover { color: #b85f77; }
.settings-body { flex: 1; min-height: 0; display: grid; grid-template-columns: minmax(220px, 33%) minmax(0, 1fr); padding: 12px clamp(24px, 5vw, 64px) 38px; }
.settings-nav { min-height: 0; overflow-y: auto; padding: 14px 28px 20px 0; border-right: 1px solid rgba(115, 71, 68, .23); }
h1, h2 { margin: 0; font-weight: 500; letter-spacing: -.055em; line-height: 1.08; }
h1 { font-size: clamp(44px, 6vw, 70px); } h1 span { color: #cc7e85; }
.nav-caption, .detail-kicker { color: #a77a7b; font-size: 11px; letter-spacing: .12em; margin: 18px 0 30px; }
.nav-items { display: flex; flex-direction: column; align-items: stretch; gap: 4px; }
.nav-items button { display: flex; align-items: center; justify-content: space-between; gap: 12px; border: 0; padding: 10px 0; color: #a58582; background: transparent; text-align: left; font-size: clamp(19px, 2.1vw, 25px); cursor: pointer; transition: color .22s ease, transform .22s ease; }
.nav-items button span { opacity: 0; font-size: 15px; transition: opacity .22s ease, transform .22s ease; }
.nav-items button:hover, .nav-items button.selected { color: #7a3f50; }
.nav-items button:hover span, .nav-items button.selected span { opacity: 1; transform: translate(3px, -3px); }
.nav-items button:hover { transform: translateX(4px); }
.settings-detail { min-width: 0; min-height: 0; overflow-y: auto; padding: 27px 0 24px clamp(28px, 5vw, 72px); }
.detail-content { position: relative; isolation: isolate; animation: detail-in .32s ease-out both; }
.detail-watermark { position: absolute; z-index: -1; right: 0; top: -34px; font-size: clamp(100px, 16vw, 220px); font-weight: 300; line-height: 1; letter-spacing: -.08em; color: rgba(179, 102, 111, .09); pointer-events: none; user-select: none; }
.detail-kicker { margin: 0 0 14px; text-transform: uppercase; }
h2 { font-size: clamp(36px, 4.5vw, 56px); }
.detail-rule { width: 48px; height: 2px; background: #cd8991; margin: 30px 0; }
.detail-message { margin: 0; font-size: 16px; line-height: 1.7; }
.detail-note { color: #987b7a; font-size: 13px; line-height: 1.7; }
.back { display: none; }
button:focus-visible { outline: 2px solid #ad7382; outline-offset: 4px; }
h1:focus, h2:focus { outline: none; }
@keyframes reveal { from { opacity: 0; transform: translateY(8px); } to { opacity: 1; transform: translateY(0); } }
@keyframes detail-in { from { opacity: 0; transform: translateX(12px); } to { opacity: 1; transform: translateX(0); } }
@media (prefers-reduced-motion: reduce) { .settings, .detail-content { animation: none; } .nav-items button, .nav-items button span { transition: none; } }
@media (max-width: 680px) {
  .settings-body { display: block; padding-top: 0; }
  .settings-nav, .settings-detail { height: 100%; padding: 16px 0 24px; border: 0; }
  .settings-detail, .show-detail .settings-nav { display: none; }
  .show-detail .settings-detail { display: block; }
  .back { display: inline-block; margin: 0 0 28px; }
}
</style>
