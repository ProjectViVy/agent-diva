<script setup lang="ts">
import {computed,onMounted,ref} from 'vue';import {runtime} from './ipc';import type {SkillDocument,Method} from './generated/protocol';
const skills=ref<SkillDocument[]>([]),selected=ref(''),busy=ref(false),error=ref(''),editing=ref(false),importing=ref(false),source=ref(''),name=ref(''),text=ref('');
const current=computed(()=>skills.value.find(s=>s.name===selected.value));
const dirty=computed(()=>importing.value?!!(source.value||name.value):editing.value&&text.value!==current.value?.text);
async function run(command:Method){busy.value=true;error.value='';try{const result=await runtime.operation(command);if(result.kind!=='skills')throw new Error('Skills 响应无效');skills.value=result.data.skills;return true;}catch(e){error.value=String(e);return false;}finally{busy.value=false;}}
async function save(){const ok=importing.value?await run({method:'skill_import',params:{source:source.value,name:name.value}}):current.value&&await run({method:'skill_save',params:{name:current.value.name,revision:current.value.revision,text:text.value}});if(ok){editing.value=false;importing.value=false;source.value='';name.value='';}return !!ok;}
async function remove(){if(current.value)await run({method:'skill_remove',params:{name:current.value.name,revision:current.value.revision}});}
function discard(){editing.value=false;importing.value=false;source.value='';name.value='';}
defineExpose({dirty,save,discard});onMounted(()=>run({method:'skills_read'}));
</script>
<template><div class="panel"><p v-if="error" role="alert" class="failure">{{error}}</p>
 <form v-if="importing" @submit.prevent="save"><fieldset :disabled="busy"><label>本地源目录<input v-model="source" required placeholder="包含 SKILL.md 的完整目录路径"/></label><label>Skill 名称<input v-model="name" required pattern="[a-z0-9-]+"/></label><small>复制到应用数据目录；源目录保留原样。</small><div class="actions"><button type="submit">导入</button><button type="button" @click="discard">放弃</button></div></fieldset></form>
 <template v-else-if="editing"><label>SKILL.md<textarea v-model="text" rows="18" :disabled="busy"/></label><div class="actions"><button :disabled="busy" @click="save">保存</button><button :disabled="busy" @click="discard">放弃编辑</button></div></template>
 <template v-else><div class="actions"><button :disabled="busy" @click="importing=true">导入本地目录</button><button :disabled="busy" @click="run({method:'skills_refresh'})">刷新清单</button></div><div class="resource-list"><button v-for="skill in skills" :key="skill.name" :class="{selected:selected===skill.name}" @click="selected=skill.name">{{skill.name}}<small>{{skill.description}}</small></button></div><div v-if="current" class="resource-detail"><pre>{{current.text}}</pre><div v-if="current.editable" class="actions"><button :disabled="busy" @click="text=current.text;editing=true">编辑</button><button :disabled="busy" @click="remove">移入恢复目录</button></div><small v-else>外部发现的 Skill 只读；导入副本后可管理。</small></div></template>
</div></template>
