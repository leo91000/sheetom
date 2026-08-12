import { defineConfig } from "tsdown";

export default defineConfig({
  entry: { index: "src/index.ts" },
  format: ["esm"],
  dts: true,
  sourcemap: true,
  clean: false,
  outDir: "dist",
  platform: "neutral",
  target: "es2022",
  deps: {
    neverBundle: [/binding-registry\.js$/, /facade\.js$/],
  },
});
