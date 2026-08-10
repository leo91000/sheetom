import { defineConfig } from "tsdown";

export default defineConfig({
  entry: ["src/index.ts"],
  format: ["esm", "cjs"],
  dts: true,
  sourcemap: true,
  clean: true,
  platform: "neutral",
  // The public API loads a Node-API binding. Keep the historical .js ESM
  // output while making the Node-compatible loader boundary explicit.
  deps: {
    neverBundle: [/^node:/],
  },
});
