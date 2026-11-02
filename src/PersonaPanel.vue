<script setup lang="ts">
import {computed,nextTick,onMounted,ref,watch} from 'vue';
import {runtime} from './ipc';
import {openPersonaDirectory} from './onboarding';
import type {PersonaView,PersonaVersionView,ManagementResult} from './generated/protocol';
const hints:Record<string,string>={
 'MISSION.MD':'使命宣言：你为什么存在、长期要完成什么。',
 'IDENTITY.MD':'身份与表达：你是谁、如何思考与说话。',
 'RELATIONSHIP.MD':'与用户的关系：如何协作、建立信任与尊重边界。',
 'REDLINE.MD':'红线：诚实、隐私、授权与不可逾越的行为约束。',
 'DREAM.MD':'长期愿景：想成为什么，以及持续改进的方向。',
 'DARK.MD':'自我审视：已识别的弱点、失败模式与诱惑。',
};
const protectedFiles=new Set(['MISSION.MD','REDLINE.MD']);
const current=ref<PersonaView|null>(null),history=ref<PersonaVersionView[]>([]);
const selected=ref('MISSION.MD'),editing=ref(false),draft=ref(''),baseline=ref('');
const busy=ref(false),error=ref(''),notice=ref('');
const opening=ref(false),openError=ref('');
async function openWorkspace(){
 if(opening.value)return;
 opening.value=true;openError.value='';
 try{await openPersonaDirectory();}
 catch(e){openError.value=String(e);}
 finally{opening.value=false;}
}
const confirming=ref<{name:string;text:string}|null>(null);
const confirmDialog=ref<HTMLDialogElement|null>(null);
let pendingSave:((ok:boolean)=>void)|undefined;
const currentFile=computed(()=>current.value?.files.find(f=>f.name===selected.value));
const dirty=computed(()=>editing.value&&draft.value!==baseline.value);
function isProtected(name:string){return protectedFiles.has(name);}
function accept(result:ManagementResult){
 if(result.kind!=='persona')throw new Error('人格响应无效');
 current.value=result.data.current;history.value=result.data.history;
 if(!current.value.files.some(f=>f.name===selected.value)&&current.value.files.length)selected.value=current.value.files[0].name;
}
async function load(){
 busy.value=true;error.value='';notice.value='';
 try{accept(await runtime.operation({method:'persona_read'}));}
 catch(e){error.value=String(e);}
 finally{busy.value=false;}
}
function edit(){const file=currentFile.value;if(!file)return;baseline.value=file.text;draft.value=file.text;editing.value=true;error.value='';notice.value='';}
function discard(){editing.value=false;draft.value='';baseline.value='';if(pendingSave){pendingSave(false);pendingSave=undefined;}confirming.value=null;}
async function commit(name:string,text:string):Promise<boolean>{
 if(!current.value)return false;
 busy.value=true;error.value='';notice.value='';
 try{
  accept(await runtime.operation({method:'persona_save_file',params:{revision:current.value.revision,expected_update_id:current.value.update_id,update_id:crypto.randomUUID(),name,text}}));
  editing.value=false;draft.value='';baseline.value='';
  notice.value=`已保存 ${name}，发布为 revision ${current.value?.revision}。`;
  return true;
 }catch(e){error.value=String(e);return false;}
 finally{busy.value=false;}
}
function save():Promise<boolean>{
 if(!editing.value||!current.value)return Promise.resolve(true);
 const file=currentFile.value;
 if(!file)return Promise.resolve(false);
 if(draft.value===baseline.value){editing.value=false;return Promise.resolve(true);}
 if(file.name==='MISSION.MD'&&!draft.value.trim()){error.value='MISSION.MD 不能为空。';return Promise.resolve(false);}
 if(isProtected(file.name)){
  confirming.value={name:file.name,text:draft.value};
  return new Promise<boolean>(resolve=>{pendingSave=resolve;});
 }
 return commit(file.name,draft.value);
}
function confirmSave(){const pending=confirming.value;confirming.value=null;if(!pending){pendingSave?.(false);pendingSave=undefined;return;}void commit(pending.name,pending.text).then(ok=>{pendingSave?.(ok);pendingSave=undefined;});}
function cancelConfirm(){confirming.value=null;pendingSave?.(false);pendingSave=undefined;}
async function rollback(target:number){
 if(!current.value)return;
 busy.value=true;error.value='';notice.value='';
 try{
  accept(await runtime.operation({method:'persona_rollback',params:{revision:current.value.revision,target,update_id:crypto.randomUUID()}}));
  editing.value=false;
  notice.value=`已恢复 revision ${target}，并发布为 revision ${current.value?.revision}。`;
 }catch(e){error.value=String(e);}
 finally{busy.value=false;}
}
watch(confirming,async value=>{
 await nextTick();
 const dialog=confirmDialog.value;
 if(!dialog)return;
 if(value){if(!dialog.open)dialog.showModal?.();if(!dialog.open)dialog.setAttribute('open','');}
 else if(dialog.open)dialog.close?.();
});
defineExpose({dirty,save,discard});
onMounted(()=>{void load();});
</script>
<template><div class="panel persona">
 <p v-if="error" class="failure" role="alert">{{error}}</p>
 <p v-if="notice" class="notice" role="status">{{notice}}</p>
 <p v-if="current?.unpublished" class="notice" role="status">检测到外部修改：新回合已使用磁盘上的当前内容。保存任一文件即可发布为 revision {{current.revision+1}}。</p>
 <p v-if="current&&!current.initialized" class="notice" role="status">这个人格尚未初始化。完成首次启动引导后会写入名字与使命。</p>
 <div class="persona-intro">
  <h3 class="persona-name">{{current?.name||'Vivy'}}</h3>
  <p>六个核心人格文件按固定顺序逐字拼接，空文件跳过，整体包在 <code>&lt;persona&gt;</code> 中置于系统提示最前。</p>
  <p><code>MISSION.MD</code> 与 <code>REDLINE.MD</code> 仅由人工维护；AutoDream 只可改写 <code>IDENTITY.MD</code>、<code>RELATIONSHIP.MD</code>、<code>DREAM.MD</code>、<code>DARK.MD</code>。Recap 与 AutoDream 是内部流程，不在此编辑。</p>
  <small class="path">角色工作区：{{current?.directory||'读取中…'}}</small>
  <div class="actions"><button :disabled="busy||opening" @click="openWorkspace">{{opening?'正在打开…':'打开角色工作区'}}</button></div>
  <p v-if="openError" class="failure" role="alert">{{openError}}</p>
 </div>
 <template v-if="!editing">
  <div class="resource-list persona-files" role="group" aria-label="核心人格文件">
   <button v-for="file in current?.files||[]" :key="file.name" :class="{selected:selected===file.name}" :aria-pressed="selected===file.name" :disabled="busy" @click="selected=file.name">
    <span class="file-line"><strong>{{file.name}}</strong><em>{{file.required?'必需':'可选'}} · {{file.auto?'AutoDream 可改写':'仅人工'}}</em></span>
    <small>{{hints[file.name]||''}}</small>
   </button>
  </div>
  <div v-if="currentFile" class="resource-detail">
   <h3>{{currentFile.name}}<small>{{currentFile.required?'必需':'可选'}} · {{currentFile.auto?'AutoDream 可改写':'仅人工'}}</small></h3>
   <pre v-if="currentFile.text.trim()">{{currentFile.text}}</pre>
   <p v-else class="detail-note">此文件为空，组装时跳过。</p>
   <div class="actions"><button :disabled="busy" @click="edit">编辑</button><button :disabled="busy" @click="load">重新读取</button></div>
  </div>
  <template v-if="current">
   <h3>已发布历史</h3>
   <p v-if="!history.length" class="detail-note">还没有已发布的历史版本；当前使用内置默认人格。</p>
   <details v-for="version in history" :key="version.revision">
    <summary>revision {{version.revision}}<small>来源：{{version.sources.join('、')||'未知'}}</small></summary>
    <pre>{{version.text}}</pre>
    <small v-if="version.truncated">内容过长，仅显示开头部分。</small>
    <div class="actions"><button :disabled="busy||version.revision===current.revision" @click="rollback(version.revision)">恢复此版本</button></div>
   </details>
   <p class="detail-note">恢复整版会创建新的 revision，并把六个文件一起写回。</p>
  </template>
 </template>
 <template v-else>
  <label>{{selected}}<textarea v-model="draft" rows="18" :disabled="busy"/></label>
  <p v-if="isProtected(selected)" class="detail-note">MISSION / REDLINE 由人工维护，保存前会再次确认。</p>
  <p v-if="selected==='MISSION.MD'&&!draft.trim()" class="failure" role="alert">MISSION.MD 不能为空。</p>
  <div class="actions"><button :disabled="busy||(selected==='MISSION.MD'&&!draft.trim())" @click="save">保存</button><button :disabled="busy" @click="discard">放弃编辑</button></div>
 </template>
 <dialog ref="confirmDialog" class="confirm" aria-label="确认修改受保护人格文件" @cancel.prevent="cancelConfirm"><div><p>确定修改 {{confirming?.name}}？</p><p class="detail-note">MISSION 与 REDLINE 只由人工维护，AutoDream 不会改写它们。</p><button @click="confirmSave">确认保存</button><button @click="cancelConfirm">取消</button></div></dialog>
</div></template>
<style scoped>
.persona-intro{margin:0 0 18px}.persona-name{margin:0 0 10px;font-size:24px;font-weight:500;color:#7a3f50}.persona-intro p{margin:0 0 6px;color:#6e4c53}.persona-intro code{font-size:12px;background:#a96f7c14;padding:1px 4px;border-radius:4px}
.notice{margin:0 0 14px;padding:9px 12px;border-left:3px solid #cd8991;background:#a96f7c12;color:#6e4c53}
.persona-files button{padding:10px 12px}.file-line{display:flex;align-items:baseline;justify-content:space-between;gap:12px}.file-line strong{font-weight:600}.file-line em{font-style:normal;font-size:11px;color:#987b7a;white-space:nowrap}
.resource-detail h3{display:flex;align-items:baseline;gap:10px;margin:0 0 8px}.resource-detail h3 small{display:inline}
</style>
