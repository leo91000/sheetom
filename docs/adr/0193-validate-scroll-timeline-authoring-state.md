---
status: accepted
---

# Validate scroll timeline authoring state

WebDX 3.45.0 makes scroll-driven animation branches individually Baseline.
Audit their CSS declarations through authored CSSOM operations; timeline
execution, source/subject DOM associations and computed offsets remain outside
SheetOM's contract. The existing Chromium 151 oracle implements this syntax.

The scroll-timeline shorthand grammar requires a name followed by an optional
axis in every comma-separated entry. Enforce that order in the shared core
parser, including support queries and all declaration mutation entry points.
Reject an invalid later entry atomically. The unordered arguments of scroll()
do not change the shorthand's ordered grammar.

Chromium and published SheetOM both reorder reset-only animation longhands when
an animation shorthand replaces existing range/timeline declarations and is
then serialized and reparsed. For this measured sequence, compare the complete
name/value/priority set, explicit reset values, rule topology and stable
serialization. Individual declarations and initial shorthand round trips retain
exact-order assertions through the existing roundtrip helper. This does not
permit dropped declarations or a general relaxation of declaration ordering.

The [dated audit](../css-baseline-audit-2026-11-09.md) records the sources and
browser observation. The shared November contracts run unchanged against native,
WASM and cold installed packages; browser probes independently compare state.
