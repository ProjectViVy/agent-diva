import { invoke, isTauri } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import { createRuntime, type DesktopTransport } from './runtime';
import type { Frame } from './generated/protocol';
import type { HostStatus } from './runtime';

const native: DesktopTransport = {
  connect:()=>isTauri()?invoke('core_connect'):Promise.reject(new Error('请使用桌面应用连接核心')),
  status:()=>invoke('core_status'),
  request:request=>invoke('core_request',{request}),
  submit:record=>invoke('core_submit',{record}),
  listen:handler=>isTauri()?listen<Frame>('core-frame',event=>handler(event.payload)):Promise.resolve(()=>{}),
  disconnected:handler=>isTauri()?listen<HostStatus>('core-disconnected',event=>handler(event.payload)):Promise.resolve(()=>{}),
};
// The synthetic transport is only included in an explicit test-mode build.
const transport=import.meta.env.MODE==='test'?(await import('./testing/mockTransport')).mockTransport:native;
export const runtime=createRuntime(transport);
export const diagnostics=()=>isTauri()?invoke<string>('core_diagnostics'):Promise.resolve('Synthetic browser mode has no host logs.');
