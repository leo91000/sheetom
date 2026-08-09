import { playwright } from "@vitest/browser-playwright";
import { defineConfig } from "vitest/config";

const supportedBrowsers = ["chromium", "firefox", "webkit"] as const;
const requestedBrowser = process.env.SHEETOM_BROWSER;
if (
  requestedBrowser !== undefined &&
  !supportedBrowsers.some(browser => browser === requestedBrowser)
) {
  throw new Error(`Unsupported SHEETOM_BROWSER: ${requestedBrowser}`);
}
const browserInstances = requestedBrowser
  ? [{ browser: requestedBrowser as (typeof supportedBrowsers)[number] }]
  : process.env.SHEETOM_BROWSER_MATRIX === "1"
    ? supportedBrowsers.map(browser => ({ browser }))
    : [{ browser: "chromium" as const }];

export default defineConfig({
  test: {
    projects: [
      {
        test: {
          name: "unit",
          environment: "node",
          include: ["tests/*.test.ts"],
        },
      },
      {
        test: {
          name: "browser",
          include: ["tests/browser/*.test.ts"],
          browser: {
            enabled: true,
            headless: true,
            provider: playwright(),
            instances: browserInstances,
          },
        },
      },
      {
        test: {
          name: "fuzz",
          environment: "node",
          include: ["tests/fuzz/*.test.ts"],
        },
      },
    ],
  },
});
