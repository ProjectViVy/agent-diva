<script setup lang="ts">
import { nextTick, ref, watch } from "vue";
import ProvidersPanel from './ProvidersPanel.vue';
import RuntimePanel from './RuntimePanel.vue';
import ManagedPanel from './ManagedPanel.vue';
import McpPanel from './McpPanel.vue';
import SkillsPanel from './SkillsPanel.vue';
import PersonaPanel from './PersonaPanel.vue';
import RecapPanel from './RecapPanel.vue';
import TasksPanel from './TasksPanel.vue';
import GeneralPanel from './GeneralPanel.vue';
type Editor={dirty:boolean;save:()=>Promise<boolean>;discard:()=>void};
const provider=ref<Editor|null>(null);
const confirm=ref(false);const leaveDialog=ref<HTMLDialogElement|null>(null);
watch(confirm,async visible=>{await nextTick();if(visible)leaveDialog.value?.showModal();else leaveDialog.value?.close();});let resolveLeave:((leave:boolean)=>void)|undefined;
async function canLeave(){if(!provider.value?.dirty)return true;confirm.value=true;return new Promise<boolean>(resolve=>{resolveLeave=resolve;});}
async function decide(action:'save'|'discard'|'stay'){if(action==='save'&&!await provider.value?.save())return;if(action==='discard')provider.value?.discard();confirm.value=false;resolveLeave?.(action!=='stay');}
defineExpose({canLeave});

const emit = defineEmits<{ close: [] }>();
const sections = [
  { id: "providers", label: "供应商", subtitle: "Providers", description: "" },
  { id: "mcp", label: "连接", subtitle: "MCP", description: "" },
  { id: "skills", label: "技能", subtitle: "Skills", description: "" },
  { id: "managed", label: "托管技能", subtitle: "Managed Skills", description: "" },
  { id: "persona", label: "人格与提示词", subtitle: "Persona / Prompts", description: "" },
  { id: "recap", label: "回顾", subtitle: "Recap", description: "" },
  { id: "tasks", label: "任务", subtitle: "Tasks", description: "" },
  { id: "runtime", label: "运行环境", subtitle: "Runtime", description: "" },
  { id: "general", label: "通用", subtitle: "General", description: "" },
] as const;
const selected = ref<(typeof sections)[number]>(sections[0]);
const showDetail = ref(false);
const detailHeading = ref<HTMLElement | null>(null);
const navHeading = ref<HTMLElement | null>(null);
async function select(section: (typeof sections)[number]) {
  if(section.id!==selected.value.id&&!await canLeave())return;
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
        <span class="detail-watermark" aria-hidden="true">{{ String(sections.findIndex(s=>s.id===selected.id)+1).padStart(2,'0') }}</span>
        <p class="detail-kicker">{{ selected.subtitle }}</p>
        <h2 id="settings-detail-title" ref="detailHeading" tabindex="-1">{{ selected.label }}</h2>
        <div class="detail-rule"></div>
        <ProvidersPanel v-if="selected.id==='providers'" ref="provider"/>
        <ManagedPanel v-else-if="selected.id==='managed'" ref="provider"/>
        <McpPanel v-else-if="selected.id==='mcp'" ref="provider"/>
        <SkillsPanel v-else-if="selected.id==='skills'" ref="provider"/>
        <PersonaPanel v-else-if="selected.id==='persona'" ref="provider"/>
        <RecapPanel v-else-if="selected.id==='recap'"/>
        <TasksPanel v-else-if="selected.id==='tasks'"/>
        <RuntimePanel v-else-if="selected.id==='runtime'"/>
        <GeneralPanel v-else/>

        </div>
      </article>
    </div>
    <dialog ref="leaveDialog" class="confirm" aria-label="未保存的编辑" @cancel.prevent="decide('stay')"><div><p>保存当前编辑？</p><button @click="decide('save')">保存</button><button @click="decide('discard')">放弃</button><button @click="decide('stay')">继续编辑</button></div></dialog>
  </section>
</template>

<style scoped>
.settings { animation: reveal .38s ease-out both; flex: 1; min-height: 0; display: flex; flex-direction: column; color: #563b41; }
.settings-top { display: flex; align-items: center; justify-content: flex-end; gap: 16px; padding: 20px clamp(24px, 5vw, 64px); }
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

<style>
.panel{min-width:0;font-size:14px;line-height:1.6}.panel button,.confirm button{padding:7px 12px;border:1px solid #a96f7c55;border-radius:7px;background:#fff7;color:#784552;cursor:pointer}.panel button:disabled{opacity:.5;cursor:default}.panel .actions{display:flex;flex-wrap:wrap;gap:8px;margin:16px 0}.panel fieldset{border:0;padding:0;min-width:0}.panel label{display:flex;flex-direction:column;gap:5px;margin:12px 0;font-size:12px;color:#80545e}.panel input,.panel select,.panel textarea{min-width:0;max-width:100%;border:1px solid #a96f7c55;border-radius:7px;padding:9px;background:#fff9;color:#533941;font:inherit}.panel .check{flex-direction:row;align-items:center}.panel .failure{color:#913b48;white-space:pre-wrap;overflow-wrap:anywhere}.panel .path{overflow-wrap:anywhere}.panel small{display:block;color:#987b7a}.resource-list{display:flex;flex-direction:column;gap:6px}.resource-list button{text-align:left}.resource-list .selected{border-color:#a96f7c;background:#fff9}.resource-detail,.model-form{margin:18px 0;padding:12px 0;border-bottom:1px solid #a96f7c30}.panel pre{max-height:260px;overflow:auto;white-space:pre-wrap;overflow-wrap:anywhere}.panel dd{margin:6px 0 16px}.confirm{border:0;padding:0;background:transparent;color:#533941;max-width:calc(100% - 32px)}.confirm::backdrop{background:#53394130}.confirm>div{padding:24px;border-radius:14px;background:#fff4ed;box-shadow:0 12px 60px #53394130}.confirm button{margin-right:8px}
</style>

<style>
.panel textarea{min-height:140px;max-height:420px;resize:vertical}
</style>
