# Keep browser capability probes out of runtime

SheetOM's runtime remains synchronous, deterministic, and offline: it consumes checked-in manifests, validators, and compatibility data without launching or contacting a browser. Pinned browser engines are used only by explicit generation, differential CI, and release-baseline workflows, and every reproducible fuzz counterexample is minimized into the permanent Operation Fixture corpus under ADR 0046.
