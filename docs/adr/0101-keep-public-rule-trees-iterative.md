---
status: accepted
---

# Keep public rule trees iterative

Every operation proportional to public rule-tree depth uses an explicit work stack. Native DTO validation, JavaScript rule hydration, CSSOM `cssText`, and Reparsable Stylesheet Serialization therefore remain usable through the configured syntax-depth boundary instead of merely converting a call-stack overflow into a controlled error. Parser recursion stays private to the bounded native parser thread; no JavaScript recursion or repeated parent reindentation may make a value accepted by the Resource Budget unusable after parsing.
