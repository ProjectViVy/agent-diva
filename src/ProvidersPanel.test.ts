// @vitest-environment happy-dom
import {it,expect,vi,beforeEach} from 'vitest';
import {reactive} from 'vue';
import {mount,flushPromises} from '@vue/test-utils';
import Panel from './ProvidersPanel.vue';
const holder=vi.hoisted(()=>({runtime:null as any}));
vi.mock('./ipc',()=>({get runtime(){return holder.runtime;}}));
const instance=(url:string)=>({protocol:'openai',base_url:url,credential:{source:'stored'},models:[{id:'EXACT/Model',context_window:32768,max_output_tokens:8192}]});
beforeEach(()=>{holder.runtime={state:reactive({providers:{generation:1,issue:null,protocols:['openai'],config:{instances:{}},presets:[{id:'deepseek',name:'DeepSeek',variants:[{id:'standard',name:'Standard',default_model:'deepseek-flash',instance:{...instance('https://example.test'),models:[{id:'deepseek-flash'}]}}]},{id:'bailian',name:'阿里云百炼',variants:['china','international','coding'].map(id=>({id,name:id,default_model:'EXACT/Model',instance:instance('https://'+id+'.test')}))}]}}),providers:vi.fn(),synchronize:vi.fn(),request:vi.fn(async(c:any)=>{if(c.method==='provider_key')throw new Error('key write failed');const r={...holder.runtime.state.providers,generation:holder.runtime.state.providers.generation+1,config:c.params.config};return r;})};});
const button=(w:any,text:string)=>w.findAll('button').find((b:any)=>b.text()===text);
it('shows presets without saving and keeps default unchanged when credential write fails',async()=>{
 const w=mount(Panel);await flushPromises();expect(holder.runtime.request).not.toHaveBeenCalled();
 await button(w,'配置 DeepSeek').trigger('click');await w.find('input[type=password]').setValue('fixture-key');
 await w.find('form').trigger('submit');await flushPromises();
 expect(w.find('[role=alert]').text()).toContain('key write failed');expect((w.find('input[type=password]').element as HTMLInputElement).value).toBe('fixture-key');
 expect(holder.runtime.state.providers.config.default_model).toBeUndefined();expect(holder.runtime.request.mock.calls).toHaveLength(2);w.unmount();
});
it('changes endpoint through the dropdown and clears the unsaved key',async()=>{
 const w=mount(Panel);await flushPromises();await w.findAll('.resource-list button')[1].trigger('click');await button(w,'配置 阿里云百炼').trigger('click');
 await w.find('input[type=password]').setValue('fixture-key');await w.find('select').setValue('international');
 expect((w.find('input[type=url]').element as HTMLInputElement).value).toBe('https://international.test');expect((w.find('input[type=password]').element as HTMLInputElement).value).toBe('');
 await w.find('form').trigger('submit');await flushPromises();expect(holder.runtime.request).not.toHaveBeenCalled();w.unmount();
});
it('commits credentials before selecting the exact default model',async()=>{
 holder.runtime.request.mockImplementation(async(c:any)=>({...holder.runtime.state.providers,generation:holder.runtime.state.providers.generation+1,config:c.method==='providers_save'?c.params.config:holder.runtime.state.providers.config}));
 const w=mount(Panel);await flushPromises();await button(w,'配置 DeepSeek').trigger('click');await w.find('input[type=password]').setValue('fixture-key');await w.find('form').trigger('submit');await flushPromises();
 expect(holder.runtime.request.mock.calls.map(([c]:any)=>c.method)).toEqual(['providers_save','provider_key','providers_save']);expect(holder.runtime.state.providers.config.default_model).toBe('deepseek/deepseek-flash');w.unmount();
});
