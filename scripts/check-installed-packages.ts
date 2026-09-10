import { access, mkdir, writeFile } from "node:fs/promises";
import { spawnSync } from "node:child_process";
import path from "node:path";
import { fileURLToPath } from "node:url";
import { backendEntry } from "./validation-backend.ts";

const option = (name: string) => process.argv.find(argument => argument.startsWith(name + "="))?.slice(name.length + 1);
const packageRoot = option("--package-root");
const reportDirectory = option("--report-dir");
if (!packageRoot || !reportDirectory) throw new Error("Usage: node scripts/check-installed-packages.ts --package-root=<fresh npm prefix> --report-dir=<evidence directory>");
const prefix = path.resolve(packageRoot);
const reports = path.resolve(reportDirectory);
for (const backend of ["native", "wasm"] as const) await access(backendEntry(backend, prefix));
// Resolve development tools here, in the repository that owns the test sources.
for (const dependency of ["esbuild", "playwright"]) import.meta.resolve(dependency);
await mkdir(reports, { recursive: true });
const checks = [
  ["modern-contracts", "test-modern-css-backends.ts", []],
  ["browser-authoring", "check-css-authoring-target.ts", ["--report=" + path.join(reports, "css-authoring-target-report.json")]],
  ["native-webref", "check-webref-property-branches.ts", ["--report=" + path.join(reports, "native-webref.json")]],
  ["wasm-webref", "check-webref-property-branches.ts", ["--wasm", "--report=" + path.join(reports, "wasm-webref.json")]],
] as const;
for (const [name, filename, args] of checks) {
  console.log("Running " + name);
  const result = spawnSync(process.execPath, [fileURLToPath(new URL(filename, import.meta.url)), ...args], {
    cwd: fileURLToPath(new URL("../", import.meta.url)),
    env: { ...process.env, SHEETOM_PACKAGE_ROOT: prefix },
    encoding: "utf8", timeout: 600_000, maxBuffer: 16 * 1024 * 1024,
  });
  const output = (result.stdout ?? "") + (result.stderr ?? "");
  await writeFile(path.join(reports, name + ".log"), output);
  if (result.error || result.signal || result.status !== 0) throw new Error(name + " failed: " + (result.error?.message ?? result.signal ?? result.status) + "\n" + output.slice(-8000));
  process.stdout.write(output);
}
console.log("Installed native/WASM validation passed. Evidence: " + reports);
