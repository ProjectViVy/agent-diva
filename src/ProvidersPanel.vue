<script setup lang="ts">
import {computed,onMounted,ref,watch} from 'vue';
import {runtime} from './ipc';
import type {Instance,ProvidersConfig,ProbeReport} from './generated/protocol';
const clone=<T,>(v:T):T=>JSON.parse(JSON.stringify(v));
const selected=ref('deepseek'),editing=ref(false),defaultEditing=ref(false),busy=ref(false),testing=ref(false),error=ref(''),key=ref(''),baseline=ref(''),generation=ref(0),providerId=ref(''),variant=ref(''),needsKey=ref(false);
const draft=ref<Instance>({protocol:'openai',credential:{source:'stored'},models:[]});
const defaultModel=ref(''),testModel=ref(''),probe=ref<ProbeReport|null>(null);
const providers=computed(()=>runtime.state.providers);
const presets=computed(()=>providers.value?.presets??[]);
const preset=computed(()=>presets.value.find(p=>p.id===providerId.value));
const current=computed(()=>providers.value?.config.instances?.[selected.value]);
const entries=computed(()=>[...presets.value.map(p=>({id:p.id,name:p.name})),...Object.entries(providers.value?.config.instances??{}).filter(([id])=>!presets.value.some(p=>p.id===id)).map(([id,p])=>({id,name:p.name||id}))]);
const signature=()=>JSON.stringify({id:providerId.value,draft:draft.value,default:defaultModel.value,key:key.value});
const dirty=computed(()=>editing.value?baseline.value!==signature():defaultEditing.value&&defaultModel.value!==(providers.value?.config.default_model??''));
const modelOptions=computed(()=>{const instances=clone(providers.value?.config.instances??{});if(editing.value&&providerId.value)instances[providerId.value]=draft.value;return Object.entries(instances).flatMap(([id,p])=>(p.models??[]).filter(m=>!(p.disabled_models??[]).includes(m.id)).map(m=>({value:`${id}/${m.id}`,label:`${p.name||id} · ${m.id}`})));});
watch(selected,()=>{testModel.value='';probe.value=null;});
function edit(id:string){
 providerId.value=id;selected.value=id;key.value='';needsKey.value=false;error.value='';
 const template=presets.value.find(p=>p.id===id);const saved=providers.value?.config.instances?.[id];
 needsKey.value=!saved;
 draft.value=clone(saved??template?.variants[0]?.instance??{protocol:'openai',credential:{source:'stored'},models:[]});
 if(!saved&&template)draft.value.name=template.name;
 variant.value=template?.variants.find(v=>v.instance.base_url===draft.value.base_url)?.id??'';
 generation.value=providers.value?.generation??0;defaultModel.value=providers.value?.config.default_model??(id==='deepseek'?'deepseek/deepseek-flash':'');
 editing.value=true;defaultEditing.value=false;baseline.value=signature();
}
function switchVariant(){const v=preset.value?.variants.find(v=>v.id===variant.value);if(!v)return;
 const name=draft.value.name;draft.value=clone(v.instance);draft.value.name=name;key.value='';needsKey.value=true;
 if(defaultModel.value.startsWith(providerId.value+'/'))defaultModel.value=`${providerId.value}/${v.default_model}`;
}
function credential(source:'stored'|'env'|'none'){key.value='';draft.value.credential=source==='env'?{source,name:''}:{source};}
async function load(reload=false){busy.value=true;try{await runtime.providers(reload);}catch(e){error.value=String(e);}finally{busy.value=false;}}
function discard(){editing.value=false;defaultEditing.value=false;key.value='';}
async function persist(config:ProvidersConfig){const response=await runtime.request({method:'providers_save',params:{generation:generation.value,config}});if(!('config'in response))throw new Error('供应商响应无效');runtime.state.providers=response;generation.value=response.generation;}
async function save(){if(busy.value)return false;busy.value=true;error.value='';let partial=false;
 try{
 if(editing.value){
  if(!providerId.value)throw new Error('请填写实例 ID');
  if(providerId.value!==selected.value&&providers.value?.config.instances?.[providerId.value])throw new Error('实例 ID 已存在');
  if(draft.value.credential?.source==='stored'&&!key.value&&(!current.value||needsKey.value||providerId.value!==selected.value))throw new Error('请填写此接入方式的密钥');
  const config=clone(providers.value?.config??{instances:{}});config.instances??={};
  if(selected.value&&selected.value!==providerId.value){delete config.instances[selected.value];if(config.default_model?.startsWith(selected.value+'/'))delete config.default_model;}
  config.instances[providerId.value]=clone(draft.value);
  if(config.default_model?.startsWith(providerId.value+'/')&&!draft.value.models?.some(m=>`${providerId.value}/${m.id}`===config.default_model))delete config.default_model;
  if(needsKey.value&&draft.value.credential?.source==='stored')config.instances[providerId.value].credential={source:'env',name:'DIVA_PENDING_CREDENTIAL'};
  await persist(config);partial=true;selected.value=providerId.value;
  if(key.value){const response=await runtime.request({method:'provider_key',params:{generation:generation.value,provider:providerId.value,key:key.value}});if(!('config'in response))throw new Error('凭据响应无效');runtime.state.providers=response;generation.value=response.generation;key.value='';}
  needsKey.value=false;
 }
 const config=clone(providers.value!.config);
 // A plain save (for example the onboarding button) must not clear the default
 // model when the default picker was never opened.
 if((editing.value||defaultEditing.value)&&config.default_model!==(defaultModel.value||undefined)){config.default_model=defaultModel.value||undefined;delete config.default_thinking;await persist(config);}
 discard();await runtime.synchronize();return true;
 }catch(e){error.value=(partial?'配置已部分保存，默认选择尚未完成。':'')+String(e);return false;}finally{busy.value=false;}
}
function changeDefault(){defaultModel.value=providers.value?.config.default_model??'';generation.value=providers.value?.generation??0;defaultEditing.value=true;error.value='';}
async function remove(){busy.value=true;error.value='';try{generation.value=providers.value!.generation;const config=clone(providers.value!.config);delete config.instances![selected.value];if(config.default_model?.startsWith(selected.value+'/')){delete config.default_model;delete config.default_thinking;}await persist(config);await runtime.synchronize();}catch(e){error.value=String(e);}finally{busy.value=false;}}
async function testConnection(){if(!current.value||!providers.value)return;testing.value=true;error.value='';try{const reference=`${selected.value}/${testModel.value||current.value.models?.[0]?.id||''}`;const r=await runtime.operation({method:'provider_test',params:{generation:providers.value.generation,reference}});if(r.kind==='provider_test')probe.value=r.data;}catch(e){error.value=String(e);}finally{testing.value=false;}}
function endpointChanged(){variant.value='';needsKey.value=true;key.value='';if(draft.value.credential?.source==='env')draft.value.credential.name='';}
function addModel(){(draft.value.models??=[]).push({id:'',context_window:32768,max_output_tokens:8192,tools:true});}
defineExpose({dirty,save,discard});onMounted(()=>load());
</script>
<template><div class="panel">
<p v-if="error" role="alert" class="failure">{{error}}</p><p v-if="providers?.issue" class="failure">{{providers.issue}}</p>
<template v-if="!editing&&!defaultEditing">
<p>默认：{{providers?.config.default_model||'DeepSeek · deepseek-flash（待配置）'}} <button :disabled="busy||!modelOptions.length" @click="changeDefault">更改</button></p>
<div class="actions"><button :disabled="busy" @click="edit('')">自定义供应商</button><button :disabled="busy" @click="load(true)">重新读取</button></div>
<div class="resource-list"><button v-for="p in entries" :key="p.id" :class="{selected:selected===p.id}" @click="selected=p.id"><strong>{{p.name}}</strong><small>{{providers?.config.instances?.[p.id]?'已配置':'未配置'}}{{providers?.config.default_model?.startsWith(p.id+'/')?' · 默认':''}}</small></button></div>
<div class="resource-detail"><template v-if="current"><h3>{{current.name||selected}}</h3><p class="path">{{current.base_url}}</p><p v-for="m in current.models" :key="m.id">{{m.id}}</p><div class="actions"><button :disabled="busy||testing" @click="edit(selected)">编辑</button><button :disabled="busy||testing" @click="remove">删除配置</button></div>
<label>连接测试模型<select v-model="testModel"><option value="">第一个配置模型</option><option v-for="m in current.models" :key="m.id" :value="m.id">{{m.id}}</option></select></label><button :disabled="testing||busy" @click="testConnection">{{testing?'测试中…':'测试文本、流式与工具'}}</button><template v-if="probe"><p>{{probe.reference}}</p><p v-for="c in probe.checks" :key="c.kind">{{c.kind}} · {{c.passed?'通过':'失败'}} {{c.detail}}</p></template></template><button v-else :disabled="busy" @click="edit(selected)">配置 {{entries.find(p=>p.id===selected)?.name}}</button></div>
</template>
<form v-else @submit.prevent="save"><fieldset :disabled="busy">
<template v-if="editing"><h3>{{preset?.name||'自定义供应商'}}</h3>
<label v-if="!preset">实例 ID<input v-model="providerId" required pattern="[a-z][a-z0-9_-]*"/></label>
<label v-if="preset&&preset.variants.length>1">接入方式<select v-model="variant" @change="switchVariant"><option value="">自定义地址</option><option v-for="v in preset.variants" :key="v.id" :value="v.id">{{v.name}}</option></select></label>
<label>凭据来源<select :value="draft.credential?.source" @change="credential(($event.target as HTMLSelectElement).value as 'stored'|'env'|'none')"><option value="stored">本地密钥</option><option value="env">环境变量</option><option v-if="!preset" value="none">无需凭据</option></select></label>
<label v-if="draft.credential?.source==='stored'">API 密钥<input v-model="key" type="password" autocomplete="new-password" :placeholder="current&&!needsKey?'留空保留已有密钥':'请输入对应接入方式的密钥'"/></label><label v-if="draft.credential?.source==='env'">环境变量名<input v-model="draft.credential.name" required/></label>
<details><summary>高级设置</summary><label>名称<input v-model="draft.name"/></label><label v-if="!preset">协议<select v-model="draft.protocol"><option v-for="p in providers?.protocols" :key="p" :value="p">{{p}}</option></select></label><label>API endpoint<input v-model="draft.base_url" type="url" required @input="endpointChanged"/></label>
<div v-for="(m,index) in draft.models" :key="index" class="model-form"><label>原始 model ID<input v-model="m.id" required/></label><label>上下文窗口<input v-model.number="m.context_window" type="number" min="1"/></label><label>单次输出上限<input v-model.number="m.max_output_tokens" type="number" min="1"/></label><label class="check"><input v-model="m.tools" type="checkbox"/>工具调用</label><button type="button" @click="draft.models?.splice(index,1)">移除模型</button></div><button type="button" @click="addModel">添加模型</button></details></template>
<label>默认聊天模型<select v-model="defaultModel"><option value="">暂不设置</option><option v-for="m in modelOptions" :key="m.value" :value="m.value">{{m.label}}</option></select></label>
<div class="actions"><button type="submit">{{busy?'保存中…':'保存'}}</button><button type="button" @click="discard">放弃编辑</button></div>
</fieldset></form></div></template>
