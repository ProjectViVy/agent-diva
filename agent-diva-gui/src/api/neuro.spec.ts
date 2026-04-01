import { describe, expect, it } from "vitest";

import { previewNeuroOverviewSnapshot, rowsForHemisphere } from "./neuro";

describe("neuro overview helpers", () => {
  it("rowsForHemisphere returns the correct side", () => {
    const snap = previewNeuroOverviewSnapshot();
    const leftish = { ...snap, leftRows: [{ id: "a", label: "L", status: "idle" }] };
    expect(rowsForHemisphere(leftish, "left")).toHaveLength(1);
    expect(rowsForHemisphere(leftish, "right")).toHaveLength(0);
  });

  it("preview snapshot is stub with empty rows", () => {
    const p = previewNeuroOverviewSnapshot();
    expect(p.dataPhase).toBe("stub");
    expect(p.leftRows).toEqual([]);
    expect(p.cortex.enabled).toBe(true);
  });
});
