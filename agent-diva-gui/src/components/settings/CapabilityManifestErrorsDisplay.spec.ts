import { describe, expect, it } from "vitest";
import { mount } from "@vue/test-utils";
import { createI18n } from "vue-i18n";

import CapabilityManifestErrorsDisplay from "./CapabilityManifestErrorsDisplay.vue";

const i18n = createI18n({
  legacy: false,
  locale: "en",
  messages: {
    en: {
      settings: {
        capabilityManifest: {
          fileLevelTitle: "File-level",
          fieldLevelTitle: "Field-level",
        },
      },
    },
  },
});

describe("CapabilityManifestErrorsDisplay", () => {
  it("renders file-level and field-level groups for mixed errors", () => {
    const wrapper = mount(CapabilityManifestErrorsDisplay, {
      props: {
        errors: [
          {
            code: "capability.manifest.json_parse",
            message: "invalid JSON",
            location_kind: "file",
          },
          {
            code: "capability.manifest.missing_field",
            message: 'missing required field "id"',
            location_kind: "field",
            path: "/capabilities/0/id",
          },
        ],
      },
      global: { plugins: [i18n] },
    });

    expect(wrapper.text()).toContain("invalid JSON");
    expect(wrapper.text()).toContain('missing required field "id"');
    expect(wrapper.text()).toContain("/capabilities/0/id");
    expect(wrapper.text()).toContain("File-level");
    expect(wrapper.text()).toContain("Field-level");
    expect(wrapper.find('[role="alert"]').exists()).toBe(true);
  });

  it("renders a single file-level error as one list item", () => {
    const wrapper = mount(CapabilityManifestErrorsDisplay, {
      props: {
        errors: [
          {
            code: "capability.manifest.json_parse",
            message: "truncated",
            location_kind: "file",
          },
        ],
      },
      global: { plugins: [i18n] },
    });

    expect(wrapper.findAll("li")).toHaveLength(1);
    expect(wrapper.text()).toContain("truncated");
  });

  it("renders multiple field errors as separate rows", () => {
    const wrapper = mount(CapabilityManifestErrorsDisplay, {
      props: {
        errors: [
          {
            code: "a",
            message: "m1",
            location_kind: "field",
            path: "/a",
          },
          {
            code: "b",
            message: "m2",
            location_kind: "field",
            path: "/b",
          },
        ],
      },
      global: { plugins: [i18n] },
    });

    expect(wrapper.findAll("li")).toHaveLength(2);
  });
});
