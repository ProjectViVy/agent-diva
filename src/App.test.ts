// @vitest-environment happy-dom
import {it,expect,vi} from 'vitest';
const holder=vi.hoisted(()=>({runtime:null as any}));
vi.mock('./ipc',()=>({get runtime(){return holder.runtime;}}));
const failedError='The model call failed: ProviderResponseError: status 503 Service Unavailable: {"error":{"code":"model_not_found","message":"No available channel for model deepseek-v4.1-flash under group default","type":"new_api_error"}}';
const defaultTools=[{id:'tool:1',task:'task:1',name:'dream_status',phase:'settled',step:1,output:'{"foreground":"idle"}',truncated:false}];
const defaultHistory=[{id:'m1',task:'task:1',role:'assistant',text:'你好',reasoning:'先检查渠道。\n再回复用户。',status:'failed',step:2,truncated:false}];
async function mountApp(debug:boolean,fixture:{history?:any[];tools?:any[]}={}){
 const {reactive}=await import('vue');
 const {debugOutput}=await import('./debug');
 debugOutput.value=debug;
 holder.runtime={state:reactive({connected:true,connecting:false,syncing:false,error:'',directory:'',instance:'core',
  snapshot:{model:{ready:true},questions:[],active_task:null,tools:fixture.tools??defaultTools,
   tasks:[{id:'task:1',request_id:'request-1',status:'failed',generation:0,error:failedError}]},
  history:fixture.history??defaultHistory,pending:[],live:null,interrupted:''}),
  connect:vi.fn(async()=>{}),dispose:vi.fn(),send:vi.fn(),stop:vi.fn()};
 const {default:App}=await import('./App.vue');
 const {mount,flushPromises}=await import('@vue/test-utils');
 const w=mount(App as any);await flushPromises();
 return w;
}
it('shows the provider conclusion under a failed message',async()=>{
 const w=await mountApp(false);
 expect(w.find('.turn-failure').text()).toBe('供应商错误：HTTP 503 · No available channel for model deepseek-v4.1-flash under group default');
 w.unmount();
});
it('keeps tool records and reasoning out of the conversation by default',async()=>{
 const w=await mountApp(false);
 expect(w.find('.tool').exists()).toBe(false);
 expect(w.find('.reasoning').exists()).toBe(false);
 w.unmount();
});
it('shows tool records and reasoning when debug output is enabled',async()=>{
 const w=await mountApp(true);
 expect(w.find('.tool-name').text()).toBe('dream_status');
 expect(w.find('.tool-line').text()).toBe('{"foreground":"idle"}');
 const reasoning=w.find('.reasoning');
 expect(reasoning.exists()).toBe(true);
 expect(reasoning.find('.reasoning-preview').text()).toBe('先检查渠道。');
 expect(reasoning.find('pre').text()).toContain('再回复用户。');
 w.unmount();
});
it('shows the reasoning of a tool-calling turn before its tool record',async()=>{
 const w=await mountApp(true,{
  tools:[{id:'tool:mid',task:'task:1',name:'bash',phase:'settled',step:2,output:'ok',truncated:false}],
  history:[
   {id:'u1',task:'task:1',role:'user',text:'开始',reasoning:'',status:'running',step:0,truncated:false},
   {id:'a1',task:'task:1',role:'assistant',text:'',reasoning:'先看目录。',status:'running',step:1,truncated:false},
   {id:'a2',task:'task:1',role:'assistant',text:'已处理完成',reasoning:'目录没问题。',status:'completed',step:3,truncated:false}
  ]});
 const html=w.find('.messages').html();
 expect(html.indexOf('先看目录')).toBeLessThan(html.indexOf('bash'));
 expect(html.indexOf('bash')).toBeLessThan(html.indexOf('目录没问题'));
 expect(html.indexOf('目录没问题')).toBeLessThan(html.indexOf('已处理完成'));
 expect(w.findAll('.reasoning')).toHaveLength(2);
 expect(w.findAll('.message-row.assistant')).toHaveLength(1);
 w.unmount();
});
