---
status: superseded by ADR-0063
---

# Test pinned engines and stable browsers

Pull requests will run applicable Web Platform Tests and the differential corpus against pinned Playwright Chromium, Firefox, and WebKit. Scheduled and release validation will add stable Google Chrome and Mozilla Firefox, with actual Safari through WebDriver on macOS; every selected engine disagreement will be retained as a Divergence Fixture.
