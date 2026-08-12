---
status: accepted
---

# Keep public rule trees iterative

Every operation proportional to public rule-tree depth uses an explicit work stack. Native DTO construction, destruction and JSON serialization, JavaScript DTO validation and rule hydration, CSSOM `cssText`, and Reparsable Stylesheet Serialization therefore remain usable through the configured syntax-depth boundary instead of merely converting a call-stack overflow into a controlled error. The shared Rust parser uses bounded explicit work stacks rather than a native-only large-stack thread; no backend-specific recursion, JavaScript recursion or repeated parent reindentation may make a value accepted by the Resource Budget unusable after parsing.
