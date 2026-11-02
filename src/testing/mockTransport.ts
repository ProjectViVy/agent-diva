import type {DesktopTransport} from '../runtime';
import type {Frame,Method,Snapshot,ProvidersView,ManagementResult,Reply,HistoryMessage,PersonaFileView,PersonaVersionView} from '../generated/protocol';
document.title='Diva · synthetic IPC test';
let handler:(f:Frame)=>void=()=>{},sequence=0;
const readiness={compiled:true,configured:true,ready:true,state:'ready',issue:null};
const snapshot:Snapshot={session:'synthetic-session',assistant:'assistant',core_version:'test-fixture',protocol_version:1,watermark:0,plugins:[],agents:[],shell:readiness,model:readiness,python:{...readiness,configured:false,ready:false,state:'unprepared'},memory:{...readiness,configured:false,ready:false,state:'off'},revision:0,active_task:null,tasks:[],questions:[],tools:[{id:'tool:1',task:'task:1',name:'read',phase:'completed',step:1,truncated:false,output:'Long tool output\n'.repeat(60)}]};
let providers:ProvidersView={presets:[],generation:1,issue:null,protocols:['openai','anthropic'],config:{default_model:'fixture/EXACT/model-id',instances:{fixture:{name:'Synthetic gateway',protocol:'openai',base_url:'http://127.0.0.1:9/v1',credential:{source:'env',name:'SYNTHETIC_ONLY'},models:[{id:'EXACT/model-id',context_window:32768,max_output_tokens:2048,tools:true}]}}}};
const history:HistoryMessage[]=[{id:'user:1',task:'task:1',role:'user',text:'这是合成 IPC 浏览器测试。',reasoning:'',status:'completed',step:0,truncated:false},{id:'assistant:1',task:'task:1',role:'assistant',text:'Markdown 与长内容测试。\n\n```typescript\n'+('const example = "'+'x'.repeat(180)+'";\n').repeat(8)+'```\n\n<div>Raw HTML is text.</div>',reasoning:'检查合成 IPC 的流式投影。',status:'completed',step:2,truncated:false}];
const persona={revision:0,updateId:'fixture-0',name:'Vivy',initialized:true,onboarded:'',files:[
 {name:'MISSION.MD',text:'帮助用户完成软件工作，并如实报告结果。',required:true,auto:false},
 {name:'IDENTITY.MD',text:'Synthetic persona',required:true,auto:true},
 {name:'RELATIONSHIP.MD',text:'与用户保持长期、直接的协作关系。',required:true,auto:true},
 {name:'REDLINE.MD',text:'诚实、保护隐私、不自我授权、尊重用户边界。',required:true,auto:false},
 {name:'DREAM.MD',text:'',required:false,auto:true},
 {name:'DARK.MD',text:'',required:false,auto:true},
] as PersonaFileView[],history:[] as PersonaVersionView[]};
const personaText=()=>persona.files.map(f=>f.text.trim()).filter(Boolean).join('\n\n');
const personaView=()=>({revision:persona.revision,update_id:persona.updateId,directory:'synthetic://.laputa/personas/main',name:persona.name,initialized:persona.initialized,text:personaText(),files:persona.files.map(f=>({...f})),unpublished:false});
const personaResult=():ManagementResult=>({kind:'persona',data:{current:personaView(),history:persona.history.map(h=>({...h}))}});
function publish(updateId:string,sources:string[]){persona.revision+=1;persona.updateId=updateId;persona.history=[...persona.history,{revision:persona.revision,update_id:updateId,text:personaText(),sources,truncated:false}];return personaResult();}
function frame(data:Reply):Frame{const f:Frame={v:1,id:'test',instance:'synthetic',seq:++sequence,type:'reply',data:structuredClone(data)};handler(f);return f;}
function operation(result:ManagementResult){const id=`operation:${sequence}`;const accepted=frame({operation:id,committed:false});queueMicrotask(()=>handler({v:1,id:null,instance:'synthetic',seq:++sequence,type:'management_completed',data:{operation:id,result,error:null}}));return accepted;}
async function request(command:Method):Promise<Frame>{switch(command.method){
 case 'snapshot':snapshot.watermark=sequence+1;return frame(snapshot);
 case 'history':return frame({messages:history,next_cursor:null});
 case 'providers_read':case 'providers_reload':return frame(providers);
 case 'providers_save':if(command.params.config.instances?.fixture?.name==='conflict')return {v:1,id:'test',instance:'synthetic',seq:++sequence,type:'error',data:{code:'conflict',message:'Synthetic external edit conflict; input must remain.'}};providers={...providers,generation:providers.generation+1,config:command.params.config};return frame(providers);
 case 'mcp_read':return operation({kind:'mcp',data:{revision:'fixture',connections:{}}});
 case 'skills_read':case 'skills_refresh':return operation({kind:'skills',data:{skills:[]}});
 case 'managed_read':return operation({kind:'managed',data:{current:null,history:[],candidates:[],issue:'Synthetic unprepared environment'}});
 case 'persona_read':return operation(personaResult());
 case 'persona_initialize':{
  const {description,update_id,fallback}=command.params;
  if(persona.onboarded===update_id)return operation({kind:'persona_onboarded',data:{current:personaView(),name:persona.name,fallback}});
  if(persona.initialized)return {v:1,id:'test',instance:'synthetic',seq:++sequence,type:'error',data:{code:'conflict',message:'a persona is already initialized'}};
  const name=fallback||!description.trim()?'Lumen':'Nova';
  persona.name=name;persona.initialized=true;persona.onboarded=update_id;
  persona.files=persona.files.map(f=>f.name==='IDENTITY.MD'?{...f,text:`我是 ${name}。`}:f);
  persona.revision+=1;persona.updateId=update_id;persona.history=[...persona.history,{revision:persona.revision,update_id,text:personaText(),sources:['onboarding'],truncated:false}];
  return operation({kind:'persona_onboarded',data:{current:personaView(),name,fallback}});
 }
 case 'persona_save_file':{
  const {revision,expected_update_id,update_id,name,text}=command.params;
  if(revision!==persona.revision||expected_update_id!==persona.updateId)return {v:1,id:'test',instance:'synthetic',seq:++sequence,type:'error',data:{code:'conflict',message:'Synthetic persona conflict; reload before saving.'}};
  persona.files=persona.files.map(f=>f.name===name?{...f,text}:f);
  return operation(publish(update_id,['diva']));
 }
 case 'persona_rollback':{
  if(command.params.revision!==persona.revision)return {v:1,id:'test',instance:'synthetic',seq:++sequence,type:'error',data:{code:'conflict',message:'Synthetic persona conflict; reload before saving.'}};
  return operation(publish(command.params.update_id,['human']));
 }
 case 'prompts_read':return operation({kind:'prompts',data:{prompts:['RECAP.md','AUTODREAM.md'].map(name=>({name,source:'default',revision:'fixture',text:'Synthetic prompt'}))}});
 case 'recap_read':return frame({revision:0,summary:'Synthetic committed recap',covered:{},jobs:[],refresh_pending:false,error:null});
 case 'tasks_read':return frame({revision:0,dream_revision:null,wakes:[],dreams:[],jobs:[]});
 case 'python_read':return frame({outcome:'not_init',phase:null,detail:'Synthetic runtime report',scope:'test'});
 default:throw new Error(`Synthetic test does not implement ${command.method}`);
}}
const status=async()=>({connected:true,error:null,data_directory:'synthetic-only',outbox:[]});
export const mockTransport:DesktopTransport={connect:status,status,request,submit:async()=>{throw new Error('Synthetic send failure; draft must remain');},listen:async fn=>{handler=fn;return ()=>{};},disconnected:async()=>()=>{}};
