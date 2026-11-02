import { describe, it, expect } from 'vitest';
import { createRuntime, type DesktopTransport } from './runtime';
import type { Frame, Snapshot } from './generated/protocol';

const snapshot = (watermark = 1): Snapshot => ({ session:'test', assistant:'main',core_version:'test',protocol_version:1,watermark,
  plugins:[],agents:[],shell:{compiled:true,configured:false,ready:false,state:'unavailable',issue:null},model:{compiled:true,configured:true,ready:true,state:'ready',issue:null},
  python:{compiled:false,configured:false,ready:false,state:'unprepared',issue:null},
  memory:{compiled:false,configured:false,ready:false,state:'off',issue:null},revision:0,active_task:null,tasks:[],questions:[],tools:[] });
function fixture() {
  let handler: (f:Frame)=>void = ()=>{};
  let sequence = 0;
  const transport: DesktopTransport = {
    listen: async (fn) => { handler=fn; return ()=>{}; },
    status:async()=>({connected:true,error:null,data_directory:'test',outbox:[]}),
    connect:async()=>({connected:true,error:null,data_directory:'test',outbox:[]}),
    request:async(command)=> {
      const seq=++sequence;
      const frame:Frame={v:1,id:'query',instance:'one',seq,type:'reply',data:command.method==='snapshot'?snapshot(seq):{messages:[],next_cursor:null}};
      handler(frame); return frame;
    },
    submit:async()=>{throw new Error('outbox write failed');},
    disconnected:async()=>()=>{},
  };
  return { transport, emit:(f:Frame)=>{sequence=Math.max(sequence,f.seq);handler(f);} };
}
describe('desktop runtime',()=>{
  it('starts a fresh synchronization when reconnecting before an old request settles',async()=>{
    const f=fixture();const runtime=createRuntime(f.transport);await runtime.connect();const request=f.transport.request;
    let finish!:(frame:Frame)=>void;
    f.transport.request=()=>new Promise(resolve=>{finish=resolve;});
    const old=runtime.synchronize();runtime.disconnect('old pipe closed');
    f.transport.request=request;await runtime.connect();
    expect(runtime.state.connected).toBe(true);expect(runtime.state.syncing).toBe(false);
    finish({v:1,id:'old',instance:'old',seq:1,type:'reply',data:snapshot(1)});await old;
    expect(runtime.state.instance).toBe('one');expect(runtime.state.connected).toBe(true);
  });
  it('retries an expired history cursor without publishing a partial history',async()=>{
    const f=fixture();const original=f.transport.request;let expired=false;
    f.transport.request=async command=>{
      if(command.method==='history'&&!expired){expired=true;return {v:1,id:'query',instance:'one',seq:2,type:'error',data:{code:'stale_identity',message:'expired cursor'}};}
      return original(command);
    };
    const runtime=createRuntime(f.transport);await runtime.connect();
    expect(expired).toBe(true);expect(runtime.state.connected).toBe(true);
    expect(runtime.state.error).toBe('');expect(runtime.state.history).toEqual([]);
  });
  it('does not resurrect a connection when an in-flight snapshot resolves after disconnect',async()=>{
    const f=fixture();const runtime=createRuntime(f.transport);await runtime.connect();
    let finish!:(frame:Frame)=>void;
    f.transport.request=()=>new Promise(resolve=>{finish=resolve;});
    const sync=runtime.synchronize();runtime.disconnect('pipe closed');
    finish({v:1,id:'query',instance:'one',seq:3,type:'reply',data:snapshot(3)});
    await sync;expect(runtime.state.connected).toBe(false);expect(runtime.state.error).toBe('pipe closed');
  });
  it('retains an early durable receipt until its control acceptance arrives',async()=>{
    const f=fixture();const runtime=createRuntime(f.transport);await runtime.connect();const original=f.transport.request;
    f.transport.request=async command=>{
      if(command.method!=='cancel')return original(command);
      f.emit({v:1,instance:'one',seq:3,id:null,type:'operation_committed',data:{operation:'cancel-op',committed:true}});
      return {v:1,instance:'one',seq:4,id:'query',type:'reply',data:{operation:'cancel-op',committed:false}};
    };
    await runtime.committed({method:'cancel',params:{task:'task',generation:0}});
    expect(runtime.state.error).toBe('');
  });
  it('retains a management result that arrives before its acceptance response',async()=>{
    const f=fixture();const runtime=createRuntime(f.transport);await runtime.connect();
    f.transport.request=async()=>{
      f.emit({v:1,instance:'one',seq:3,id:null,type:'management_completed',data:{operation:'op',result:{kind:'prompts',data:{prompts:[]}},error:null}});
      return {v:1,instance:'one',seq:4,id:'query',type:'reply',data:{operation:'op',committed:false}};
    };
    expect(await runtime.operation({method:'prompts_read'})).toEqual({kind:'prompts',data:{prompts:[]}});
  });
  it('retains the draft and removes an unpersisted bubble when outbox writing fails',async()=>{
    const {transport}=fixture();const runtime=createRuntime(transport);await runtime.connect();
    await expect(runtime.send('hello','stable')).rejects.toThrow('outbox write failed');
    expect(runtime.state.pending).toEqual([]);
    expect(runtime.state.error).toContain('outbox write failed');
  });
  it('never converts provisional streaming text into a completed message on disconnect',async()=>{
    const f=fixture();const runtime=createRuntime(f.transport);await runtime.connect();
    f.emit({v:1,instance:'one',seq:3,id:null,type:'live',data:{task:'task:1',text:'unfinished',reasoning:''}});
    runtime.disconnect('core died');
    expect(runtime.state.history).toEqual([]);
    expect(runtime.state.interrupted).toBe('unfinished');
  });
});
