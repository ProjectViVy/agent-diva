// @vitest-environment happy-dom
import {it,expect} from 'vitest';
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
