/**
 * useSubAgents — sub-agent lifecycle state composable.
 *
 * Polls for active/completed child agent results. In mock mode (Tauri
 * not available), returns sample data for development. Once the Rust
 * backend exposes a `list_subagent_results` IPC command, swap the mock
 * for a real `invoke()` call.
 */

import { computed, onUnmounted, ref } from 'vue';
import { invoke } from '@tauri-apps/api/core';
import { isTauriRuntime } from '../api/desktop';

// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------

/** Terminal status of a sub-agent task (mirrors Rust SubAgentStatus). */
export type SubAgentStatus = 'ok' | 'error' | 'timeout' | 'cancelled';

/** A single sub-agent result entry. */
export interface SubAgentEntry {
  task_id: string;
  status: SubAgentStatus;
  summary?: string;
  elapsed_ms: number;
  tool_call_count: number;
  started_at?: number; // epoch ms — used for active elapsed timer
}

// ---------------------------------------------------------------------------
// Module state (shared across component instances)
// ---------------------------------------------------------------------------

const children = ref<SubAgentEntry[]>([]);
const polling = ref(false);
let timer: ReturnType<typeof setInterval> | null = null;

// ---------------------------------------------------------------------------
// Mock data
// ---------------------------------------------------------------------------

const MOCK_CHILDREN: SubAgentEntry[] = [
  {
    task_id: 'a1b2c3',
    status: 'ok',
    summary: 'Analyzed 3 log files and found 2 anomalies',
    elapsed_ms: 4520,
    tool_call_count: 6,
  },
  {
    task_id: 'd4e5f6',
    status: 'error',
    summary: 'Failed to connect to remote API: timeout',
    elapsed_ms: 12300,
    tool_call_count: 3,
  },
  {
    task_id: 'g7h8i9',
    status: 'timeout',
    summary: 'Task timed out after 30s',
    elapsed_ms: 30000,
    tool_call_count: 0,
  },
  {
    task_id: 'j0k1l2',
    status: 'cancelled',
    summary: undefined,
    elapsed_ms: 800,
    tool_call_count: 1,
  },
];

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

/**
 * Composable for sub-agent lifecycle monitoring.
 *
 * @param pollIntervalMs — how often to poll (default 3000ms)
 *
 * @example
 * ```ts
 * const { children, activeChildren, completedChildren, startPolling, stopPolling } = useSubAgents(2000);
 * startPolling();
 * ```
 */
export function useSubAgents(pollIntervalMs = 3000) {
  /** Only active (running) children — currently always empty in poll mode. */
  const activeChildren = computed(() =>
    children.value.filter((c) => !c.status || c.status === ('running' as unknown as SubAgentStatus)),
  );

  /** Completed / errored / timed-out children. */
  const completedChildren = computed(() =>
    children.value.filter((c) => c.status && c.status !== ('running' as unknown as SubAgentStatus)),
  );

  /** Total count badge text. */
  const totalCount = computed(() => children.value.length);

  async function fetchResults(): Promise<void> {
    if (!isTauriRuntime()) {
      // Mock mode
      children.value = MOCK_CHILDREN;
      return;
    }

    try {
      const result = await invoke<SubAgentEntry[]>('list_subagent_results');
      children.value = result;
    } catch {
      // Backend command not yet wired — keep current state
    }
  }

  function startPolling(): void {
    if (polling.value) return;
    polling.value = true;
    fetchResults();
    timer = setInterval(fetchResults, pollIntervalMs);
  }

  function stopPolling(): void {
    if (timer) {
      clearInterval(timer);
      timer = null;
    }
    polling.value = false;
  }

  /** Format elapsed_ms to a human-readable string. */
  function formatElapsed(ms: number): string {
    if (ms < 1000) return `${ms}ms`;
    const secs = Math.floor(ms / 1000);
    if (secs < 60) return `${secs}s`;
    const mins = Math.floor(secs / 60);
    const remainSecs = secs % 60;
    return `${mins}m${remainSecs}s`;
  }

  /** Get status icon for a sub-agent status. */
  function statusIcon(status: SubAgentStatus): string {
    switch (status) {
      case 'ok':
        return '✅';
      case 'error':
        return '❌';
      case 'timeout':
        return '⏱️';
      case 'cancelled':
        return '🚫';
      default:
        return '❓';
    }
  }

  onUnmounted(() => {
    stopPolling();
  });

  return {
    children,
    activeChildren,
    completedChildren,
    totalCount,
    polling,
    startPolling,
    stopPolling,
    fetchResults,
    formatElapsed,
    statusIcon,
  };
}
