import { describe, expect, it } from "vitest";
import { mount, flushPromises } from "@vue/test-utils";
import { createI18n } from "vue-i18n";
import ProcessFeedbackStrip from "./ProcessFeedbackStrip.vue";
import type { ProcessEventWire } from "../../types/swarmProcess";

const i18n = createI18n({
  legacy: false,
  locale: "en",
  messages: {
    en: {
      processFeedback: {
        statusRunning: "In progress",
        statusDone: "Finished",
        statusCapped: "Capped",
        expand: "Show",
        collapse: "Hide",
        summaryTool: "Tool: {tool}",
        stepToolStart: "Started {name}",
        stepToolEnd: "Finished {name}",
        stepFinished: "Done ({reason})",
        stepCapped: "Capped",
      },
    },
  },
});

function phase(msg: string): ProcessEventWire {
  return { schemaVersion: 0, name: "swarm_phase_changed", message: msg, phaseId: "p1" };
}

async function flushRaf() {
  await new Promise<void>((resolve) => requestAnimationFrame(() => resolve()));
}

describe("ProcessFeedbackStrip", () => {
  it("toggles collapse and shows capped status after streaming → capped", async () => {
    const wrapper = mount(ProcessFeedbackStrip, {
      props: { show: true, events: [phase("iter 1")] },
      global: { plugins: [i18n] },
    });
    await flushPromises();
    await flushRaf();
    expect(wrapper.find('[data-testid="process-feedback-strip"]').exists()).toBe(true);
    expect(wrapper.text()).toContain("In progress");

    await wrapper.get('[data-testid="process-feedback-toggle"]').trigger("click");
    expect(wrapper.text()).toContain("Show");

    await wrapper.setProps({
      events: [
        phase("iter 1"),
        {
          schemaVersion: 0,
          name: "swarm_run_capped",
          message: "budget hit",
          stopReason: "budgetExceeded",
        },
      ],
    });
    await flushPromises();
    await flushRaf();
    expect(wrapper.text()).toContain("Capped");
  });
});
