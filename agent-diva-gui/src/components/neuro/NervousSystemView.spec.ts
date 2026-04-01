import { describe, expect, it } from "vitest";
import { mount } from "@vue/test-utils";
import { createI18n } from "vue-i18n";

import en from "../../locales/en";
import NervousSystemView from "./NervousSystemView.vue";

const i18n = createI18n({
  legacy: false,
  locale: "en",
  messages: { en },
});

describe("NervousSystemView", () => {
  it("renders BrainOverview as primary content and an optional collapsed vision stub", () => {
    const wrapper = mount(NervousSystemView, {
      global: { plugins: [i18n] },
    });

    expect(wrapper.find(".brain-overview").exists()).toBe(true);

    const details = wrapper.find("details.vision-stub");
    expect(details.exists()).toBe(true);
    const el = details.element as HTMLDetailsElement;
    expect(el.open).toBe(false);

    const badge = (en as { neuro: { vision: { badge: string } } }).neuro.vision.badge;
    expect(wrapper.text()).toContain(badge);
  });

  it("binds vision subtitle and body to i18n keys in the details panel", () => {
    const wrapper = mount(NervousSystemView, {
      global: { plugins: [i18n] },
    });

    const vision = (en as { neuro: { vision: { subtitle: string; body: string } } }).neuro.vision;
    const paras = wrapper.findAll("details.vision-stub .space-y-2 p");
    expect(paras).toHaveLength(2);
    expect(paras[0].text()).toBe(vision.subtitle);
    expect(paras[1].text()).toBe(vision.body);
  });
});
