# Gate material performance regressions

Performance jobs will compare the branch and main on the same pinned runner and publish their Reference Workload results, but block only when a change exceeds both a calibrated absolute resource ceiling and a material relative regression. Final ceilings will be calibrated after the internal module rewrite, while every release must run the complete workload.
