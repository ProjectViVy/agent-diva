import { describe, expect, it, vi, beforeEach } from "vitest";
import { mount, flushPromises } from "@vue/test-utils";
import { createI18n } from "vue-i18n";

const invokeMock = vi.fn();

vi.mock("@tauri-apps/api/core", () => ({
  invoke: (cmd: string, args?: unknown) => invokeMock(cmd, args),
}));

vi.mock("@tauri-apps/api/event", () => ({
  listen: vi.fn().mockResolvedValue(() => {}),
}));

const showAppToast = vi.fn();
vi.mock("../utils/appToast", () => ({
  showAppToast: (...args: unknown[]) => showAppToast(...args),
}));

import { CORTEX_SYNC_REJECTED } from "../api/cortex";
import CortexToggle from "./CortexToggle.vue";

const i18n = createI18n({
  legacy: false,
  locale: "zh",
  messages: {
    zh: {
      cortex: {
        layerLabel: "蜂群层",
        stateOn: "已开启",
        stateOff: "已关闭",
        toggleSyncFailed: "通用同步失败",
        toggleSyncFailedCodeRejected: "同步被拒绝",
      },
    },
  },
});

describe("CortexToggle", () => {
  beforeEach(() => {
    invokeMock.mockReset();
    showAppToast.mockReset();
  });

  it("on toggle invoke failure keeps switch state and shows error toast", async () => {
    invokeMock
      .mockResolvedValueOnce({ enabled: false, schemaVersion: 0 })
      .mockRejectedValueOnce(new Error(CORTEX_SYNC_REJECTED))
      .mockResolvedValueOnce({ enabled: false, schemaVersion: 0 });

    const wrapper = mount(CortexToggle, {
      global: { plugins: [i18n] },
    });

    await flushPromises();
    const btn = wrapper.get('[role="switch"]');
    expect(btn.attributes("aria-checked")).toBe("false");

    await btn.trigger("click");
    await flushPromises();

    expect(btn.attributes("aria-checked")).toBe("false");
    expect(showAppToast).toHaveBeenCalledWith(
      "同步被拒绝",
      "error",
      5200,
    );
  });

  it("on generic invoke failure uses generic i18n message", async () => {
    invokeMock
      .mockResolvedValueOnce({ enabled: true, schemaVersion: 0 })
      .mockRejectedValueOnce(new Error("network down"))
      .mockResolvedValueOnce({ enabled: true, schemaVersion: 0 });

    const wrapper = mount(CortexToggle, {
      global: { plugins: [i18n] },
    });

    await flushPromises();
    const btn = wrapper.get('[role="switch"]');
    expect(btn.attributes("aria-checked")).toBe("true");

    await btn.trigger("click");
    await flushPromises();

    expect(btn.attributes("aria-checked")).toBe("true");
    expect(showAppToast).toHaveBeenCalledWith(
      "通用同步失败",
      "error",
      5200,
    );
  });
});
