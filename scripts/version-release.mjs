import { execFileSync } from "node:child_process";

execFileSync("npm", ["exec", "changeset", "version"], { stdio: "inherit" });
execFileSync(
  "npm",
  ["install", "--package-lock-only", "--ignore-scripts"],
  { stdio: "inherit" },
);
