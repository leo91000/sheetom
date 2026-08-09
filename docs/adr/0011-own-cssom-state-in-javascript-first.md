# Own CSSOM state in JavaScript first

The first implementation will own live CSSOM objects, Declaration Records, ordering, validation, and safe serialization in TypeScript while reusing the existing `lightningcss` npm native addon for parsing and recovered AST data. A custom Rust/N-API addon will be introduced only if differential tests demonstrate lost semantic information, an unavoidable parser or printer limitation, or a measured performance bottleneck; this avoids bundling and distributing a second native binary before it provides proven value.
