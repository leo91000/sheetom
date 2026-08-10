export const expectedNativeArtifacts = Object.freeze([
  "sheetom-native.darwin-arm64.node",
  "sheetom-native.darwin-x64.node",
  "sheetom-native.linux-arm64-gnu.node",
  "sheetom-native.linux-arm64-musl.node",
  "sheetom-native.linux-x64-gnu.node",
  "sheetom-native.linux-x64-musl.node",
  "sheetom-native.win32-arm64-msvc.node",
  "sheetom-native.win32-x64-msvc.node",
]);

export function assertCompleteNativeArtifactNames(names) {
  const actual = [...names].sort();
  const expected = [...expectedNativeArtifacts].sort();
  if (JSON.stringify(actual) === JSON.stringify(expected)) return;
  throw new Error(
    `Native artifact set is incomplete: expected ${expected.join(", ")}; ` +
      `received ${actual.join(", ")}`,
  );
}

export function assertCompleteNativeTarballEntries(entries) {
  const names = entries
    .filter(entry => /^package\/native\/[^/]+\.node$/u.test(entry))
    .map(entry => entry.slice("package/native/".length));
  assertCompleteNativeArtifactNames(names);
}
