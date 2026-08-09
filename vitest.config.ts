import { playwright } from "@vitest/browser-playwright";
import { defineConfig } from "vitest/config";

const browserInstances = process.env.SHEETOM_BROWSER_MATRIX === "1"
  ? [
      { browser: "chromium" as const },
      { browser: "firefox" as const },
      { browser: "webkit" as const },
    ]
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
