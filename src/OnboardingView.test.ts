// @vitest-environment happy-dom
import {beforeEach,describe,expect,it,vi} from 'vitest';
import {mount,flushPromises} from '@vue/test-utils';
import {reactive} from 'vue';
import OnboardingView from './OnboardingView.vue';
import type {Method,PersonaView} from './generated/protocol';
const holder=vi.hoisted(()=>({runtime:null as any,saveResult:true}));
vi.mock('./ipc',()=>({get runtime(){return holder.runtime;}}));
vi.mock('./onboarding',()=>({openPersonaDirectory:vi.fn(async()=>'C:/Users/test/.laputa/personas/main')}));
vi.mock('./ProvidersPanel.vue',async()=>{
 const {defineComponent,h}=await import('vue');
 return {default:defineComponent({name:'ProvidersPanelStub',setup(_,{expose}){expose({save:async()=>holder.saveResult});return()=>h('div',{class:'providers-stub'});}})};
});
const view=(name='Nova',initialized=true):PersonaView=>({revision:1,update_id:'u1',directory:'C:/Users/test/.laputa/personas/main',name,initialized,text:'',files:[],unpublished:false});
function runtime(operation:ReturnType<typeof vi.fn>){
 holder.runtime={state:reactive({connected:true,connecting:false,error:''}),connect:vi.fn(async()=>{}),
  providers:vi.fn(async()=>({generation:3,config:{default_model:'fixture/exact-model',instances:{}}})),operation};
 return holder.runtime;
}
const passing={kind:'provider_test',data:{reference:'fixture/exact-model',checks:[{kind:'text',passed:true,detail:'passed'},{kind:'stream',passed:true,detail:'passed'},{kind:'tools',passed:true,detail:'passed'}]}};
const onboarded=(name='Nova',fallback=false)=>({kind:'persona_onboarded',data:{current:view(name),name,fallback}});
function button(wrapper:ReturnType<typeof mount>,text:string){return wrapper.findAll('button').find(b=>b.text()===text);}
async function reachPersona(operation:ReturnType<typeof vi.fn>){
 runtime(operation);
 const wrapper=mount(OnboardingView);await flushPromises();
 await button(wrapper,'保存并测试')!.trigger('click');await flushPromises();
 await button(wrapper,'下一步')!.trigger('click');
 await flushPromises();
 return wrapper;
}
describe('first-run onboarding',()=>{
 beforeEach(()=>{holder.saveResult=true;});
 it('keeps the next step locked until all three checks pass',async()=>{
  const operation=vi.fn(async(command:Method)=>{
   if(command.method==='provider_test')return {kind:'provider_test',data:{reference:'fixture/exact-model',checks:[{kind:'text',passed:true,detail:'passed'},{kind:'stream',passed:false,detail:'stream rejected'},{kind:'tools',passed:true,detail:'passed'}]}};
   throw new Error(`unexpected ${command.method}`);
  });
  runtime(operation);
  const wrapper=mount(OnboardingView);await flushPromises();
  await button(wrapper,'保存并测试')!.trigger('click');await flushPromises();
  expect(button(wrapper,'下一步')!.attributes('disabled')).toBeDefined();
  expect(wrapper.text()).toContain('全部通过');
  expect(wrapper.text()).toContain('stream rejected');
  wrapper.unmount();
 });
 it('saves the provider, tests it, then writes the description directly',async()=>{
  const operation=vi.fn(async(command:Method)=>{
   if(command.method==='provider_test')return passing;
   if(command.method==='persona_initialize')return onboarded();
   throw new Error(`unexpected ${command.method}`);
  });
  const wrapper=await reachPersona(operation);
  await wrapper.find('textarea').setValue('我是岚，我的使命是帮用户把复杂问题想清楚。');
  await button(wrapper,'开始认识你')!.trigger('click');await flushPromises();
  const init=operation.mock.calls.find(([command]:[Method])=>command.method==='persona_initialize')![0] as Extract<Method,{method:'persona_initialize'}>;
  expect(init.params.description).toBe('我是岚，我的使命是帮用户把复杂问题想清楚。');
  expect(init.params.fallback).toBe(false);
  expect(init.params.update_id).toMatch(/^[0-9a-f-]{36}$/i);
  expect(wrapper.text()).toContain('认识完成');
  expect(wrapper.text()).toContain('Nova');
  expect(wrapper.text()).toContain('C:/Users/test/.laputa/personas/main');
  await button(wrapper,'开始对话')!.trigger('click');
  expect(wrapper.emitted('done')).toHaveLength(1);
  wrapper.unmount();
 });
 it('keeps the description after a failed extraction',async()=>{
  const operation=vi.fn(async(command:Method)=>{
   if(command.method==='provider_test')return passing;
   if(command.method==='persona_initialize')throw new Error('core handshake timed out');
   throw new Error(`unexpected ${command.method}`);
  });
  const wrapper=await reachPersona(operation);
  await wrapper.find('textarea').setValue('保留这段描述');
  await button(wrapper,'开始认识你')!.trigger('click');await flushPromises();
  expect(wrapper.find('[role="alert"]').text()).toContain('core handshake timed out');
  expect((wrapper.find('textarea').element as HTMLTextAreaElement).value).toBe('保留这段描述');
  wrapper.unmount();
 });
 it('uses the built-in persona with a random name',async()=>{
  const operation=vi.fn(async(command:Method)=>{
   if(command.method==='provider_test')return passing;
   if(command.method==='persona_initialize')return onboarded('Lumen',true);
   throw new Error(`unexpected ${command.method}`);
  });
  const wrapper=await reachPersona(operation);
  await button(wrapper,'使用内置人格')!.trigger('click');await flushPromises();
  const init=operation.mock.calls.find(([command]:[Method])=>command.method==='persona_initialize')![0] as Extract<Method,{method:'persona_initialize'}>;
  expect(init.params.fallback).toBe(true);
  expect(wrapper.text()).toContain('Lumen');
  expect(wrapper.text()).toContain('内置人格');
  wrapper.unmount();
 });
 it('shows the existing persona when initialization is refused',async()=>{
  const operation=vi.fn(async(command:Method)=>{
   if(command.method==='provider_test')return passing;
   if(command.method==='persona_initialize')throw new Error('a persona is already initialized');
   if(command.method==='persona_read')return {kind:'persona',data:{current:view('Vivy'),history:[]}};
   throw new Error(`unexpected ${command.method}`);
  });
  const wrapper=await reachPersona(operation);
  await wrapper.find('textarea').setValue('新的描述');
  await button(wrapper,'开始认识你')!.trigger('click');await flushPromises();
  expect(wrapper.text()).toContain('这个人格已经存在');
  expect(wrapper.text()).toContain('Vivy');
  wrapper.unmount();
 });
 it('shows the existing persona when the built-in fallback is refused',async()=>{
  const operation=vi.fn(async(command:Method)=>{
   if(command.method==='provider_test')return passing;
   if(command.method==='persona_initialize')throw new Error('a persona is already initialized');
   if(command.method==='persona_read')return {kind:'persona',data:{current:view('Vivy'),history:[]}};
   throw new Error(`unexpected ${command.method}`);
  });
  const wrapper=await reachPersona(operation);
  await button(wrapper,'使用内置人格')!.trigger('click');await flushPromises();
  expect(wrapper.text()).toContain('这个人格已经存在');
  expect(wrapper.text()).toContain('Vivy');
  wrapper.unmount();
 });
});
