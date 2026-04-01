import { describe, expect, it } from "vitest";

import en from "../../locales/en";
import zh from "../../locales/zh";

describe("neuro.vision i18n (FR16 / Story 3.4)", () => {
  const keys = ["badge", "title", "subtitle", "body"] as const;

  it("defines the same keys in en and zh", () => {
    const enVision = (en as { neuro: { vision: Record<string, string> } }).neuro.vision;
    const zhVision = (zh as { neuro: { vision: Record<string, string> } }).neuro.vision;
    for (const k of keys) {
      expect(typeof enVision[k]).toBe("string");
      expect(enVision[k].length).toBeGreaterThan(0);
      expect(typeof zhVision[k]).toBe("string");
      expect(zhVision[k].length).toBeGreaterThan(0);
    }
  });

  it("English copy states non-MVP / acceptance scope explicitly", () => {
    const v = (en as { neuro: { vision: Record<string, string> } }).neuro.vision;
    const combined = `${v.badge} ${v.subtitle} ${v.body}`.toLowerCase();
    expect(combined).toMatch(/mvp|acceptance|vision|later|roadmap/);
    expect(combined).not.toMatch(/\bshipped\b|\bgenerally available\b/);
  });

  it("Chinese copy states 愿景 / 验收 / 后续 semantics", () => {
    const v = (zh as { neuro: { vision: Record<string, string> } }).neuro.vision;
    const combined = `${v.badge}${v.subtitle}${v.body}`;
    expect(combined).toMatch(/愿景|后续|验收|MVP/);
  });
});
