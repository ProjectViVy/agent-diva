import { test, expect } from "@playwright/test";

test.describe("agent-diva-gui 浏览器模式（无 Tauri）", () => {
  test("主界面与欢迎语可见", async ({ page }) => {
    await page.goto("/");
    await expect(page.getByTestId("user-visible-app-root")).toBeVisible();
    await expect(page.getByText(/DIVA/)).toBeVisible();
  });

  test("发送消息后显示模拟助手回复", async ({ page }) => {
    await page.goto("/");
    const composer = page.getByTestId("person-main-composer");
    await expect(composer).toBeVisible();

    const input = composer.getByRole("textbox");
    await input.fill("e2e hello");
    await composer.getByRole("button").filter({ has: page.locator("svg.lucide-send") }).click();

    await expect(page.getByText("e2e hello").first()).toBeVisible();
    await expect(page.getByText(/\[模拟回复\]/)).toBeVisible({ timeout: 15_000 });
    await expect(page.getByText(/我收到了:/)).toBeVisible();
  });

  test("生成过程中可停止（浏览器 mock 路径）", async ({ page }) => {
    await page.goto("/");
    const composer = page.getByTestId("person-main-composer");
    const input = composer.getByRole("textbox");
    await input.fill("stop flow");
    await composer.getByRole("button").filter({ has: page.locator("svg.lucide-send") }).click();

    const stopBtn = composer.getByTitle("停止生成");
    await expect(stopBtn).toBeEnabled({ timeout: 5_000 });
    await stopBtn.click();

    await expect(page.getByText(/\[Mock\]/)).toBeVisible();
    await expect(page.getByText(/已停止当前生成/)).toBeVisible();
  });
});
