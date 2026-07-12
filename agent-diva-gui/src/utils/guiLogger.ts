import { appendGuiLog, type GuiLogEntry } from '../api/desktop';

const MAX_TEXT = 4096;
const MAX_QUEUE = 200;
const SENSITIVE_KEY = /api[_-]?key|token|password|authorization|secret/i;
let queue: GuiLogEntry[] = [];
let flushing = false;

function truncate(value: string): string {
  return value.length > MAX_TEXT ? `${value.slice(0, MAX_TEXT)}…[truncated]` : value;
}

function sanitize(value: unknown, seen = new WeakSet<object>()): unknown {
  if (typeof value === 'string') return truncate(value);
  if (value === null || typeof value !== 'object') return typeof value === 'bigint' ? value.toString() : value;
  if (seen.has(value)) return '[Circular]';
  seen.add(value);
  if (Array.isArray(value)) return value.slice(0, 100).map(item => sanitize(item, seen));
  return Object.fromEntries(Object.entries(value as Record<string, unknown>).slice(0, 100).map(([key, item]) => [key, SENSITIVE_KEY.test(key) ? '[REDACTED]' : sanitize(item, seen)]));
}

function text(value: unknown): string {
  if (typeof value === 'string') return truncate(value);
  try { return truncate(JSON.stringify(sanitize(value))); } catch { return '[Unserializable]'; }
}

function flush(): void {
  if (flushing || !queue.length) return;
  flushing = true;
  const entries = queue.splice(0);
  void appendGuiLog(entries).catch(() => undefined).finally(() => { flushing = false; flush(); });
}

/** Capture browser console output without changing its developer-tools behavior. */
export function installGuiLogger(windowLabel = 'main'): void {
  const methods: Array<'debug' | 'log' | 'info' | 'warn' | 'error'> = ['debug', 'log', 'info', 'warn', 'error'];
  for (const level of methods) {
    const original = console[level].bind(console);
    console[level] = (...args: unknown[]) => {
      original(...args);
      queue.push({ timestamp: new Date().toISOString(), level, source: 'gui', message: args.map(text).join(' '), args: sanitize(args), windowLabel });
      if (queue.length > MAX_QUEUE) queue.shift();
      flush();
    };
  }
  window.addEventListener('error', event => console.error('window.error', { message: event.message, filename: event.filename, lineno: event.lineno, colno: event.colno }));
  window.addEventListener('unhandledrejection', event => console.error('unhandledrejection', event.reason));
}
