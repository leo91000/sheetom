# Split fast browser differentials from native oracle runs

Pull requests will execute Operation Fixtures through SheetOM and pinned Playwright Chromium, Firefox, and WebKit adapters. Nightly and release gates add full selected native WPT plus stable Chrome and Firefox, while releases additionally require actual Safari on macOS; this preserves fast differential coverage without placing every native-oracle run on each pull request.
