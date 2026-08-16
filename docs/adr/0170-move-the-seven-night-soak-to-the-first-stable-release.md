---
status: superseded by ADR-0186
---

# Move the seven-night soak to the first stable release

RC6 may publish after its unchanged release pull request passes the complete CI matrix, native Chrome and Firefox oracles, immutable compatibility recording, process-safety subprocess suite, fuzzing, performance comparison, and all supported runtime and native-package consumers. Its Compatibility Report must contain zero unexplained outcomes and zero mismatches in the versioned Chromium property/value and Webref branch gates.

The seven-consecutive-night full-matrix soak remains mandatory for `0.1.0`, the first stable release. The scheduled workflow records evidence on one unchanged stable release pull request SHA; manual runs remain diagnostic and do not count. Moving the calendar-duration gate acknowledges that RC6 is explicitly a prerelease while preserving the longer observation window before SheetOM makes its first stable compatibility promise.
