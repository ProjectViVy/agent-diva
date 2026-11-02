// @vitest-environment happy-dom
import {describe,it,expect,vi} from 'vitest';
import {mount,flushPromises} from '@vue/test-utils';
import McpPanel from './McpPanel.vue';
const operation=vi.hoisted(()=>vi.fn());
vi.mock('./ipc',()=>({runtime:{operation}}));
describe('MCP configuration editor',()=>{
  it('preserves an explicit deny-all filter when changing an unrelated timeout',async()=>{
    const view={revision:'rev',connections:{locked:{type:'stdio',command:'python',args:[],enabled:true,env:{},headers:{},enabledTools:[],disabledTools:[],startupTimeoutMs:20000,callTimeoutMs:60000}}};
    operation.mockResolvedValue({kind:'mcp',data:view});
    const wrapper=mount(McpPanel);await flushPromises();
    const button=(text:string)=>wrapper.findAll('button').find(b=>b.text()===text)!;
    await button('lockedstdio · 启用').trigger('click');
    await button('编辑').trigger('click');
    await wrapper.find('input[max="600000"]').setValue('90000');
    await wrapper.find('form').trigger('submit');await flushPromises();
    const save=operation.mock.calls.find(([command])=>command.method==='mcp_save')![0];
    expect(save.params.connections.locked.callTimeoutMs).toBe(90000);
    expect(save.params.connections.locked.enabledTools).toEqual([]);
    wrapper.unmount();
  });
});
