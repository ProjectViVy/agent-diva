<script setup lang="ts">
import { computed, onMounted, ref } from 'vue';
import { runtime } from './ipc';
import type { Instance, ProvidersConfig, ProbeReport } from './generated/protocol';
function clone<T>(value:T):T{return JSON.parse(JSON.stringify(value)) as T;}
const probe=ref<ProbeReport|null>(null);const testing=ref(false);const testModel=ref('');
const selected=ref('');const editing=ref(false);const busy=ref(false);const error=ref('');
const providerId=ref('');const baseline=ref('');const generation=ref(0);const key=ref('');
const draft=ref<Instance>({protocol:'openai',credential:{source:'none'},models:[]});
const defaultModel=ref('');const thinking=ref('');
const providers=computed(()=>runtime.state.providers);
const current=computed(()=>providers.value?.config.instances?.[selected.value]);
const signature=()=>JSON.stringify({id:providerId.value,instance:draft.value,default:defaultModel.value,thinking:thinking.value,key:key.value});
const dirty=computed(()=>editing.value&&baseline.value!==signature());
function edit(id:string){
 selected.value=id;providerId.value=id;draft.value=clone(providers.value?.config.instances?.[id]??{protocol:'openai',credential:{source:'env',name:''},models:[]});
 generation.value=providers.value?.generation??0;defaultModel.value=providers.value?.config.default_model??'';thinking.value=providers.value?.config.default_thinking??'';
 key.value='';editing.value=true;baseline.value=signature();error.value='';
}
async function load(reload=false){busy.value=true;error.value='';try{await runtime.providers(reload);if(!selected.value)selected.value=Object.keys(providers.value?.config.instances??{})[0]??'';}catch(e){error.value=String(e);}finally{busy.value=false;}}
async function save(){
 if(busy.value)return false;busy.value=true;error.value='';
 try{
  const config:ProvidersConfig=clone(providers.value?.config??{instances:{}});
  config.instances??={};if(selected.value&&selected.value!==providerId.value)delete config.instances[selected.value];
  config.instances[providerId.value]=clone(draft.value);
  config.default_model=defaultModel.value||undefined;config.default_thinking=thinking.value||undefined;
  let response=await runtime.request({method:'providers_save',params:{generation:generation.value,config}});
  if(!('config'in response))throw new Error('供应商响应无效');
  runtime.state.providers=response;generation.value=response.generation;selected.value=providerId.value;
  if(key.value){response=await runtime.request({method:'provider_key',params:{generation:response.generation,provider:providerId.value,key:key.value}});if(!('config'in response))throw new Error('凭据响应无效');runtime.state.providers=response;key.value='';}
  editing.value=false;await runtime.synchronize();return true;
 }catch(e){error.value=String(e);return false;}finally{busy.value=false;}
}
async function remove(){
 if(!current.value)return;busy.value=true;error.value='';
 try{const config=clone(providers.value!.config);delete config.instances![selected.value];if(config.default_model?.startsWith(selected.value+'/'))delete config.default_model;
 const response=await runtime.request({method:'providers_save',params:{generation:providers.value!.generation,config}});if('config'in response)runtime.state.providers=response;selected.value='';await runtime.synchronize();
 }catch(e){error.value=String(e);}finally{busy.value=false;}
}
async function testConnection(){if(!providers.value)return;testing.value=true;error.value='';try{const reference=testModel.value||`${selected.value}/${current.value?.models?.[0]?.id??''}`;const result=await runtime.operation({method:'provider_test',params:{generation:providers.value.generation,reference}});if(result.kind==='provider_test')probe.value=result.data;}catch(e){error.value=String(e);}finally{testing.value=false;}}
function addModel(){(draft.value.models??=[]).push({id:'',context_window:32768,max_output_tokens:2048,tools:true});}
const models=computed(()=>Object.entries(providers.value?.config.instances??{}).flatMap(([id,p])=>(p.models??[]).map(m=>`${id}/${m.id}`)));
function credential(source:'env'|'stored'|'none'){draft.value.credential=source==='env'?{source,name:''}:{source};}
defineExpose({dirty,save,discard:()=>{editing.value=false;key.value='';}});
onMounted(()=>load());
</script>
<template>
 <div class="panel"><p v-if="error" role="alert" class="failure">{{error}}</p><p v-if="providers?.issue" class="failure">{{providers.issue}}</p>
  <div v-if="!editing" class="actions"><button :disabled="busy" @click="edit('')">添加供应商</button><button :disabled="busy" @click="load(true)">重新读取</button></div>
  <template v-if="!editing"><div class="resource-list"><button v-for="(p,id) in providers?.config.instances" :key="id" :class="{selected:selected===id}" @click="selected=String(id)"><strong>{{p.name||id}}</strong><small>{{p.protocol}} · {{p.base_url||p.preset}}</small></button></div>
   <div v-if="current" class="resource-detail"><h3>{{current.name||selected}}</h3><p>{{current.base_url||current.preset}}</p><p v-for="model in current.models" :key="model.id">{{model.id}}</p><p>凭据：{{current.credential?.source==='env'?current.credential.name:current.credential?.source}}</p><div class="actions"><button :disabled="busy" @click="edit(selected)">编辑</button><button :disabled="busy" @click="remove">删除配置</button></div><label>连接测试模型<select v-model="testModel"><option value="">第一个配置模型</option><option v-for="model in current.models" :key="model.id" :value="`${selected}/${model.id}`">{{model.id}}</option></select></label><button :disabled="testing||!current.models?.length" @click="testConnection">{{testing?'测试中…':'测试文本、流式与工具'}}</button><small>只发送固定测试提示与无副作用工具声明，不发送会话内容。</small><template v-if="probe"><p class="path">{{probe.reference}}</p><p v-for="check in probe.checks" :key="check.kind" :class="{failure:!check.passed}">{{check.kind}} · {{check.passed?'通过':'失败'}}<small>{{check.detail}}</small></p></template></div>
  </template>
  <form v-else @submit.prevent="save"><fieldset :disabled="busy"><label>实例 ID<input v-model="providerId" required pattern="[a-z0-9][a-z0-9_-]*"/></label><label>名称<input v-model="draft.name"/></label><label>协议<select v-model="draft.protocol"><option v-for="protocol in providers?.protocols" :key="protocol" :value="protocol">{{protocol}}</option></select></label><label>API endpoint<input v-model="draft.base_url" type="url" required/></label>
   <label>凭据来源<select :value="draft.credential?.source" @change="credential(($event.target as HTMLSelectElement).value as 'env'|'stored'|'none')"><option value="env">环境变量</option><option value="stored">本地凭据</option><option value="none">无需凭据</option></select></label>
   <label v-if="draft.credential?.source==='env'">环境变量名<input v-model="draft.credential.name" required/></label><label v-if="draft.credential?.source==='stored'">新密钥（留空保留已有值）<input v-model="key" type="password" autocomplete="new-password"/></label>
   <h3>模型</h3><div v-for="(model,index) in draft.models" :key="index" class="model-form"><label>原始 model ID<input v-model="model.id" required/></label><label>上下文窗口<input v-model.number="model.context_window" type="number" min="1" required/></label><label>最大输出<input v-model.number="model.max_output_tokens" type="number" min="1" required/></label><label class="check"><input v-model="model.tools" type="checkbox"/>工具调用</label><button type="button" @click="draft.models?.splice(index,1)">移除模型</button></div><button type="button" @click="addModel">添加模型</button>
   <label>默认模型<input v-model="defaultModel" list="configured-models" placeholder="实例 ID/原始 model ID"/></label><datalist id="configured-models"><option v-for="model in models" :key="model" :value="model"/></datalist><label>Reasoning<select v-model="thinking"><option value="">模型默认</option><option v-for="level in ['off','minimal','low','medium','high','xhigh','max']" :key="level">{{level}}</option></select></label>
   <div class="actions"><button type="submit">{{busy?'保存中…':'保存'}}</button><button type="button" @click="editing=false;key=''">放弃编辑</button></div>
  </fieldset></form>
 </div>
</template>
