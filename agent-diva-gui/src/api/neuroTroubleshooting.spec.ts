import { describe, expect, it } from "vitest";

import type { NeuroOverviewSnapshotV0 } from "./neuro";
import { deriveNeuroTroubleshootTemplate } from "./neuroTroubleshooting";

const baseSnap = (): NeuroOverviewSnapshotV0 => ({
  schemaVersion: 0,
  dataPhase: "stub",
  cortex: { enabled: true, schemaVersion: 0 },
  leftRows: [],
  rightRows: [],
});

describe("deriveNeuroTroubleshootTemplate", () => {
  it("returns null while loading", () => {
    expect(
      deriveNeuroTroubleshootTemplate({
        loading: true,
        loadError: false,
        snapshot: baseSnap(),
        side: "left",
      }),
    ).toBeNull();
  });

  it("returns error variant when loadError", () => {
    const t = deriveNeuroTroubleshootTemplate({
      loading: false,
      loadError: true,
      snapshot: null,
      side: "left",
    });
    expect(t?.variant).toBe("error");
    expect(t?.suggestedActions.map((a) => a.behavior.event)).toEqual([
      "retry",
      "open-settings",
      "disable-cortex",
    ]);
  });

  it("omits disable-cortex when showDisableCortexAction is false", () => {
    const t = deriveNeuroTroubleshootTemplate({
      loading: false,
      loadError: true,
      snapshot: null,
      side: "left",
      showDisableCortexAction: false,
    });
    expect(t?.suggestedActions.map((a) => a.behavior.event)).toEqual([
      "retry",
      "open-settings",
    ]);
  });

  it("returns empty variant when no snapshot and no error", () => {
    const t = deriveNeuroTroubleshootTemplate({
      loading: false,
      loadError: false,
      snapshot: null,
      side: "left",
    });
    expect(t?.variant).toBe("empty");
    expect(t?.suggestedActions.some((a) => a.behavior.event === "back-to-chat")).toBe(
      true,
    );
  });

  it("returns null when rows exist for side", () => {
    const snap: NeuroOverviewSnapshotV0 = {
      ...baseSnap(),
      dataPhase: "live",
      leftRows: [{ id: "a", label: "x", status: "active" }],
    };
    expect(
      deriveNeuroTroubleshootTemplate({
        loading: false,
        loadError: false,
        snapshot: snap,
        side: "left",
      }),
    ).toBeNull();
  });

  it("returns idle when live and zero rows on side", () => {
    const snap: NeuroOverviewSnapshotV0 = {
      ...baseSnap(),
      dataPhase: "live",
    };
    const t = deriveNeuroTroubleshootTemplate({
      loading: false,
      loadError: false,
      snapshot: snap,
      side: "left",
    });
    expect(t?.variant).toBe("idle");
  });

  it("returns empty stub copy when stub and zero rows", () => {
    const t = deriveNeuroTroubleshootTemplate({
      loading: false,
      loadError: false,
      snapshot: baseSnap(),
      side: "left",
    });
    expect(t?.variant).toBe("empty");
    expect(t?.bodyKey).toBe("neuro.troubleshoot.emptyStubRowsBody");
  });

  it("returns empty degraded copy when degraded and zero rows", () => {
    const snap: NeuroOverviewSnapshotV0 = {
      ...baseSnap(),
      dataPhase: "degraded",
    };
    const t = deriveNeuroTroubleshootTemplate({
      loading: false,
      loadError: false,
      snapshot: snap,
      side: "left",
    });
    expect(t?.variant).toBe("empty");
    expect(t?.bodyKey).toBe("neuro.troubleshoot.emptyDegradedRowsBody");
  });
});
