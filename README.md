# Embedded-perfmon

An opinionated tool to gather, transport and analyze embassy trace data.

Currently depends on this pr: https://github.com/embassy-rs/embassy/pull/6290

Parts of the project:
- `embedded-perfmon-runtime`: The main crate the firmware interacts with.
  - defines the required global functions to gather the trace
  - exposes some functions for the firmware to call
  - defines some functions the firmware needs to implement to send out the trace data
- `embedded-perfmon-transport`: Crate containing the event types and de/serialization functions
- `embedded-perfmon-analyzer`: Lib and CLI crate for translating the transport byte stream into easy to process data/json

## Guide for firmware

Include the `runtime` as a dependency.
```rust
use embedded_perfmon_runtime as _;
```

Enable the trace feature on `embassy-executor` & `embassy-mcxa` to have both crates generate trace events.

Implement the runtime functions for writing trace events and getting time data. If these functions are not implemented, you'll get a linker error. Check out the RTT example to see how you could do that. (Hint: You'll need `#[unsafe(no_mangle)]`)

Check out the runtime docs to see what tracing functions you can or should call during your application.

By default the tracing will always run. If you want to be able to start and stop the tracing, then use the `start-stop` cargo feature of the runtime.
This adds the required caching so all info that needs to be captured will be present and emitted when start is called.

## Guide for analyzing

Collect the trace byte stream from your device. Then use the analyzer lib or cli to get processed values of out of it.

You can get a json schema for the output data by running the cli with the `schema` command.
