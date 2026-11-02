<script setup lang="ts">
import {computed,onMounted,ref} from 'vue';
import {runtime} from './ipc';
import type {PersonaView,PromptView,ManagementResult} from './generated/protocol';
const current=ref<PersonaView|null>(null),history=ref<PersonaView[]>([]),prompts=ref<PromptView[]>([]);
const selected=ref('persona'),editing=ref(false),draft=ref(''),baseline=ref(''),busy=ref(false),error=ref('');
const prompt=computed(()=>prompts.value.find(p=>p.name===selected.value));
const dirty=computed(()=>editing.value&&draft.value!==baseline.value);
function accept(result:ManagementResult){if(result.kind==='persona'){current.value=result.data.current;history.value=result.data.history;}else if(result.kind==='prompts')prompts.value=result.data.prompts;}
async function load(){busy.value=true;error.value='';try{accept(await runtime.operation({method:'persona_read'}));accept(await runtime.operation({method:'prompts_read'}));}catch(e){error.value=String(e);}finally{busy.value=false;}}
function edit(){baseline.value=selected.value==='persona'?current.value?.text??'':prompt.value?.text??'';draft.value=baseline.value;editing.value=true;}
async function save(){busy.value=true;error.value='';try{
 if(selected.value==='persona'&&current.value)accept(await runtime.operation({method:'persona_update',params:{revision:current.value.revision,expected_update_id:current.value.update_id,update_id:crypto.randomUUID(),text:draft.value}}));
 else if(prompt.value)accept(await runtime.operation({method:'prompt_save',params:{name:prompt.value.name,revision:prompt.value.revision,text:draft.value}}));
 else throw new Error('请先读取当前内容');editing.value=false;return true;
}catch(e){error.value=String(e);return false;}finally{busy.value=false;}}
async function rollback(target:number){if(!current.value)return;busy.value=true;error.value='';try{accept(await runtime.operation({method:'persona_rollback',params:{revision:current.value.revision,target,update_id:crypto.randomUUID()}}));}catch(e){error.value=String(e);}finally{busy.value=false;}}
defineExpose({dirty,save,discard:()=>editing.value=false});onMounted(load);
</script>
<template><div class="panel"><p v-if="error" class="failure" role="alert">{{error}}</p>
 <div v-if="!editing" class="resource-list"><button :class="{selected:selected==='persona'}" @click="selected='persona'">Persona <small>revision {{current?.revision}}</small></button><button v-for="p in prompts" :key="p.name" :class="{selected:selected===p.name}" @click="selected=p.name">{{p.name}}<small>有效来源：{{p.source}}</small></button></div>
 <div class="resource-detail"><template v-if="editing"><label>内容<textarea v-model="draft" rows="16" :disabled="busy"/></label><div class="actions"><button :disabled="busy" @click="save">保存</button><button :disabled="busy" @click="editing=false">放弃编辑</button></div></template>
 <template v-else><pre>{{selected==='persona'?current?.text:prompt?.text}}</pre><div class="actions"><button :disabled="busy||!current" @click="edit">编辑</button><button :disabled="busy" @click="load">重新读取</button></div>
 <template v-if="selected==='persona'"><h3>已生效历史</h3><details v-for="revision in history" :key="revision.revision"><summary>revision {{revision.revision}}</summary><pre>{{revision.text}}</pre><button :disabled="busy||revision.revision===current?.revision" @click="rollback(revision.revision)">恢复此版本</button></details></template></template></div>
</div></template>
