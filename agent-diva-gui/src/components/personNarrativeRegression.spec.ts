/**
 * Story 4.1 — Person 单一叙事回归（FR8 / FR9）
 * 机器断言：用户可见区域内「独立 agent 对话流」壳体 data-testid 数量 ≤ 1；
 * 神经视图不得再挂载第二套并列聊天壳。
 */
import { describe, expect, it, vi, beforeEach } from "vitest";
import { mount, flushPromises } from "@vue/test-utils";
import { createI18n } from "vue-i18n";

import en from "../locales/en";
import zh from "../locales/zh";
import ChatView from "./ChatView.vue";
import NormalMode from "./NormalMode.vue";
import NervousSystemView from "./neuro/NervousSystemView.vue";

const invokeMock = vi.fn();

vi.mock("@tauri-apps/api/core", () => ({
  invoke: (cmd: string, args?: unknown) => invokeMock(cmd, args),
}));

vi.mock("@tauri-apps/api/event", () => ({
  listen: vi.fn().mockResolvedValue(() => {}),
}));

const i18n = createI18n({
  legacy: false,
  locale: "en",
  messages: { en, zh },
});

const STREAM = "[data-testid=\"person-agent-conversation-stream\"]";
const TRANSCRIPT = "[data-testid=\"person-main-transcript\"]";
const COMPOSER = "[data-testid=\"person-main-composer\"]";
const USER_ROOT = "[data-testid=\"user-visible-app-root\"]";

function minimalNormalModeProps() {
  return {
    messages: [] as { role: "user"; content: string }[],
    isTyping: false,
    chatDisplayPrefs: {
      autoExpandReasoning: false,
      autoExpandToolDetails: false,
      showRawMetaByDefault: false,
    },
    saveConfigAction: vi.fn().mockResolvedValue(undefined),
    saveToolsConfigAction: vi.fn().mockResolvedValue(undefined),
    saveChannelConfigAction: vi.fn().mockResolvedValue(undefined),
  };
}

function appVisibleRoot(wrapper: { find: (s: string) => { element: HTMLElement } }) {
  return wrapper.find(USER_ROOT).element;
}

describe("Story 4.1 person narrative regression", () => {
  beforeEach(() => {
    invokeMock.mockReset();
    invokeMock.mockImplementation((cmd: string) => {
      if (cmd === "reset_session") return Promise.resolve();
      return Promise.resolve({});
    });
  });

  it("path A/B chat: exactly one conversation stream shell with one transcript and one composer", () => {
    const wrapper = mount(ChatView, {
      props: {
        messages: [],
        isTyping: false,
        themeMode: "love",
      },
      global: { plugins: [i18n] },
    });

    const root = wrapper.element as HTMLElement;
    const streams = root.querySelectorAll(STREAM);
    expect(streams.length).toBe(1);

    const shell = root.querySelector(STREAM) as HTMLElement;
    expect(shell.querySelectorAll(TRANSCRIPT).length).toBe(1);
    expect(shell.querySelectorAll(COMPOSER).length).toBe(1);

    const transcript = shell.querySelector(TRANSCRIPT) as HTMLElement;
    expect(transcript.getAttribute("role")).toBe("log");
    expect(transcript.getAttribute("aria-label")).toBe(
      (en as { chat: { mainTranscriptAria: string } }).chat.mainTranscriptAria,
    );
  });

  it("path A/B chat with messages: still single stream (no duplicate shells)", () => {
    const wrapper = mount(ChatView, {
      props: {
        messages: [
          { role: "user" as const, content: "hi" },
          { role: "agent" as const, content: "hello" },
        ],
        isTyping: false,
      },
      global: { plugins: [i18n] },
    });

    const root = wrapper.element as HTMLElement;
    expect(root.querySelectorAll(STREAM).length).toBe(1);
  });

  it("simulated Tauri path: cortex bar may appear but does not add a second conversation stream", async () => {
    vi.stubGlobal("__TAURI_INTERNALS__", {});
    try {
      invokeMock.mockResolvedValue({ enabled: true, schemaVersion: 0 });

      const wrapper = mount(ChatView, {
        props: { messages: [], isTyping: false },
        global: { plugins: [i18n] },
      });
      await flushPromises();

      const root = wrapper.element as HTMLElement;
      expect(root.querySelectorAll(STREAM).length).toBe(1);
      expect(wrapper.find('[data-testid="cortex-toggle"]').exists()).toBe(true);
    } finally {
      vi.unstubAllGlobals();
    }
  });

  it("neuro view: no person-agent-conversation-stream (FR9 — not a second visible chat room)", () => {
    const wrapper = mount(NervousSystemView, {
      global: { plugins: [i18n] },
    });

    const root = wrapper.element as HTMLElement;
    expect(root.querySelectorAll(STREAM).length).toBe(0);
    expect(root.querySelectorAll(COMPOSER).length).toBe(0);
  });

  it("zh locale: aria labels do not imply multiple peer assistants", () => {
    const zhI18n = createI18n({
      legacy: false,
      locale: "zh",
      messages: { zh },
    });
    const wrapper = mount(ChatView, {
      props: { messages: [], isTyping: false },
      global: { plugins: [zhI18n] },
    });
    const shell = wrapper.find(STREAM);
    const zhChat = (zh as {
      chat: {
        primaryPersonNarrativeAria: string;
        mainTranscriptAria: string;
        mainComposerAria: string;
      };
    }).chat;
    expect(shell.attributes("aria-label")).toBe(zhChat.primaryPersonNarrativeAria);
    const transcript = wrapper.find(TRANSCRIPT);
    expect(transcript.attributes("aria-label")).toBe(zhChat.mainTranscriptAria);
    const composer = wrapper.find(COMPOSER);
    expect(composer.attributes("aria-label")).toBe(zhChat.mainComposerAria);
  });

  it("under user-visible-app-root (NormalMode chat): exactly one conversation stream", async () => {
    const wrapper = mount(NormalMode, {
      props: minimalNormalModeProps(),
      global: { plugins: [i18n] },
    });
    await flushPromises();
    const root = appVisibleRoot(wrapper);
    expect(root.querySelectorAll(STREAM).length).toBe(1);
    expect(root.querySelectorAll(TRANSCRIPT).length).toBe(1);
    expect(root.querySelectorAll(COMPOSER).length).toBe(1);
  });

  it("under user-visible-app-root: Neuro route has no person conversation stream", async () => {
    const wrapper = mount(NormalMode, {
      props: minimalNormalModeProps(),
      global: { plugins: [i18n] },
    });
    await flushPromises();
    await wrapper.find(".app-titlebar button").trigger("click");
    await flushPromises();
    const neuroBtn = wrapper
      .findAll("aside button")
      .find((b) => b.text().includes("Neuro"));
    expect(neuroBtn).toBeDefined();
    await neuroBtn!.trigger("click");
    await flushPromises();
    const root = appVisibleRoot(wrapper);
    expect(root.querySelectorAll(STREAM).length).toBe(0);
    expect(root.querySelectorAll(COMPOSER).length).toBe(0);
  });

  it("under user-visible-app-root: Neuro then Chat again — still single stream", async () => {
    const wrapper = mount(NormalMode, {
      props: minimalNormalModeProps(),
      global: { plugins: [i18n] },
    });
    await flushPromises();

    async function openSidebar() {
      await wrapper.find(".app-titlebar button").trigger("click");
      await flushPromises();
    }

    await openSidebar();
    const neuroBtn = wrapper
      .findAll("aside button")
      .find((b) => b.text().includes("Neuro"));
    await neuroBtn!.trigger("click");
    await flushPromises();
    expect(appVisibleRoot(wrapper).querySelectorAll(STREAM).length).toBe(0);

    await openSidebar();
    const chatBtn = wrapper
      .findAll("aside button")
      .find((b) => b.text().includes("Chat"));
    await chatBtn!.trigger("click");
    await flushPromises();
    const root = appVisibleRoot(wrapper);
    expect(root.querySelectorAll(STREAM).length).toBe(1);
  });

  it("under user-visible-app-root + Tauri: cortex bar without second stream", async () => {
    vi.stubGlobal("__TAURI_INTERNALS__", {});
    try {
      invokeMock.mockResolvedValue({ enabled: true, schemaVersion: 0 });
      const wrapper = mount(NormalMode, {
        props: minimalNormalModeProps(),
        global: { plugins: [i18n] },
      });
      await flushPromises();
      const root = appVisibleRoot(wrapper);
      expect(root.querySelectorAll(STREAM).length).toBe(1);
      expect(wrapper.find('[data-testid="cortex-toggle"]').exists()).toBe(true);
    } finally {
      vi.unstubAllGlobals();
    }
  });
});
