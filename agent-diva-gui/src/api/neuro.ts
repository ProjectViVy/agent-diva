import { invoke } from "@tauri-apps/api/core";

import type { CortexState } from "./cortex";

/** 与 Rust `NeuroDataPhase` 线格式一致（serde `lowercase`）。 */
export type NeuroDataPhase = "live" | "stub" | "degraded";

/** 与 `agent-diva-swarm` `NeuroActivityRowV0`（camelCase JSON）一致。 */
export interface NeuroActivityRowV0 {
  id: string;
  label: string;
  detail?: string | null;
  status: string;
}

/** 与 `agent-diva-swarm` `NeuroOverviewSnapshotV0` 一致；`cortex` 与 `get_cortex_state` 同源。 */
export interface NeuroOverviewSnapshotV0 {
  schemaVersion: number;
  dataPhase: NeuroDataPhase;
  cortex: CortexState;
  leftRows: NeuroActivityRowV0[];
  rightRows: NeuroActivityRowV0[];
}

export const getNeuroOverviewSnapshot = () =>
  invoke<NeuroOverviewSnapshotV0>("get_neuro_overview_snapshot");

export function rowsForHemisphere(
  snap: NeuroOverviewSnapshotV0,
  side: "left" | "right",
): NeuroActivityRowV0[] {
  return side === "left" ? snap.leftRows : snap.rightRows;
}

/** 浏览器预览：无 Tauri 时的诚实 stub，禁止冒充 live。 */
export function previewNeuroOverviewSnapshot(): NeuroOverviewSnapshotV0 {
  return {
    schemaVersion: 0,
    dataPhase: "stub",
    cortex: { enabled: true, schemaVersion: 0 },
    leftRows: [],
    rightRows: [],
  };
}
