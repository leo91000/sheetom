import { defineConfig } from "tsdown";

export default defineConfig({
  entry: { "binding-registry": "src/binding-registry.ts" },
  format: ["esm"],
  dts: false,
  sourcemap: true,
  clean: false,
  outDir: "dist",
  platform: "neutral",
  target: "es2022",
});
