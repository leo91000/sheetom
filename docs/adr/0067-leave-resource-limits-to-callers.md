# Leave resource limits to callers

The browser-shaped interface will not impose nonstandard stylesheet-size, nesting-depth, mutation-count, or diagnostic-input limits. Documentation will identify resource-exhaustion risk for untrusted input, opt-in diagnostics retain their complete input until drained, callers may isolate or bound their workloads, and project fuzzing and performance gates protect the documented Reference Workload.
