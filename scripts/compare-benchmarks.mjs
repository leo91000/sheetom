import { readFile } from "node:fs/promises";

const [baselinePath, candidatePath] = process.argv.slice(2);
if (!baselinePath || !candidatePath) {
  throw new Error("Usage: compare-benchmarks.mjs <baseline.json> <candidate.json>");
}

const baseline = JSON.parse(await readFile(baselinePath, "utf8"));
const candidate = JSON.parse(await readFile(candidatePath, "utf8"));
const relativeLimit = 1.25;
const workloadLimits = {
  stress: {
    parseMilliseconds: 5_000,
    mutationMilliseconds: 2_000,
    serializationMilliseconds: 1_000,
    rssDeltaBytes: 512 * 1024 * 1024,
  },
  publisher: {
    totalMilliseconds: 15_000,
    parseMilliseconds: 10_000,
    mutationMilliseconds: 5_000,
    serializationMilliseconds: 2_000,
    secondSerializationMilliseconds: 2_000,
    rssDeltaBytes: 768 * 1024 * 1024,
  },
};
const failures = [];

function compareWorkload(name, baselineResults, candidateResults, limits) {
  if (!baselineResults || !candidateResults) {
    throw new Error(`Missing benchmark workload: ${name}`);
  }
  for (const [metric, absoluteLimit] of Object.entries(limits)) {
    const baselineValue = baselineResults[metric];
    const candidateValue = candidateResults[metric];
    if (typeof baselineValue !== "number" || typeof candidateValue !== "number") {
      throw new Error(`Missing ${name} benchmark metric: ${metric}`);
    }

    const relativeRegression = candidateValue > baselineValue * relativeLimit;
    const absoluteRegression = candidateValue > absoluteLimit;
    if (!relativeRegression || !absoluteRegression) continue;
    failures.push(
      `${name}.${metric}: ${candidateValue.toFixed(2)} exceeds base ${baselineValue.toFixed(2)} by more than 25% and absolute limit ${absoluteLimit}`,
    );
  }
}

compareWorkload("stress", baseline.results, candidate.results, workloadLimits.stress);
compareWorkload(
  "publisher",
  baseline.publisher?.results,
  candidate.publisher?.results,
  workloadLimits.publisher,
);

console.log(JSON.stringify({
  baseline: { stress: baseline.results, publisher: baseline.publisher.results },
  candidate: { stress: candidate.results, publisher: candidate.publisher.results },
}, null, 2));
if (failures.length > 0) throw new Error(failures.join("\n"));
