<script setup lang="ts">
import { computed, onUnmounted, ref } from 'vue';
import ProvidersPanel from './ProvidersPanel.vue';
import { runtime } from './ipc';
import { openPersonaDirectory } from './onboarding';
import type { ProbeReport, PythonReport } from './generated/protocol';
const emit = defineEmits<{ done: [] }>();
const step = ref<1 | 2 | 3 | 4>(1);
const providers = ref<{ save(): Promise<boolean> } | null>(null);
const testing = ref(false);
const providerError = ref('');
const probe = ref<ProbeReport | null>(null);
const providerReady = computed(() => !!probe.value && probe.value.checks.length === 3 && probe.value.checks.every(check => check.passed));
const state = runtime.state;
async function verifyProvider() {
  const panel = providers.value;
  if (!panel || testing.value) return;
  testing.value = true; providerError.value = ''; probe.value = null;
  try {
    if (!await panel.save()) { providerError.value = '请先完成并保存供应商配置。'; return; }
    const view = await runtime.providers();
    const reference = view.config.default_model;
    if (!reference) { providerError.value = '请先在供应商设置中选择默认聊天模型。'; return; }
    const result = await runtime.operation({ method: 'provider_test', params: { generation: view.generation, reference } });
    if (result.kind !== 'provider_test') throw new Error('供应商测试响应无效');
    probe.value = result.data;
    if (result.data.checks.length !== 3 || result.data.checks.some(check => !check.passed)) providerError.value = '文本、流式与工具三项检查全部通过后才能继续。';
  } catch (error) {
    providerError.value = `供应商错误：${String(error)}`;
  } finally {
    testing.value = false;
  }
}
const description = ref('');
const working = ref(false);
const personaError = ref('');
const onboarded = ref<{ name: string; directory: string; fallback: boolean; existing: boolean } | null>(null);
const descriptionBytes = computed(() => new TextEncoder().encode(description.value).length);
const canInitialize = computed(() => !working.value && !!description.value.trim() && descriptionBytes.value <= 8192);
async function initialize(fallback = false) {
  if (working.value || (!fallback && !canInitialize.value)) return;
  working.value = true; personaError.value = '';
  try {
    const result = await runtime.operation({
      method: 'persona_initialize',
      params: { description: description.value.trim(), update_id: crypto.randomUUID(), fallback },
    }, 180000);
    if (result.kind !== 'persona_onboarded') throw new Error('初始化响应无效');
    onboarded.value = { name: result.data.name, directory: result.data.current.directory, fallback: result.data.fallback || fallback, existing: false };
    enterWorkspace();
  } catch (error) {
    const message = String(error);
    if (message.includes('already initialized')) await showExisting();
    else personaError.value = message;
  } finally {
    working.value = false;
  }
}
async function showExisting() {
  try {
    const result = await runtime.operation({ method: 'persona_read' });
    if (result.kind !== 'persona') throw new Error('人格响应无效');
    onboarded.value = { name: result.data.current.name, directory: result.data.current.directory, fallback: false, existing: true };
    personaError.value = '';
    enterWorkspace();
  } catch (error) {
    personaError.value = String(error);
  }
}
const python = ref<PythonReport | null>(null);
const wantMemory = ref(true);
const workspaceBusy = ref(false);
const workspaceSkipped = ref(false);
const workspaceError = ref('');
let timer: ReturnType<typeof setTimeout> | undefined;
let gone = false;
const memory = computed(() => state.snapshot?.memory ?? null);
const memoryEnabled = computed(() => memory.value?.state === 'unchecked' || memory.value?.state === 'ready' || memory.value?.state === 'closed');
const pythonReady = computed(() => python.value?.outcome === 'ready');
const pythonBusy = computed(() => workspaceBusy.value || !!python.value?.phase);
const pythonLabel = computed(() => {
  if (python.value?.phase) return `初始化中（${python.value.phase}）`;
  switch (python.value?.outcome) {
    case 'ready': return '已就绪';
    case 'error': return '初始化失败';
    case 'not_init': return '未初始化';
    default: return '正在读取…';
  }
});
const memoryLabel = computed(() => {
  switch (memory.value?.state) {
    case 'ready': return '已就绪';
    case 'unchecked': return '已启用，待初始化';
    case 'error': return '初始化出错';
    case 'closed': return '已关闭';
    case 'off': return '未启用';
    default: return '正在读取…';
  }
});
const workspaceAction = computed(() => {
  if (pythonReady.value) return '继续';
  if (pythonBusy.value) return '正在初始化…';
  if (workspaceError.value) return '重试初始化';
  return '初始化工作区';
});
function enterWorkspace() {
  step.value = 3;
  void readWorkspace();
}
async function readWorkspace() {
  clearTimeout(timer);
  try {
    const report = await runtime.request({ method: 'python_read' });
    if ('outcome' in report) python.value = report;
  } catch (error) {
    workspaceError.value = String(error);
  }
  if (!gone && python.value?.phase) timer = setTimeout(readWorkspace, 1500);
}
async function prepareWorkspace() {
  if (pythonBusy.value) return;
  workspaceBusy.value = true; workspaceError.value = ''; workspaceSkipped.value = false;
  let settled = false;
  try {
    const result = await runtime.operation(
      { method: 'python_prepare', params: { managed: true, memory: wantMemory.value } },
      2_200_000,
    );
    if (result.kind === 'python') python.value = result.data;
    settled = true;
  } catch (error) {
    workspaceError.value = `工作区初始化失败：${String(error)}`;
  } finally {
    workspaceBusy.value = false;
  }
  if (!settled) await readWorkspace();
}
async function cancelWorkspace() {
  try { await runtime.request({ method: 'python_cancel' }); }
  catch (error) { workspaceError.value = String(error); }
  await readWorkspace();
}
async function finishWorkspace() {
  workspaceSkipped.value = !pythonReady.value;
  await runtime.synchronize().catch(() => {});
  step.value = 4;
}
onUnmounted(() => { gone = true; clearTimeout(timer); });
const opening = ref(false);
const openError = ref('');
async function openWorkspace() {
  if (opening.value) return;
  opening.value = true; openError.value = '';
  try { await openPersonaDirectory(); }
  catch (error) { openError.value = String(error); }
  finally { opening.value = false; }
}
</script>
<template>
 <section class="onboarding" aria-label="首次启动初始化">
  <header class="intro">
   <p class="kicker">First meeting</p>
   <h1>让我认识你<span>.</span></h1>
   <p class="lede">在开始对话之前，先连接一个可用的模型，然后用一句话告诉我：我是谁，我的使命是什么。</p>
   <ol class="steps" aria-label="初始化步骤">
    <li :class="{active:step===1,done:step>1}"><span>01</span>连接模型</li>
    <li :class="{active:step===2,done:step>2}"><span>02</span>第一次认识你</li>
    <li :class="{active:step===3,done:step>3}"><span>03</span>准备工作区</li>
    <li :class="{active:step===4,done:false}"><span>04</span>完成</li>
   </ol>
  </header>
  <p v-if="!state.connected" class="connection" role="status">尚未连接核心。<button :disabled="state.connecting" @click="runtime.connect">{{state.connecting?'正在连接…':'重试连接'}}</button></p>
  <p v-if="state.error" class="failure" role="alert">{{state.error}}</p>

  <section v-if="step===1" class="card" aria-labelledby="onboarding-provider-title">
   <h2 id="onboarding-provider-title">连接一个可用的大模型</h2>
   <p class="note">填写供应商密钥并保存，然后运行连接测试。文本、流式与工具三项检查全部通过后才能继续。</p>
   <ProvidersPanel ref="providers"/>
   <p v-if="providerError" class="failure" role="alert">{{providerError}}</p>
   <template v-if="probe">
    <ul class="checks" aria-label="连接测试结果">
     <li v-for="check in probe.checks" :key="check.kind" :class="{passed:check.passed}"><strong>{{check.kind}}</strong><span>{{check.passed?'通过':'失败'}}</span><small>{{check.detail}}</small></li>
    </ul>
   </template>
   <div class="actions">
    <button class="primary" :disabled="testing||!state.connected" @click="verifyProvider">{{testing?'正在测试…':'保存并测试'}}</button>
    <button :disabled="!providerReady||testing" @click="step=2">下一步</button>
   </div>
  </section>

  <section v-else-if="step===2" class="card" aria-labelledby="onboarding-persona-title">
   <h2 id="onboarding-persona-title">第一次认识你</h2>
   <p class="greeting">第一次认识你，请介绍一下我是谁吧，以及我的使命。</p>
   <label class="prompt">你的描述<textarea v-model="description" rows="7" :disabled="working" placeholder="例如：你叫岚，你存在的意义是陪我把复杂的问题想清楚；你说话直接、安静，会记得我们长期在做的事。"/></label>
   <p class="counter" :class="{over:descriptionBytes>8192}">{{descriptionBytes}} / 8192 字节</p>
   <p class="note">描述会交给模型提取一次，并直接写入你的全局 Laputa 目录，不会先展示预览。表达不清时会使用内置人格与随机名字，之后可以在设置中修改。</p>
   <p v-if="personaError" class="failure" role="alert">{{personaError}}</p>
   <div class="actions">
    <button class="primary" :disabled="!canInitialize" @click="initialize(false)">{{working?'正在整理…':'开始认识你'}}</button>
    <button :disabled="working||!state.connected" @click="initialize(true)">使用内置人格</button>
   </div>
  </section>

  <section v-else-if="step===3" class="card" aria-labelledby="onboarding-workspace-title">
   <h2 id="onboarding-workspace-title">准备工作区</h2>
   <p class="note">Python 工作台负责代码执行与技能运行；原生记忆负责长期协作内容。可以现在一起初始化，也可以稍后在设置 → 运行环境中补做。</p>
   <dl class="result">
    <dt>Python</dt><dd>{{pythonLabel}}<small v-if="python?.detail">{{python.detail}}</small></dd>
    <dt>记忆后端</dt><dd>{{memoryLabel}}<small v-if="memory?.issue">{{memory.issue}}</small></dd>
   </dl>
   <label v-if="!memoryEnabled" class="prompt inline"><input v-model="wantMemory" type="checkbox" :disabled="pythonBusy"/>启用原生记忆（本地 MemPalace，默认开启）</label>
   <p v-if="workspaceError" class="failure" role="alert">{{workspaceError}}</p>
   <div class="actions">
    <button class="primary" :disabled="pythonBusy" @click="pythonReady?finishWorkspace():prepareWorkspace()">{{workspaceAction}}</button>
    <button v-if="python?.phase" :disabled="workspaceBusy" @click="cancelWorkspace">取消初始化</button>
    <button v-else :disabled="workspaceBusy" @click="finishWorkspace">稍后初始化</button>
   </div>
  </section>

  <section v-else class="card" aria-labelledby="onboarding-done-title">
   <h2 id="onboarding-done-title">认识完成</h2>
   <p class="greeting">{{onboarded?.existing?'这个人格已经存在，无需重新生成。':'从现在起，我叫 '+onboarded?.name+'。'}}</p>
   <dl class="result">
    <dt>名字</dt><dd>{{onboarded?.name}}</dd>
    <dt>角色工作区</dt><dd class="path">{{onboarded?.directory}}</dd>
    <dt>来源</dt><dd>{{onboarded?.existing?'已有的人格':onboarded?.fallback?'内置人格（表述不清，已随机命名）':'模型提取'}}</dd>
    <dt>Python</dt><dd>{{pythonBusy?'初始化中':pythonReady?'已就绪':workspaceSkipped?'稍后初始化':'未初始化'}}</dd>
    <dt>记忆后端</dt><dd>{{memoryLabel}}{{workspaceSkipped&&!memoryEnabled?'（稍后初始化）':''}}</dd>
   </dl>
   <p v-if="openError" class="failure" role="alert">{{openError}}</p>
   <div class="actions">
    <button :disabled="opening" @click="openWorkspace">{{opening?'正在打开…':'打开角色工作区'}}</button>
    <button class="primary" @click="emit('done')">开始对话</button>
   </div>
  </section>
 </section>
</template>
<style scoped>
.onboarding{flex:1;min-height:0;overflow-y:auto;padding:clamp(24px,5vw,56px) clamp(22px,6vw,90px) 48px;color:#563b41}
.intro{max-width:760px;margin:0 0 26px}
.kicker{margin:0 0 10px;color:#a77a7b;font-size:11px;letter-spacing:.16em;text-transform:uppercase}
h1{margin:0;font-size:clamp(38px,5.4vw,64px);font-weight:500;letter-spacing:-.055em;line-height:1.05}
h1 span{color:#cc7e85}
.lede{margin:14px 0 0;max-width:60ch;color:#7d5a60;line-height:1.75;font-size:14px}
.steps{display:flex;flex-wrap:wrap;gap:10px 26px;margin:22px 0 0;padding:0;list-style:none;font-size:12px;color:#ad8b88}
.steps li{display:flex;align-items:center;gap:8px}
.steps li span{display:grid;place-items:center;width:22px;height:22px;border:1px solid #cba3a3;border-radius:50%;font-size:10px}
.steps li.active{color:#8a4a5b}
.steps li.active span{border-color:#b3666f;background:#b3666f;color:#fff}
.steps li.done span{border-color:#a9bfae;background:#a9bfae;color:#fff}
.connection{margin:0 0 16px;color:#7d5a60;font-size:13px}
.connection button{margin-left:10px}
.card{max-width:760px;padding:22px 24px 20px;border:1px solid rgba(169,111,124,.3);border-radius:16px;background:rgba(255,250,246,.55);box-shadow:0 8px 40px rgba(126,76,70,.06)}
.card h2{margin:0 0 8px;font-size:20px;font-weight:500;color:#6d3f4c}
.note{margin:8px 0 14px;color:#987b7a;font-size:13px;line-height:1.7}
.greeting{margin:0 0 14px;font-size:16px;line-height:1.7;color:#6d3f4c}
.prompt{display:flex;flex-direction:column;gap:6px;font-size:12px;color:#80545e}
.prompt textarea{min-height:150px;max-height:340px;resize:vertical;border:1px solid #a96f7c55;border-radius:9px;padding:11px 13px;background:#fff9;color:#533941;font:inherit;font-size:14px;line-height:1.65}
.prompt.inline{flex-direction:row;align-items:center;gap:9px;margin:12px 0 0;font-size:13px;color:#7d5a60;cursor:pointer}
.prompt.inline input{accent-color:#a96f7c}
.counter{margin:6px 0 0;text-align:right;font-size:11px;color:#a77a7b}
.counter.over{color:#913b48}
.checks{display:flex;flex-direction:column;gap:6px;margin:14px 0 0;padding:0;list-style:none;font-size:12px}
.checks li{display:grid;grid-template-columns:72px 40px 1fr;gap:10px;align-items:baseline;padding:8px 10px;border-radius:8px;background:#a96f7c14;color:#8a353b}
.checks li.passed{background:#9fbfa81f;color:#4f6b57}
.checks li span{font-weight:600}
.checks li small{color:inherit;opacity:.8;overflow-wrap:anywhere}
.actions{display:flex;flex-wrap:wrap;gap:10px;margin:18px 0 0}
.actions button{padding:9px 16px;border:1px solid #a96f7c55;border-radius:9px;background:#fff8;color:#784552;font:inherit;cursor:pointer}
.actions button.primary{border-color:#a96f7c;background:#a96f7c;color:#fff}
.actions button:disabled{opacity:.5;cursor:default}
.failure{margin:10px 0 0;color:#913b48;font-size:13px;white-space:pre-wrap;overflow-wrap:anywhere}
.result{display:grid;grid-template-columns:88px 1fr;gap:8px 14px;margin:18px 0 0;font-size:13px}
.result dt{color:#a77a7b}
.result dd{margin:0;color:#6d3f4c}
.result dd.path{overflow-wrap:anywhere}
.result dd small{display:block;margin-top:3px;color:#987b7a;font-size:11px;line-height:1.6;overflow-wrap:anywhere}
</style>
