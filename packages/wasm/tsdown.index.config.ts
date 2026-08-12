import { defineConfig } from "tsdown";
import { fileURLToPath } from "node:url";

const activeBinding = fileURLToPath(new URL("./src/active-binding.ts", import.meta.url));

export default defineConfig({
  entry: { index: "src/index.ts" },
  format: ["esm"],
  dts: true,
  sourcemap: true,
  clean: false,
  outDir: "dist",
  platform: "neutral",
  target: "es2022",
  alias: {
    "./default-engine-binding.js": activeBinding,
  },
  deps: {
    neverBundle: [/facade_factory\.js$/, /sheetom_wasm_factory\.js$/],
  },
});
