import { describe, expect, it } from "vitest";
import { mount } from "@vue/test-utils";
import { createI18n } from "vue-i18n";

import type { NeuroOverviewSnapshotV0 } from "../../api/neuro";
import NeuroDetailPanel from "./NeuroDetailPanel.vue";

const troubleshootEn = {
  errorTitle: "Err title",
  errorBody: "Err body",
  emptyNoSnapshotTitle: "No snap title",
  emptyNoSnapshotBody: "No snap body",
  emptyNoRowsTitle: "No rows title",
  emptyStubRowsBody: "Stub rows body",
  emptyDegradedRowsBody: "Deg rows body",
  idleTitle: "Idle title",
  idleBody: "Idle body",
  actionRetry: "Retry",
  actionBackToChat: "Back",
  actionOpenSettings: "Settings",
  actionDisableCortex: "Cortex off",
};

const messages = {
  en: {
    neuro: {
      regionCortexHub: "Cortex hub",
      regionMotorBridge: "Motor bridge",
      detailPanelSubtitle: "Subtitle",
      detailLoading: "Loading",
      detailStubExplainer: "Stub honest",
      detailDegradedExplainer: "Degraded honest",
      dataPhase: { live: "Live", stub: "Stub", degraded: "Off" },
      troubleshoot: troubleshootEn,
    },
  },
};

const i18n = createI18n({ legacy: false, locale: "en", messages });

const stubSnap = (): NeuroOverviewSnapshotV0 => ({
  schemaVersion: 0,
  dataPhase: "stub",
  cortex: { enabled: true, schemaVersion: 0 },
  leftRows: [],
  rightRows: [],
});

describe("NeuroDetailPanel", () => {
  it("shows empty troubleshoot callout when stub and no rows", () => {
    const w = mount(NeuroDetailPanel, {
      props: { side: "left", snapshot: stubSnap() },
      global: { plugins: [i18n] },
    });
    expect(w.find('[data-testid="data-phase-badge"]').exists()).toBe(true);
    expect(w.text()).toContain("Stub honest");
    expect(w.find('[data-testid="neuro-troubleshoot-empty"]').exists()).toBe(true);
    expect(w.text()).toContain("Stub rows body");
    expect(w.find('[data-testid="neuro-detail-rows"]').exists()).toBe(false);
  });

  it("shows idle troubleshoot when live with zero rows", () => {
    const snap: NeuroOverviewSnapshotV0 = { ...stubSnap(), dataPhase: "live" };
    const w = mount(NeuroDetailPanel, {
      props: { side: "left", snapshot: snap },
      global: { plugins: [i18n] },
    });
    expect(w.find('[data-testid="neuro-troubleshoot-idle"]').exists()).toBe(true);
    expect(w.text()).toContain("Idle body");
  });

  it("shows error troubleshoot when loadError", () => {
    const w = mount(NeuroDetailPanel, {
      props: { side: "left", snapshot: null, loadError: true },
      global: { plugins: [i18n] },
    });
    expect(w.find('[data-testid="neuro-troubleshoot-error"]').exists()).toBe(true);
    expect(w.text()).toContain("Err body");
  });

  it("hides disable-cortex action when showDisableCortexAction is false", () => {
    const w = mount(NeuroDetailPanel, {
      props: {
        side: "left",
        snapshot: null,
        loadError: true,
        showDisableCortexAction: false,
      },
      global: { plugins: [i18n] },
    });
    expect(w.findAll("button").map((b) => b.text())).toEqual([
      "Retry",
      "Settings",
    ]);
  });

  it("lists rows when live with data", () => {
    const snap: NeuroOverviewSnapshotV0 = {
      ...stubSnap(),
      dataPhase: "live",
      leftRows: [{ id: "1", label: "Phase A", status: "active" }],
    };
    const w = mount(NeuroDetailPanel, {
      props: { side: "left", snapshot: snap },
      global: { plugins: [i18n] },
    });
    expect(w.findAll('[data-testid="neuro-detail-rows"] li')).toHaveLength(1);
    expect(w.text()).toContain("Phase A");
    expect(w.find('[data-testid="neuro-troubleshoot-idle"]').exists()).toBe(false);
  });

  it("emits retry when error action clicked", async () => {
    const w = mount(NeuroDetailPanel, {
      props: { side: "left", snapshot: null, loadError: true },
      global: { plugins: [i18n] },
    });
    const buttons = w.findAll("button");
    const retry = buttons.find((b) => b.text() === "Retry");
    expect(retry).toBeDefined();
    await retry!.trigger("click");
    expect(w.emitted("retry")).toBeTruthy();
  });
});
