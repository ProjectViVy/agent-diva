import type {DesktopTransport} from '../runtime';
import type {Frame,Method,Snapshot,ProvidersView,ManagementResult,Reply,HistoryMessage} from '../generated/protocol';
document.title='Diva · synthetic IPC test';
let handler:(f:Frame)=>void=()=>{},sequence=0;
const readiness={compiled:true,configured:true,ready:true,state:'ready',issue:null};
const snapshot:Snapshot={session:'synthetic-session',assistant:'assistant',core_version:'test-fixture',protocol_version:1,watermark:0,plugins:[],agents:[],shell:readiness,model:readiness,python:{...readiness,configured:false,ready:false,state:'unprepared'},memory:{...readiness,configured:false,ready:false,state:'off'},revision:0,active_task:null,tasks:[],questions:[],tools:[{id:'tool:1',task:'task:1',name:'read',phase:'completed',truncated:false,output:'Long tool output\n'.repeat(60)}]};
let providers:ProvidersView={presets:[],generation:1,issue:null,protocols:['openai','anthropic'],config:{default_model:'fixture/EXACT/model-id',instances:{fixture:{name:'Synthetic gateway',protocol:'openai',base_url:'http://127.0.0.1:9/v1',credential:{source:'env',name:'SYNTHETIC_ONLY'},models:[{id:'EXACT/model-id',context_window:32768,max_output_tokens:2048,tools:true}]}}}};
const history:HistoryMessage[]=[{id:'user:1',task:'task:1',role:'user',text:'这是合成 IPC 浏览器测试。',reasoning:'',status:'completed',truncated:false},{id:'assistant:1',task:'task:1',role:'assistant',text:'Markdown 与长内容测试。\n\n```typescript\n'+('const example = "'+'x'.repeat(180)+'";\n').repeat(8)+'```\n\n<div>Raw HTML is text.</div>',reasoning:'检查合成 IPC 的流式投影。',status:'completed',truncated:false}];
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
 case 'persona_read':return operation({kind:'persona',data:{current:{revision:0,update_id:'',text:'Synthetic persona'},history:[]}});
 case 'prompts_read':return operation({kind:'prompts',data:{prompts:['ASSISTANT.md','RECAP.md','AUTODREAM.md'].map(name=>({name,source:'default',revision:'fixture',text:'Synthetic prompt'}))}});
 case 'recap_read':return frame({revision:0,summary:'Synthetic committed recap',covered:{},jobs:[],refresh_pending:false,error:null});
 case 'tasks_read':return frame({revision:0,dream_revision:null,wakes:[],dreams:[],jobs:[]});
 case 'python_read':return frame({outcome:'not_init',phase:null,detail:'Synthetic runtime report',scope:'test'});
 default:throw new Error(`Synthetic test does not implement ${command.method}`);
}}
const status=async()=>({connected:true,error:null,data_directory:'synthetic-only',outbox:[]});
export const mockTransport:DesktopTransport={connect:status,status,request,submit:async()=>{throw new Error('Synthetic send failure; draft must remain');},listen:async fn=>{handler=fn;return ()=>{};},disconnected:async()=>()=>{}};
