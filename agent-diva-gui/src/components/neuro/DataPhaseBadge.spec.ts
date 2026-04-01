import { describe, expect, it } from "vitest";
import { mount } from "@vue/test-utils";
import { createI18n } from "vue-i18n";

import DataPhaseBadge from "./DataPhaseBadge.vue";

const i18n = createI18n({
  legacy: false,
  locale: "en",
  messages: {
    en: {
      neuro: {
        dataPhase: { live: "Live", stub: "No live stream", degraded: "Cortex off" },
      },
    },
  },
});

describe("DataPhaseBadge", () => {
  it("renders stub label and test id", () => {
    const w = mount(DataPhaseBadge, {
      props: { phase: "stub" },
      global: { plugins: [i18n] },
    });
    expect(w.find('[data-testid="data-phase-badge"]').exists()).toBe(true);
    expect(w.text()).toContain("No live stream");
  });

  it("renders degraded label", () => {
    const w = mount(DataPhaseBadge, {
      props: { phase: "degraded" },
      global: { plugins: [i18n] },
    });
    expect(w.text()).toContain("Cortex off");
  });
});
