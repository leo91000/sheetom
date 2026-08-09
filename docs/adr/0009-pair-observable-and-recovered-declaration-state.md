# Pair observable and recovered declaration state

Each Declaration Record will atomically store browser-facing value text alongside its recovered token or typed representation, priority, and declaration order. Keeping both views in one record supports Chromium-compatible getters and safe output without the synchronization risk of parallel raw and semantic stylesheets.
