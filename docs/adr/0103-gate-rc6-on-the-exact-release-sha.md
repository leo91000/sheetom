---
status: accepted
---

# Gate RC6 on the exact release SHA

RC6 publication requires seven successful scheduled full-CI runs on seven consecutive UTC dates for one unchanged Changesets release pull request SHA. The daily orchestrator dispatches the existing complete CI matrix and records a dated commit status only after that run succeeds; manual runs use a separate context and cannot satisfy the gate. Any release-PR update changes the SHA and resets the evidence. After merge, the Release workflow resolves the merged pull request and verifies the seven dated statuses before npm publication.
