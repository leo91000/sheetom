import { execFileSync } from "node:child_process";
import { readdir } from "node:fs/promises";

const base = process.env.SHEETOM_CHANGESET_BASE;
if (!base) {
  throw new Error("SHEETOM_CHANGESET_BASE must name the pull request base commit");
}

const changedFiles = execFileSync(
  "git",
  ["diff", "--name-only", `${base}...HEAD`],
  { encoding: "utf8" },
).trim().split("\n").filter(Boolean);

const impactful = changedFiles.some(file =>
  file === "package.json" ||
  file === "package-lock.json" ||
  file.startsWith("src/") ||
  file.startsWith("compatibility/baselines/"),
);

if (!impactful) {
  console.log("No consumer-impacting files changed; a Changeset is not required.");
  process.exit(0);
}

if (process.env.SHEETOM_NO_CHANGESET === "1") {
  console.log("The no-changeset pull request label explicitly classifies this change.");
  process.exit(0);
}

const entries = await readdir(".changeset");
const changesets = entries.filter(entry => entry.endsWith(".md") && entry !== "README.md");
if (changesets.length === 0) {
  throw new Error(
    "Consumer-impacting changes require a Changeset or the no-changeset pull request label",
  );
}

console.log(`Found ${changesets.length} Changeset entr${changesets.length === 1 ? "y" : "ies"}.`);
