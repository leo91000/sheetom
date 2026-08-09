# Publish a machine-readable compatibility report

Each release will commit a schema-validated `compatibility/baselines/<version>.json` containing its WPT pin, dependency and browser versions, dispositions, oracle observations, resolutions, and summary counts. The current report ships in the npm package and generates human-readable release evidence, while prior reports remain immutable.
