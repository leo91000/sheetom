import assert from "node:assert/strict";
import test from "node:test";

import {
  soakContextPrefix,
  verifyConsecutiveSoakStatuses,
} from "./verify-rc6-soak.mjs";

function status(date, state = "success", createdAt = `${date}T03:00:00Z`) {
  return {
    context: `${soakContextPrefix}${date}`,
    state,
    created_at: createdAt,
  };
}

test("the RC6 gate accepts seven fresh consecutive scheduled runs", () => {
  const dates = [
    "2026-08-04",
    "2026-08-05",
    "2026-08-06",
    "2026-08-07",
    "2026-08-08",
    "2026-08-09",
    "2026-08-10",
  ];
  assert.deepEqual(
    verifyConsecutiveSoakStatuses(dates.map(date => status(date)), {
      now: new Date("2026-08-10T12:00:00Z"),
    }),
    dates,
  );
});

test("the RC6 gate rejects gaps, stale evidence, and a latest failed rerun", () => {
  const datesWithGap = [
    "2026-08-03",
    "2026-08-04",
    "2026-08-05",
    "2026-08-07",
    "2026-08-08",
    "2026-08-09",
    "2026-08-10",
  ];
  assert.throws(
    () => verifyConsecutiveSoakStatuses(datesWithGap.map(date => status(date)), {
      now: new Date("2026-08-10T12:00:00Z"),
    }),
    /2026-08-06/u,
  );
  assert.throws(
    () => verifyConsecutiveSoakStatuses(datesWithGap.map(date => status(date)), {
      now: new Date("2026-08-12T12:00:00Z"),
    }),
    /stale/u,
  );

  const failedRerun = [
    ...datesWithGap.filter(date => date !== "2026-08-03").map(date => status(date)),
    status("2026-08-06"),
    status("2026-08-10", "failure", "2026-08-10T04:00:00Z"),
  ];
  assert.throws(
    () => verifyConsecutiveSoakStatuses(failedRerun, {
      now: new Date("2026-08-10T12:00:00Z"),
    }),
    /stale|missing/u,
  );
});
