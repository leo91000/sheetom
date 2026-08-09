import { describe, expect, it } from "vitest";
import {
  extractReleaseNotes,
  npmTagForVersion,
  parsePackResult,
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
});
