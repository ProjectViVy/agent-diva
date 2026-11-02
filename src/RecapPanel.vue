<script setup lang="ts">
import {onMounted,ref} from 'vue';import {runtime} from './ipc';import type {RecapView} from './generated/protocol';
const value=ref<RecapView|null>(null),busy=ref(false),error=ref('');
async function read(refresh=false){busy.value=true;error.value='';try{const result=await runtime.request({method:refresh?'recap_refresh':'recap_read'});if(!('summary' in result))throw new Error('Recap 响应无效');value.value=result;}catch(e){error.value=String(e);}finally{busy.value=false;}}
onMounted(()=>read());
</script>
<template><div class="panel"><p v-if="error||value?.error" role="alert" class="failure">{{error||value?.error}}</p><p>已提交 revision {{value?.revision??0}} · {{value?.refresh_pending?'刷新待处理':'无待处理刷新'}}</p><pre>{{value?.summary||'尚无已提交摘要'}}</pre><h3>覆盖范围</h3><p v-for="(count,session) in value?.covered" :key="session" class="path">{{session}} · {{count}}</p><h3>待处理工作</h3><p v-for="job in value?.jobs" :key="job.id">{{job.id}} · {{job.phase}} · {{job.spans}} spans</p><p>Memory：{{runtime.state.snapshot?.memory.state}}</p><div class="actions"><button :disabled="busy" @click="read()">重新读取</button><button :disabled="busy" @click="read(true)">请求刷新摘要</button></div></div></template>
