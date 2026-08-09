# Block pull requests on the platform matrix

Every pull request will install and exercise the packed package under Node 22 and 24 on Linux x64, Windows x64, and macOS arm64, with failures in any combination blocking merge. The young project prefers early native-package compatibility evidence over moving non-Linux failures to a later release gate.
