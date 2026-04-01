import { invoke } from '@tauri-apps/api/core';

/** 与 `agent_diva_core::bus::RunTelemetrySnapshotV0`（camelCase JSON）一致（FR22 / ADR-E / Story 5.2）。 */
export interface RunTelemetrySnapshotV0 {
  schemaVersion: number;
  /** 主 ReAct 迭代次数（不含序曲 LLM）。 */
  internalStepCount: number;
  /** 蜂群序曲 LLM 调用次数；缺省视为 0（旧网关载荷）。 */
  preludeLlmCalls?: number;
  /** 与 `swarm_phase_changed` 发射次数对齐（序曲 + 主循环，有管道时）。 */
  phaseCount: number;
  overSuggestedBudget?: boolean | null;
  /** FullSwarm 收敛循环 `rounds_completed`；缺省表示未进入或未上报。 */
  fullSwarmConvergenceRounds?: number | null;
}

export const getRunTelemetrySnapshot = () =>
  invoke<RunTelemetrySnapshotV0 | null>('get_run_telemetry_snapshot');
