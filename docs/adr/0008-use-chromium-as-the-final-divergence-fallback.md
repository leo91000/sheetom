# Use Chromium as the final divergence fallback

Compatibility decisions will follow normative specifications and Web Platform Tests first, then behavior shared by pinned Chromium, Firefox, and WebKit builds. When the platform remains ambiguous and engines disagree, the browser-facing API will follow Chromium and capture that choice in a differential fixture rather than add runtime engine profiles.
