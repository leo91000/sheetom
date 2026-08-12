import { readFile } from "node:fs/promises";
import { fileURLToPath } from "node:url";

export const soakContextPrefix = "sheetom/stable-soak/";

function utcDate(date) {
  return date.toISOString().slice(0, 10);
}

function previousUtcDate(date, days) {
  const copy = new Date(`${date}T00:00:00.000Z`);
  copy.setUTCDate(copy.getUTCDate() - days);
  return utcDate(copy);
}

export function verifyConsecutiveSoakStatuses(
  statuses,
  { requiredRuns = 7, now = new Date() } = {},
) {
  const latestByContext = new Map();
  const ordered = [...statuses].sort(
    (left, right) => Date.parse(right.created_at) - Date.parse(left.created_at),
  );
  for (const status of ordered) {
    if (!status.context?.startsWith(soakContextPrefix)) continue;
    if (!latestByContext.has(status.context)) latestByContext.set(status.context, status);
  }

  const successfulDates = [...latestByContext.values()]
    .filter(status => status.state === "success")
    .map(status => status.context.slice(soakContextPrefix.length))
    .filter(date => /^\d{4}-\d{2}-\d{2}$/u.test(date))
    .sort();
  const latestDate = successfulDates.at(-1);
  if (!latestDate) throw new Error("The first stable release has no successful scheduled soak run");

  const ageInDays = Math.floor(
    (Date.parse(`${utcDate(now)}T00:00:00.000Z`) - Date.parse(`${latestDate}T00:00:00.000Z`))
      / 86_400_000,
  );
  if (ageInDays < 0 || ageInDays > 1) {
    throw new Error(`Latest successful first-stable soak run is stale: ${latestDate}`);
  }

  const successfulDateSet = new Set(successfulDates);
  const requiredDates = Array.from(
    { length: requiredRuns },
    (_, index) => previousUtcDate(latestDate, requiredRuns - index - 1),
  );
  const missingDates = requiredDates.filter(date => !successfulDateSet.has(date));
  if (missingDates.length > 0) {
    throw new Error(`First-stable soak is missing consecutive dates: ${missingDates.join(", ")}`);
  }
  return requiredDates;
}

async function main() {
  const statusesArgument = process.argv.find(argument => argument.startsWith("--statuses="));
  if (!statusesArgument) throw new Error("Usage: verify-release-soak.mjs --statuses=path");
  const statuses = JSON.parse(
    await readFile(statusesArgument.slice("--statuses=".length), "utf8"),
  );
  const dates = verifyConsecutiveSoakStatuses(statuses);
  console.log(`Verified seven consecutive first-stable soak runs: ${dates.join(", ")}`);
}

if (process.argv[1] === fileURLToPath(import.meta.url)) await main();
