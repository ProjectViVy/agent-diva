// @vitest-environment happy-dom
import {beforeEach,describe,expect,it,vi} from 'vitest';
import {mount,flushPromises} from '@vue/test-utils';
import {reactive} from 'vue';
import OnboardingView from './OnboardingView.vue';
import type {Method,PersonaView,PythonReport} from './generated/protocol';
const holder=vi.hoisted(()=>({runtime:null as any,saveResult:true}));
vi.mock('./ipc',()=>({get runtime(){return holder.runtime;}}));
vi.mock('./onboarding',()=>({openPersonaDirectory:vi.fn(async()=>'C:/Users/test/.laputa/personas/main')}));
vi.mock('./ProvidersPanel.vue',async()=>{
 const {defineComponent,h}=await import('vue');
 return {default:defineComponent({name:'ProvidersPanelStub',setup(_,{expose}){expose({save:async()=>holder.saveResult});return()=>h('div',{class:'providers-stub'});}})};
});
const view=(name='Nova',initialized=true):PersonaView=>({revision:1,update_id:'u1',directory:'C:/Users/test/.laputa/personas/main',name,initialized,text:'',files:[],unpublished:false});
const pythonReport=(phase:string|null=null):PythonReport=>({outcome:'not_init',phase,detail:'Synthetic runtime report',scope:'test'});
function defaultRequest(){
 return vi.fn(async(command:Method)=>{
  if(command.method==='python_read')return pythonReport();
  if(command.method==='python_cancel')return pythonReport();
  throw new Error(`unexpected ${command.method}`);
 });
}
function runtime(operation:ReturnType<typeof vi.fn>,request:ReturnType<typeof vi.fn>=defaultRequest()){
 holder.runtime={state:reactive({connected:true,connecting:false,error:'',snapshot:null}),connect:vi.fn(async()=>{}),synchronize:vi.fn(async()=>{}),
  providers:vi.fn(async()=>({generation:3,config:{default_model:'fixture/exact-model',instances:{}}})),operation,request};
 return holder.runtime;
}
const passing={kind:'provider_test',data:{reference:'fixture/exact-model',checks:[{kind:'text',passed:true,detail:'passed'},{kind:'stream',passed:true,detail:'passed'},{kind:'tools',passed:true,detail:'passed'}]}};
const onboarded=(name='Nova',fallback=false)=>({kind:'persona_onboarded',data:{current:view(name),name,fallback}});
const prepared=(memory=true)=>({kind:'python',data:{outcome:'ready',phase:null,detail:memory?'Python and memory ready':'Python ready',scope:memory?'core+memory':'core'}});
function button(wrapper:ReturnType<typeof mount>,text:string){return wrapper.findAll('button').find(b=>b.text()===text);}
async function reachPersona(operation:ReturnType<typeof vi.fn>,request?:ReturnType<typeof vi.fn>){
 runtime(operation,request);
 const wrapper=mount(OnboardingView);await flushPromises();
 await button(wrapper,'保存并测试')!.trigger('click');await flushPromises();
 await button(wrapper,'下一步')!.trigger('click');
 await flushPromises();
 return wrapper;
}
async function skipWorkspace(wrapper:ReturnType<typeof mount>){
 await button(wrapper,'稍后初始化')!.trigger('click');
 await flushPromises();
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
  expect(wrapper.text()).toContain('准备工作区');
  await skipWorkspace(wrapper);
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
  expect(wrapper.text()).toContain('准备工作区');
  await skipWorkspace(wrapper);
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
  await skipWorkspace(wrapper);
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
  await skipWorkspace(wrapper);
  expect(wrapper.text()).toContain('这个人格已经存在');
  expect(wrapper.text()).toContain('Vivy');
  wrapper.unmount();
 });
 it('prepares Python and memory together by default',async()=>{
  const operation=vi.fn(async(command:Method)=>{
   if(command.method==='provider_test')return passing;
   if(command.method==='persona_initialize')return onboarded();
   if(command.method==='python_prepare')return prepared(true);
   throw new Error(`unexpected ${command.method}`);
  });
  const wrapper=await reachPersona(operation);
  await wrapper.find('textarea').setValue('我是岚。');
  await button(wrapper,'开始认识你')!.trigger('click');await flushPromises();
  await button(wrapper,'初始化工作区')!.trigger('click');await flushPromises();
  const prepare=operation.mock.calls.find(([command]:[Method])=>command.method==='python_prepare')![0] as Extract<Method,{method:'python_prepare'}>;
  expect(prepare.params).toEqual({managed:true,memory:true});
  expect(wrapper.text()).toContain('已就绪');
  await button(wrapper,'继续')!.trigger('click');await flushPromises();
  expect(wrapper.text()).toContain('认识完成');
  wrapper.unmount();
 });
 it('honors an unchecked memory option',async()=>{
  const operation=vi.fn(async(command:Method)=>{
   if(command.method==='provider_test')return passing;
   if(command.method==='persona_initialize')return onboarded();
   if(command.method==='python_prepare')return prepared(false);
   throw new Error(`unexpected ${command.method}`);
  });
  const wrapper=await reachPersona(operation);
  await wrapper.find('textarea').setValue('我是岚。');
  await button(wrapper,'开始认识你')!.trigger('click');await flushPromises();
  await wrapper.find('input[type="checkbox"]').setValue(false);
  await button(wrapper,'初始化工作区')!.trigger('click');await flushPromises();
  const prepare=operation.mock.calls.find(([command]:[Method])=>command.method==='python_prepare')![0] as Extract<Method,{method:'python_prepare'}>;
  expect(prepare.params).toEqual({managed:true,memory:false});
  wrapper.unmount();
 });
 it('skips workspace initialization without preparing',async()=>{
  const operation=vi.fn(async(command:Method)=>{
   if(command.method==='provider_test')return passing;
   if(command.method==='persona_initialize')return onboarded();
   throw new Error(`unexpected ${command.method}`);
  });
  const wrapper=await reachPersona(operation);
  await wrapper.find('textarea').setValue('我是岚。');
  await button(wrapper,'开始认识你')!.trigger('click');await flushPromises();
  await skipWorkspace(wrapper);
  expect(operation.mock.calls.some(([command]:[Method])=>command.method==='python_prepare')).toBe(false);
  expect(wrapper.text()).toContain('稍后初始化');
  wrapper.unmount();
 });
 it('reports a preparation failure and offers a retry',async()=>{
  let attempts=0;
  const operation=vi.fn(async(command:Method)=>{
   if(command.method==='provider_test')return passing;
   if(command.method==='persona_initialize')return onboarded();
   if(command.method==='python_prepare'){
    attempts+=1;
    if(attempts===1)throw new Error('python download failed');
    return prepared(true);
   }
   throw new Error(`unexpected ${command.method}`);
  });
  const wrapper=await reachPersona(operation);
  await wrapper.find('textarea').setValue('我是岚。');
  await button(wrapper,'开始认识你')!.trigger('click');await flushPromises();
  await button(wrapper,'初始化工作区')!.trigger('click');await flushPromises();
  expect(wrapper.find('[role="alert"]').text()).toContain('python download failed');
  expect(button(wrapper,'重试初始化')).toBeDefined();
  await button(wrapper,'重试初始化')!.trigger('click');await flushPromises();
  expect(wrapper.text()).toContain('已就绪');
  wrapper.unmount();
 });
 it('cancels a running preparation',async()=>{
  const request=vi.fn(async(command:Method)=>{
   if(command.method==='python_read')return pythonReport('dependencies');
   if(command.method==='python_cancel')return pythonReport('dependencies');
   throw new Error(`unexpected ${command.method}`);
  });
  const operation=vi.fn(async(command:Method)=>{
   if(command.method==='provider_test')return passing;
   if(command.method==='persona_initialize')return onboarded();
   throw new Error(`unexpected ${command.method}`);
  });
  const wrapper=await reachPersona(operation,request);
  await wrapper.find('textarea').setValue('我是岚。');
  await button(wrapper,'开始认识你')!.trigger('click');await flushPromises();
  expect(wrapper.text()).toContain('初始化中（dependencies）');
  await button(wrapper,'取消初始化')!.trigger('click');await flushPromises();
  expect(request.mock.calls.some(([command]:[Method])=>command.method==='python_cancel')).toBe(true);
  wrapper.unmount();
 });
});
