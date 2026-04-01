import type { NeuroOverviewSnapshotV0 } from "./neuro";
import { rowsForHemisphere } from "./neuro";

/**
 * 神经系统详情排障模板（Story 3.3 / FR7 / UX-IMPL-3）。
 * 与 `NeuroOverviewSnapshotV0` 及 `rowsForHemisphere` 同源推导，不在此伪造连接状态。
 */
export type NeuroTroubleshootVariant = "empty" | "error" | "idle";

/** 建议动作：由 UI 层映射为 emit / 路由 / invoke（本仓库仅 emit）。 */
export type NeuroTroubleshootEmit =
  | "retry"
  | "back-to-chat"
  | "open-settings"
  | "disable-cortex";

export interface NeuroTroubleshootSuggestedAction {
  /** i18n 键，相对于根 messages，如 `neuro.troubleshoot.actionRetry` */
  labelKey: string;
  behavior: { type: "emit"; event: NeuroTroubleshootEmit };
}

export interface NeuroTroubleshootTemplate {
  variant: NeuroTroubleshootVariant;
  /** 与 i18n 键一致，供 `t(titleKey)` */
  titleKey: string;
  bodyKey: string;
  suggestedActions: NeuroTroubleshootSuggestedAction[];
}

/**
 * 从与 BrainOverview / 详情列表相同快照推导应展示的排障模板。
 * - `loading === true` 时不展示本模板（与「闲态」区分）。
 * - `rows.length > 0` 时不展示（主内容区为列表）。
 */
export function deriveNeuroTroubleshootTemplate(input: {
  loading: boolean;
  loadError: boolean;
  snapshot: NeuroOverviewSnapshotV0 | null;
  side: "left" | "right";
  /** 非 Tauri（如浏览器预览）无 `set_cortex_enabled`，应置为 false。默认 true。 */
  showDisableCortexAction?: boolean;
}): NeuroTroubleshootTemplate | null {
  const {
    loading,
    loadError,
    snapshot,
    side,
    showDisableCortexAction = true,
  } = input;
  if (loading) return null;

  if (loadError) {
    const suggestedActions: NeuroTroubleshootSuggestedAction[] = [
      {
        labelKey: "neuro.troubleshoot.actionRetry",
        behavior: { type: "emit", event: "retry" },
      },
      {
        labelKey: "neuro.troubleshoot.actionOpenSettings",
        behavior: { type: "emit", event: "open-settings" },
      },
    ];
    if (showDisableCortexAction) {
      suggestedActions.push({
        labelKey: "neuro.troubleshoot.actionDisableCortex",
        behavior: { type: "emit", event: "disable-cortex" },
      });
    }
    return {
      variant: "error",
      titleKey: "neuro.troubleshoot.errorTitle",
      bodyKey: "neuro.troubleshoot.errorBody",
      suggestedActions,
    };
  }

  if (!snapshot) {
    return {
      variant: "empty",
      titleKey: "neuro.troubleshoot.emptyNoSnapshotTitle",
      bodyKey: "neuro.troubleshoot.emptyNoSnapshotBody",
      suggestedActions: [
        {
          labelKey: "neuro.troubleshoot.actionBackToChat",
          behavior: { type: "emit", event: "back-to-chat" },
        },
      ],
    };
  }

  const rows = rowsForHemisphere(snapshot, side);
  if (rows.length > 0) return null;

  if (snapshot.dataPhase === "live") {
    return {
      variant: "idle",
      titleKey: "neuro.troubleshoot.idleTitle",
      bodyKey: "neuro.troubleshoot.idleBody",
      suggestedActions: [
        {
          labelKey: "neuro.troubleshoot.actionBackToChat",
          behavior: { type: "emit", event: "back-to-chat" },
        },
      ],
    };
  }

  const isStub = snapshot.dataPhase === "stub";
  return {
    variant: "empty",
    titleKey: "neuro.troubleshoot.emptyNoRowsTitle",
    bodyKey: isStub
      ? "neuro.troubleshoot.emptyStubRowsBody"
      : "neuro.troubleshoot.emptyDegradedRowsBody",
    suggestedActions: [
      {
        labelKey: "neuro.troubleshoot.actionBackToChat",
        behavior: { type: "emit", event: "back-to-chat" },
      },
    ],
  };
}
