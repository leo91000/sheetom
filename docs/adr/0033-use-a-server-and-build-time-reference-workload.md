# Use a server and build-time reference workload

Performance decisions will use a Reference Workload of stylesheets up to roughly one megabyte or ten thousand rules with mutation bursts around ten thousand operations across Node, Bun, and Deno. A Rust escalation requires profiling evidence that this workload is blocked by parsing or the JavaScript state model; hypothetical native speedups are insufficient.
