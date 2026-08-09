import { describe, expect, it } from "vitest";
import {
  extractReleaseNotes,
  npmTagForVersion,
  parsePackResult,
  waitForDistTag,
} from "../scripts/publish-release.mjs";

describe("release automation", () => {
  it("routes prereleases to next and stable releases to latest", () => {
    expect(npmTagForVersion("0.1.0-rc.1")).toBe("next");
    expect(npmTagForVersion("0.1.0")).toBe("latest");
  });

  it("extracts only the requested changelog section", () => {
    const changelog = [
      "# sheetom",
      "",
      "## 0.2.0",
      "",
      "New release.",
      "",
      "## 0.1.0",
      "",
      "Old release.",
    ].join("\n");
    expect(extractReleaseNotes(changelog, "0.2.0")).toBe("New release.");
    expect(extractReleaseNotes(changelog, "9.0.0")).toBe("Release 9.0.0.");
  });

  it("requires npm pack to return exactly one artifact", () => {
    expect(parsePackResult('[{"filename":"sheetom-0.1.0.tgz"}]')).toEqual({
      filename: "sheetom-0.1.0.tgz",
    });
    expect(() => parsePackResult("[]")).toThrow(/exactly one/);
  });

  it("waits for an npm dist-tag to propagate", async () => {
    let reads = 0;
    const distTags = await waitForDistTag("sheetom", "next", "0.1.0-rc.1", {
      attempts: 3,
      intervalMs: 0,
      readTags: async () => {
        reads += 1;
        return { next: reads === 1 ? "0.1.0-rc.0" : "0.1.0-rc.1" };
      },
      wait: async () => {},
    });

    expect(reads).toBe(2);
    expect(distTags.next).toBe("0.1.0-rc.1");
  });

  it("bounds npm dist-tag propagation retries", async () => {
    let waits = 0;
    const result = waitForDistTag("sheetom", "next", "0.1.0-rc.1", {
      attempts: 3,
      intervalMs: 0,
      readTags: async () => ({ next: "0.1.0-rc.0" }),
      wait: async () => {
        waits += 1;
      },
    });

    await expect(result).rejects.toThrow(
      "npm dist-tag next did not point to 0.1.0-rc.1 after 3 attempts; " +
        "last observed 0.1.0-rc.0",
    );
    expect(waits).toBe(2);
  });
});
