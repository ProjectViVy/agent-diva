// @vitest-environment happy-dom
import {it,expect,vi} from 'vitest';
import {mount,flushPromises} from '@vue/test-utils';
import ManagedPanel from './ManagedPanel.vue';
const operation=vi.hoisted(()=>vi.fn());
vi.mock('./ipc',()=>({runtime:{operation}}));

it('does not open an obsolete candidate response against a new selection',async()=>{
  const candidates=['first','second'].map(id=>({id,name:id,base:'base',revision:id,dependencies:[],files:{},state:'candidate',result:null}));
  operation.mockResolvedValueOnce({kind:'managed',data:{current:{id:'base'},candidates,history:[],issue:null}});
  let finish!:(value:unknown)=>void;
  operation.mockImplementationOnce(()=>new Promise(resolve=>{finish=resolve;}));
  const wrapper=mount(ManagedPanel);await flushPromises();
  const button=(text:string)=>wrapper.findAll('button').find(b=>b.text()===text)!;
  await wrapper.findAll('.resource-list button')[0].trigger('click');
  await button('编辑候选').trigger('click');
  await wrapper.findAll('.resource-list button')[1].trigger('click');
  finish({kind:'managed_candidate',data:{...candidates[0],files:{'SKILL.md':'First candidate source'}}});
  await flushPromises();
  expect(wrapper.findAll('button').some(b=>b.text()==='保存为新候选')).toBe(false);
  expect(wrapper.find('.resource-list .selected').text()).toContain('second');
  wrapper.unmount();
});
