// @vitest-environment happy-dom
import {it,expect,vi} from 'vitest';
const holder=vi.hoisted(()=>({runtime:null as any}));
vi.mock('./ipc',()=>({get runtime(){return holder.runtime;}}));
it('shows the provider conclusion under a failed message',async()=>{
 const {reactive}=await import('vue');
 const error='The model call failed: ProviderResponseError: status 503 Service Unavailable: {"error":{"code":"model_not_found","message":"No available channel for model deepseek-v4.1-flash under group default","type":"new_api_error"}}';
 holder.runtime={state:reactive({connected:true,connecting:false,syncing:false,error:'',directory:'',instance:'core',
  snapshot:{model:{ready:true},questions:[],tools:[],active_task:null,tasks:[{id:'task:1',request_id:'request-1',status:'failed',generation:0,error}]},
  history:[{id:'m1',task:'task:1',role:'user',text:'你好',status:'failed',truncated:false}],pending:[],live:null,interrupted:''}),
  connect:vi.fn(async()=>{}),dispose:vi.fn(),send:vi.fn(),stop:vi.fn()};
 const {default:App}=await import('./App.vue');
 const {mount,flushPromises}=await import('@vue/test-utils');
 const w=mount(App as any);await flushPromises();
 expect(w.find('.turn-failure').text()).toBe('供应商错误：HTTP 503 · No available channel for model deepseek-v4.1-flash under group default');
 w.unmount();
});
