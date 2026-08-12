import { execFileSync } from "node:child_process";

execFileSync("npm", ["exec", "changeset", "version"], { stdio: "inherit" });
execFileSync(process.execPath, ["scripts/sync-cargo-version.ts"], { stdio: "inherit" });
execFileSync(process.execPath, ["scripts/sync-native-packages.ts", "--record"], {
  stdio: "inherit",
});
execFileSync(process.execPath, ["scripts/sync-wasm-package.ts", "--record"], {
  stdio: "inherit",
});
execFileSync(
  "npm",
  ["install", "--package-lock-only", "--ignore-scripts"],
  { stdio: "inherit" },
);
execFileSync(process.execPath, ["scripts/engine-abi.ts", "--record"], { stdio: "inherit" });
