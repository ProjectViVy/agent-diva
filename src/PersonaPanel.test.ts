// @vitest-environment happy-dom
import {beforeEach,describe,expect,it,vi} from 'vitest';
import {mount,flushPromises} from '@vue/test-utils';
import PersonaPanel from './PersonaPanel.vue';
import type {ManagementResult,Method,PersonaFileView,PersonaVersionView} from './generated/protocol';
const operation=vi.hoisted(()=>vi.fn());
vi.mock('./ipc',()=>({runtime:{operation}}));
const baseFiles:PersonaFileView[]=[
 {name:'MISSION.MD',text:'Existing mission',required:true,auto:false},
 {name:'IDENTITY.MD',text:'Existing identity',required:true,auto:true},
 {name:'RELATIONSHIP.MD',text:'Existing relationship',required:true,auto:true},
 {name:'REDLINE.MD',text:'Existing redline',required:true,auto:false},
 {name:'DREAM.MD',text:'Existing dream',required:false,auto:true},
 {name:'DARK.MD',text:'',required:false,auto:true},
];
const history:PersonaVersionView[]=[{revision:1,update_id:'u1',text:'Older persona',sources:['human'],truncated:false}];
function persona(overrides:Partial<{revision:number;update_id:string;files:PersonaFileView[];unpublished:boolean}>={}):ManagementResult{
 const files=overrides.files??baseFiles.map(f=>({...f}));
 return {kind:'persona',data:{current:{revision:overrides.revision??3,update_id:overrides.update_id??'u3',directory:'C:/workspace/.vivy/laputa/personas/main',text:files.map(f=>f.text.trim()).filter(Boolean).join('\n\n'),files,unpublished:overrides.unpublished??false},history}};
}
function button(wrapper:ReturnType<typeof mount>,text:string){return wrapper.findAll('button').find(b=>b.text()===text);}
function fileButton(wrapper:ReturnType<typeof mount>,name:string){return wrapper.findAll('.persona-files button').find(b=>b.find('strong').text()===name)!;}
describe('Laputa core persona panel',()=>{
 beforeEach(()=>{
  operation.mockReset();
  operation.mockImplementation(async (command:Method)=>{
   if(command.method==='persona_read')return persona();
   if(command.method==='persona_save_file'){
    const files=baseFiles.map(f=>f.name===command.params.name?{...f,text:command.params.text}:{...f});
    return persona({revision:4,update_id:command.params.update_id,files});
   }
   if(command.method==='persona_rollback')return persona({revision:4,update_id:command.params.update_id});
   throw new Error(`unexpected ${command.method}`);
  });
 });
 it('lists the six files with roles and never requests prompts',async()=>{
  const wrapper=mount(PersonaPanel);await flushPromises();
  expect(operation).toHaveBeenCalledWith({method:'persona_read'});
  expect(operation.mock.calls.some(([command])=>command.method==='prompts_read'||command.method==='prompt_save'||command.method==='persona_update')).toBe(false);
  expect(wrapper.findAll('.persona-files button').map(b=>b.find('strong').text())).toEqual(baseFiles.map(f=>f.name));
  const text=wrapper.text();
  expect(text).toContain('使命宣言');
  expect(text).toContain('AutoDream 可改写');
  expect(text).toContain('仅人工');
  expect(text).toContain('目录：C:/workspace/.vivy/laputa/personas/main');
  wrapper.unmount();
 });
 it('saves a single file with revision, update id and fresh update id',async()=>{
  const wrapper=mount(PersonaPanel);await flushPromises();
  await fileButton(wrapper,'IDENTITY.MD').trigger('click');
  await button(wrapper,'编辑')!.trigger('click');
  await wrapper.find('textarea').setValue('Updated identity');
  await button(wrapper,'保存')!.trigger('click');
  await flushPromises();
  const save=operation.mock.calls.find(([command])=>command.method==='persona_save_file')![0];
  expect(save.params).toMatchObject({revision:3,expected_update_id:'u3',name:'IDENTITY.MD',text:'Updated identity'});
  expect(save.params.update_id).toMatch(/^[0-9a-f-]{36}$/i);
  expect(wrapper.find('textarea').exists()).toBe(false);
  wrapper.unmount();
 });
 it('asks before saving a protected file and cancelling keeps the draft',async()=>{
  const wrapper=mount(PersonaPanel);await flushPromises();
  await fileButton(wrapper,'MISSION.MD').trigger('click');
  await button(wrapper,'编辑')!.trigger('click');
  await wrapper.find('textarea').setValue('New mission');
  await button(wrapper,'保存')!.trigger('click');
  await flushPromises();
  expect(operation.mock.calls.some(([command])=>command.method==='persona_save_file')).toBe(false);
  expect(wrapper.find('dialog').attributes('aria-label')).toBe('确认修改受保护人格文件');
  await button(wrapper,'取消')!.trigger('click');
  await flushPromises();
  expect(operation.mock.calls.some(([command])=>command.method==='persona_save_file')).toBe(false);
  expect((wrapper.find('textarea').element as HTMLTextAreaElement).value).toBe('New mission');
  await button(wrapper,'保存')!.trigger('click');
  await flushPromises();
  await button(wrapper,'确认保存')!.trigger('click');
  await flushPromises();
  const save=operation.mock.calls.find(([command])=>command.method==='persona_save_file')![0];
  expect(save.params).toMatchObject({name:'MISSION.MD',text:'New mission'});
  wrapper.unmount();
 });
 it('resolves the leave-guard save only after protected confirmation',async()=>{
  const wrapper=mount(PersonaPanel);await flushPromises();
  await fileButton(wrapper,'REDLINE.MD').trigger('click');
  await button(wrapper,'编辑')!.trigger('click');
  await wrapper.find('textarea').setValue('New redline');
  const pending=(wrapper.vm as unknown as {save:()=>Promise<boolean>}).save();
  await flushPromises();
  expect(operation.mock.calls.some(([command])=>command.method==='persona_save_file')).toBe(false);
  await button(wrapper,'确认保存')!.trigger('click');
  expect(await pending).toBe(true);
  const save=operation.mock.calls.find(([command])=>command.method==='persona_save_file')![0];
  expect(save.params).toMatchObject({name:'REDLINE.MD',text:'New redline'});
  wrapper.unmount();
 });
 it('keeps the draft when a save conflicts',async()=>{
  const wrapper=mount(PersonaPanel);await flushPromises();
  await fileButton(wrapper,'IDENTITY.MD').trigger('click');
  await button(wrapper,'编辑')!.trigger('click');
  await wrapper.find('textarea').setValue('Draft after conflict');
  operation.mockRejectedValueOnce(new Error('persona changed; reload before saving'));
  await button(wrapper,'保存')!.trigger('click');
  await flushPromises();
  expect(wrapper.find('[role="alert"]').text()).toContain('reload before saving');
  expect((wrapper.find('textarea').element as HTMLTextAreaElement).value).toBe('Draft after conflict');
  wrapper.unmount();
 });
 it('blocks a blank MISSION before writing',async()=>{
  const wrapper=mount(PersonaPanel);await flushPromises();
  await fileButton(wrapper,'MISSION.MD').trigger('click');
  await button(wrapper,'编辑')!.trigger('click');
  await wrapper.find('textarea').setValue('   ');
  expect(button(wrapper,'保存')!.attributes('disabled')).toBeDefined();
  await button(wrapper,'保存')!.trigger('click');
  await flushPromises();
  expect(operation.mock.calls.some(([command])=>command.method==='persona_save_file')).toBe(false);
  wrapper.unmount();
 });
 it('rolls back the whole revision through persona_rollback',async()=>{
  const wrapper=mount(PersonaPanel);await flushPromises();
  await button(wrapper,'恢复此版本')!.trigger('click');
  await flushPromises();
  const rollback=operation.mock.calls.find(([command])=>command.method==='persona_rollback')![0];
  expect(rollback.params).toMatchObject({revision:3,target:1});
  expect(rollback.params.update_id).toMatch(/^[0-9a-f-]{36}$/i);
  wrapper.unmount();
 });
});
