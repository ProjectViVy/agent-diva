<script setup lang="ts">
import { onMounted,onUnmounted,ref } from 'vue';
import type {PythonReport} from './generated/protocol';
import { runtime,diagnostics } from './ipc';
const logs=ref('');async function readLogs(){try{logs.value=await diagnostics();}catch(e){error.value=String(e);}}
const s=runtime.state;
const python=ref<PythonReport|null>(null),busy=ref(false),error=ref('');let timer:ReturnType<typeof setTimeout>|undefined;let gone=false;
async function read(){try{const r=await runtime.request({method:'python_read'});if('outcome' in r)python.value=r;}catch(e){error.value=String(e);}if(!gone&&python.value?.phase)timer=setTimeout(read,1500);}
async function prepare(){busy.value=true;error.value='';try{const job=runtime.operation({method:'python_prepare',params:{managed:true}});timer=setTimeout(read,200);const r=await job;if(r.kind==='python')python.value=r.data;}catch(e){error.value=String(e);}finally{busy.value=false;}}
async function cancel(){try{await runtime.request({method:'python_cancel'});await read();}catch(e){error.value=String(e);}}
onMounted(read);onUnmounted(()=>{gone=true;clearTimeout(timer);});
</script>
<template><div class="panel"><p>{{s.connected?'核心已连接':'核心未连接'}}</p><p v-if="s.error" class="failure">{{s.error}}</p><p class="path">{{s.directory}}</p><p>Core {{s.snapshot?.core_version}} · Protocol {{s.snapshot?.protocol_version}}</p><button :disabled="s.connecting" @click="runtime.connect">重新连接</button><dl v-if="s.snapshot"><template v-for="(value,name) in {model:s.snapshot.model,python:s.snapshot.python,memory:s.snapshot.memory}" :key="name"><dt>{{name}}</dt><dd>{{value.state}} · 编译 {{value.compiled?'是':'否'}} · 配置 {{value.configured?'是':'否'}} · 就绪 {{value.ready?'是':'否'}}</dd></template></dl><h3>Python 初始化</h3><p v-if="error" class="failure" role="alert">{{error}}</p><p>{{python?.outcome}} · {{python?.phase}}</p><pre>{{python?.detail}}</pre><small>显式初始化会安装锁定的工作台依赖。失败或取消会保留已有环境。</small><div class="actions"><button :disabled="busy||!!python?.phase" @click="prepare">初始化 / 重试</button><button :disabled="!python?.phase" @click="cancel">取消初始化</button><button @click="read">重新读取</button></div><details><summary>宿主日志</summary><button @click="readLogs">读取最近日志</button><pre>{{logs}}</pre></details><details><summary>已组装组件</summary><p v-for="plugin in s.snapshot?.plugins" :key="plugin.name">{{plugin.name}}<small>{{plugin.package}}</small></p></details></div></template>
