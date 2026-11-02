// @vitest-environment happy-dom
import {it,expect,vi} from 'vitest';
const reset=vi.hoisted(()=>vi.fn(async()=>{}));
vi.mock('./onboarding',()=>({resetOnboarding:reset}));
it('toggles and persists debug output',async()=>{
 const {debugOutput}=await import('./debug');
 debugOutput.value=false;
 localStorage.removeItem('agent-diva.debug-output');
 const {default:GeneralPanel}=await import('./GeneralPanel.vue');
 const {mount,flushPromises}=await import('@vue/test-utils');
 const w=mount(GeneralPanel as any);
 const toggle=()=>w.get('button[role="switch"]');
 expect(toggle().attributes('aria-checked')).toBe('false');
 await toggle().trigger('click');await flushPromises();
 expect(debugOutput.value).toBe(true);
 expect(toggle().attributes('aria-checked')).toBe('true');
 expect(localStorage.getItem('agent-diva.debug-output')).toBe('1');
 await toggle().trigger('click');await flushPromises();
 expect(debugOutput.value).toBe(false);
 expect(localStorage.getItem('agent-diva.debug-output')).toBe('0');
 w.unmount();
});
it('resets onboarding from the general panel',async()=>{
 const {default:GeneralPanel}=await import('./GeneralPanel.vue');
 const {mount,flushPromises}=await import('@vue/test-utils');
 reset.mockClear();
 const w=mount(GeneralPanel as any);
 const button=w.findAll('button').find(b=>b.text()==='重新运行');
 await button!.trigger('click');await flushPromises();
 expect(reset).toHaveBeenCalledTimes(1);
 w.unmount();
});
