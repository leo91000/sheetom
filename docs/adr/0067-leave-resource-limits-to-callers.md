# Leave resource limits to callers

The browser-shaped interface will not impose nonstandard stylesheet-size, nesting-depth, or mutation-count limits. Documentation will identify resource-exhaustion risk for untrusted input, callers may isolate or bound their workloads, and project fuzzing and performance gates protect the documented Reference Workload.
