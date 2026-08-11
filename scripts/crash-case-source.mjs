export function materializeCrashSource(crashCase) {
  if (crashCase.mode === "dynamic-range-depth") {
    return `dynamic-range-limit: ${"dynamic-range-limit-mix(".repeat(crashCase.nestingDepth)}standard${" 100%)".repeat(crashCase.nestingDepth)}`;
  }
  if (crashCase.mode === "geometric-svg-path") {
    return `d: path("M0 0 ${"L1 1 ".repeat(crashCase.repeatCount)}")`;
  }
  if (crashCase.mode === "geometric-shape-commands") {
    return `shape-outside: shape(from 0 0, ${"line to 1px 1px, ".repeat(crashCase.repeatCount)}close)`;
  }
  if (crashCase.mode === "geometric-polygon-points") {
    return `shape-outside: polygon(${"1px 1px, ".repeat(crashCase.repeatCount)}0 0)`;
  }
  if (crashCase.mode === "geometric-nested-gradient") {
    return `shape-outside: linear-gradient(red, ${"color-mix(in srgb, red, ".repeat(crashCase.nestingDepth)}blue${")".repeat(crashCase.nestingDepth)})`;
  }
  if (crashCase.mode === "geometric-oversized") {
    return `d: path("M0 0 ${"L1 1 ".repeat(210_000)}")`;
  }
  if (crashCase.oversized) return "x".repeat((1024 * 1024) + 1);
  if (crashCase.nestingDepth) {
    return `--x: ${"fn(".repeat(crashCase.nestingDepth)}value`;
  }
  if (crashCase.declarationCount) return "x:;".repeat(crashCase.declarationCount);
  return crashCase.source;
}
