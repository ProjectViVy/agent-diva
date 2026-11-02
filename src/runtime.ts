import { reactive } from 'vue';
import type { Frame, Method, Snapshot, HistoryMessage, Live, ProvidersView, ManagementCompleted, ManagementResult } from './generated/protocol';

export type PendingSend = { request_id: string; text: string };
export type HostStatus = { connected: boolean; error: string | null; data_directory: string; outbox: PendingSend[] };
export interface DesktopTransport {
  connect(): Promise<HostStatus>;
  status(): Promise<HostStatus>;
  request(command: Method): Promise<Frame>;
  submit(record: PendingSend): Promise<Frame>;
  listen(handler: (frame: Frame) => void): Promise<() => void>;
  disconnected(handler: (status: HostStatus) => void): Promise<() => void>;
}

export function createRuntime(transport: DesktopTransport) {
  const state = reactive({ connected: false, connecting: false, syncing: false, error: '', directory: '', instance: '',
    snapshot: null as Snapshot | null, history: [] as HistoryMessage[], pending: [] as PendingSend[],
    live: null as Live | null, interrupted: '', providers: null as ProvidersView | null });
  let sequence = 0;
  let buffered: Frame[] = [];
  let stop: (() => void)[] = [];
  let installed = false;
  let refresh: Promise<void> | undefined;
  let refreshEpoch=-1;
  let refreshAgain = false;
  let disposed = false;
  let epoch = 0;
  const completions=new Map<string,ManagementCompleted>();
  const waiting=new Map<string,{resolve:(result:ManagementCompleted)=>void;reject:(error:Error)=>void}>();
  const commits=new Set<string>();
  const commitWaiters=new Map<string,{resolve:()=>void;reject:(error:Error)=>void}>();
  function disconnect(error: string) {
    epoch++;
    if (state.live?.text) state.interrupted = state.live.text;
    state.live = null; state.connected = false; state.syncing=false; state.error = error;
    for(const waiter of waiting.values())waiter.reject(new Error(error));
    waiting.clear();completions.clear();
    for(const waiter of commitWaiters.values())waiter.reject(new Error(error));commitWaiters.clear();commits.clear();
  }
  function apply(frame: Frame) {
    if (frame.type === 'ready') { state.snapshot=frame.data; state.connected=true; }
    else if (frame.type === 'live') state.live=frame.data;
    else if (frame.type === 'questions' && state.snapshot) state.snapshot.questions=frame.data;
    else if (['changed','admission_committed','operation_committed'].includes(frame.type)) void synchronize();
  }
  function receive(frame: Frame) {
    if (disposed) return;
    if(frame.type==='operation_committed'&&frame.data.committed){
      const key=`${frame.instance}/${frame.data.operation}`;const waiter=commitWaiters.get(key);
      if(waiter){commitWaiters.delete(key);waiter.resolve();}else{commits.add(key);if(commits.size>32)commits.delete(commits.values().next().value!);}
    }
    // Completion receipts cannot be replaced by a newer conversation snapshot.
    if(frame.type==='management_completed') {
      const key=`${frame.instance}/${frame.data.operation}`;
      const waiter=waiting.get(key);
      if(waiter){waiting.delete(key);waiter.resolve(frame.data);}
      else {completions.set(key,frame.data);if(completions.size>32)completions.delete(completions.keys().next().value!);}
    }
    if (state.syncing) {
      if (buffered.length >= 256) { disconnect('事件缓存已满，请重新连接'); return; }
      buffered.push(frame); return;
    }
    if (frame.instance === state.instance && frame.seq <= sequence) return;
    if (frame.instance !== state.instance) {
      if (state.live?.text) state.interrupted=state.live.text;
      state.instance=frame.instance; sequence=0;
      if (frame.type !== 'ready') { void synchronize(); return; }
    }
    if (frame.seq !== sequence + 1) { void synchronize(); return; }
    sequence=frame.seq; apply(frame);
  }
  async function request(command: Method) {
    const frame=await transport.request(command);
    if (frame.type==='error') throw Object.assign(new Error(frame.data.message), { code: frame.data.code });
    if (frame.type!=='reply') throw new Error('核心没有返回关联响应');
    return frame.data;
  }
  async function operation(command:Method):Promise<ManagementResult> {
    const frame=await transport.request(command);
    if(frame.type==='error')throw new Error(frame.data.message);
    if(frame.type!=='reply'||!('operation' in frame.data))throw new Error('核心未接受管理操作');
    const key=`${frame.instance}/${frame.data.operation}`;
    let result=completions.get(key);completions.delete(key);
    if(!result)result=await new Promise<ManagementCompleted>((resolve,reject)=>{
      const timer=setTimeout(()=>{waiting.delete(key);reject(new Error('操作结果尚不确定，请重新读取状态后再操作'));},120000);
      waiting.set(key,{resolve:r=>{clearTimeout(timer);resolve(r);},reject:e=>{clearTimeout(timer);reject(e);}});
    });
    if(result.error)throw new Error(result.error.message);
    if(!result.result)throw new Error('管理操作缺少结果');
    return result.result;
  }
  async function synchronize() {
    // Only the saved receipt turns a control into a completed UI action.
    if (refresh&&refreshEpoch===epoch) { refreshAgain=true; return refresh; }
    const started=epoch;
    refreshEpoch=started;
    refresh=(async()=>{
      let retries=0;
      do {
        refreshAgain=false; state.syncing=true; buffered=[];
        try {
        const frame=await transport.request({method:'snapshot'});
        if(started!==epoch||disposed)return;
        if (frame.type!=='reply' || !('watermark' in frame.data)) throw new Error('无法取得核心快照');
        const snapshot=frame.data;
        // Pagination belongs to one committed revision. On cursor expiry retry the entire projection.
        const history:HistoryMessage[]=[];
        let cursor:string|null=null;
        do {
          const page=await request({method:'history',params:{cursor,limit:100}});
          if(started!==epoch||disposed)return;
          if (!('messages' in page)) throw new Error('无效历史响应');
          history.push(...page.messages); cursor=page.next_cursor;
        } while(cursor!==null);
        state.instance=frame.instance; sequence=snapshot.watermark; state.snapshot=snapshot; state.connected=true;
        state.history=history;state.error='';
        state.pending=state.pending.filter(p=>!snapshot.tasks.some(t=>t.request_id===p.request_id));
        state.syncing=false;
        const events=buffered; buffered=[];
        for (const event of events) receive(event);
        retries=0;
        } catch(error) {
          if(started!==epoch||disposed)return;
          if((error as {code?:string}).code==='stale_identity'&&retries++<3){refreshAgain=true;continue;}
          throw error;
        }
      } while(refreshAgain && started===epoch);
    })().catch(error=>{if(started===epoch)state.error=String(error);}).finally(()=>{if(started===epoch){state.syncing=false;refresh=undefined;}});
    return refresh;
  }
  async function connect() {
    state.connecting=true; state.error='';
    try {
      if (!installed) {
        stop=await Promise.all([transport.listen(receive),transport.disconnected(s=>disconnect(s.error??'核心已断开'))]);
        installed=true;
      }
      const status=await transport.connect(); state.directory=status.data_directory; state.pending=status.outbox;
      await synchronize();
    } catch(error) { disconnect(String(error)); }
    finally { state.connecting=false; }
  }
  async function send(text: string, request_id: string=crypto.randomUUID()) {
    const record={request_id,text}; if(!state.pending.some(p=>p.request_id===request_id))state.pending.push(record); state.error='';
    try {
      const frame=await transport.submit(record);
      if (frame.type==='error') throw new Error(frame.data.message);
      await synchronize();
    } catch(error) {
      const status=await transport.status().catch(()=>null);
      if (status) state.pending=status.outbox;
      state.error=String(error); throw error;
    }
  }
  async function providers(reload=false) {
    const data=await request({method:reload?'providers_reload':'providers_read'});
    if (!('config' in data)) throw new Error('无效供应商响应');
    state.providers=data; return data;
  }
  async function committed(command:Method) {
    const frame=await transport.request(command);
    if(frame.type==='error')throw new Error(frame.data.message);
    if(frame.type!=='reply'||!('operation' in frame.data))throw new Error('核心未接受持久化操作');
    const key=`${frame.instance}/${frame.data.operation}`;
    if(!frame.data.committed&&!commits.delete(key))await new Promise<void>((resolve,reject)=>{
      const timer=setTimeout(()=>{commitWaiters.delete(key);reject(new Error('提交结果尚不确定，请重新读取状态'));},30000);
      commitWaiters.set(key,{resolve:()=>{clearTimeout(timer);resolve();},reject:e=>{clearTimeout(timer);reject(e);}});
    });
    await synchronize();
  }
  return { state,connect,disconnect,send,request,operation,committed,providers,synchronize,
    stop:async()=>{const s=state.snapshot;const task=s?.tasks.find(t=>t.id===s.active_task);if(task)await committed({method:'cancel',params:{task:task.id,generation:task.generation}});},
    dispose:()=>{disposed=true;disconnect('运行时已关闭');stop.forEach(fn=>fn());} };
}
