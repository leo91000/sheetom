import { execFileSync } from "node:child_process";

execFileSync("npm", ["exec", "changeset", "version"], { stdio: "inherit" });
execFileSync(process.execPath, ["scripts/sync-cargo-version.mjs"], { stdio: "inherit" });
execFileSync(process.execPath, ["scripts/sync-native-packages.mjs", "--record"], {
  stdio: "inherit",
});
execFileSync(
  "npm",
  ["install", "--package-lock-only", "--ignore-scripts"],
  { stdio: "inherit" },
);
execFileSync(process.execPath, ["scripts/engine-abi.mjs", "--record"], { stdio: "inherit" });
