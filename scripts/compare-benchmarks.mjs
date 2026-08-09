import { readFile } from "node:fs/promises";

const [baselinePath, candidatePath] = process.argv.slice(2);
if (!baselinePath || !candidatePath) {
  throw new Error("Usage: compare-benchmarks.mjs <baseline.json> <candidate.json>");
}

const baseline = JSON.parse(await readFile(baselinePath, "utf8"));
const candidate = JSON.parse(await readFile(candidatePath, "utf8"));
const relativeLimit = 1.25;
const absoluteLimits = {
  parseMilliseconds: 5_000,
  mutationMilliseconds: 2_000,
  serializationMilliseconds: 1_000,
  rssDeltaBytes: 512 * 1024 * 1024,
};
const failures = [];

for (const [metric, absoluteLimit] of Object.entries(absoluteLimits)) {
  const baselineValue = baseline.results[metric];
  const candidateValue = candidate.results[metric];
  if (typeof baselineValue !== "number" || typeof candidateValue !== "number") {
    throw new Error(`Missing benchmark metric: ${metric}`);
  }

  const relativeRegression = candidateValue > baselineValue * relativeLimit;
  const absoluteRegression = candidateValue > absoluteLimit;
  if (relativeRegression && absoluteRegression) {
    failures.push(
      `${metric}: ${candidateValue.toFixed(2)} exceeds base ${baselineValue.toFixed(2)} by more than 25% and absolute limit ${absoluteLimit}`,
    );
  }
}

console.log(JSON.stringify({ baseline: baseline.results, candidate: candidate.results }, null, 2));
if (failures.length > 0) throw new Error(failures.join("\n"));
