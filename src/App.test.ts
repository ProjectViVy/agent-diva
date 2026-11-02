// @vitest-environment happy-dom
import {it,expect,vi} from 'vitest';
const holder=vi.hoisted(()=>({runtime:null as any}));
vi.mock('./ipc',()=>({get runtime(){return holder.runtime;}}));
async function mountApp(debug:boolean){
 const {reactive}=await import('vue');
 const {debugOutput}=await import('./debug');
 debugOutput.value=debug;
 holder.runtime={state:reactive({connected:true,connecting:false,syncing:false,error:'',directory:'',instance:'core',
  snapshot:{model:{ready:true},questions:[],active_task:null,tools:[{id:'tool:1',task:'task:1',name:'dream_status',phase:'settled',output:'{"foreground":"idle"}',truncated:false}],
   tasks:[{id:'task:1',request_id:'request-1',status:'failed',generation:0,error:'The model call failed: ProviderResponseError: status 503 Service Unavailable: {"error":{"code":"model_not_found","message":"No available channel for model deepseek-v4.1-flash under group default","type":"new_api_error"}}'}]},
  history:[{id:'m1',task:'task:1',role:'assistant',text:'你好',reasoning:'先检查渠道。\n再回复用户。',status:'failed',truncated:false}],pending:[],live:null,interrupted:''}),
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
 expect(w.find('.debug-records').exists()).toBe(false);
 expect(w.find('.reasoning').exists()).toBe(false);
 w.unmount();
});
it('shows tool records and reasoning when debug output is enabled',async()=>{
 const w=await mountApp(true);
 expect(w.find('.debug-records').exists()).toBe(true);
 expect(w.find('.tool-name').text()).toBe('dream_status');
 expect(w.find('.tool-line').text()).toBe('{"foreground":"idle"}');
 const reasoning=w.find('.reasoning');
 expect(reasoning.exists()).toBe(true);
 expect(reasoning.find('.reasoning-preview').text()).toBe('先检查渠道。');
 expect(reasoning.find('pre').text()).toContain('再回复用户。');
 w.unmount();
});
