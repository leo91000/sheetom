# Use a pull-based diagnostic queue

Consumers may opt each sheet into collecting Mutation Diagnostics and drain them through `takeDiagnostics`. Diagnostics will not invoke user code during CSSOM operations, so reporting cannot alter browser-compatible return values, exceptions, state transitions, or timing.
