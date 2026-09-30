# Security

## Reporting

Report privately through GitHub: **Security → Report a vulnerability** on
[Wandalen/ring](https://github.com/Wandalen/ring/security). If that button is missing, open an
issue saying only that you have a report and need a private channel — no details in public.

Include the crate, the operation, a reproduction (test, loom model or command), and
`rustc -V`. Expect an acknowledgement within a week.

## In scope

- Undefined behaviour reachable from safe code — in practice `ring_spsc` and `ring_mpsc`, the
  only crates with `unsafe` (see [`unsafe_allowlist.txt`](bench_harness/gate/declared/ring/unsafe_allowlist.txt)).
- Data races: a record observed before its write, torn or stale slot contents, a producer
  lapping the consumer.
- Broken delivery guarantees: a published record lost, duplicated or reordered, or reported
  as kept when it was not — beyond the documented `DropNewest` behaviour.
- A hang or unbounded spin reachable through the public API of the export crates.

Out of scope: `ring_bench`, `ring_testkit`, `ring_debug`, `ring_trace` and `bench_harness`
(development tools, `publish = false`), and misuse the docs already call out as a caller
error.

## Supported versions

Nothing has been released yet. Fixes land on `master`.

## Dependencies

Three direct external dependencies: `crossbeam-queue` (optional), `loom` (cfg-gated) and
`trybuild` (dev only). `./verb/test level::4` runs `cargo audit` when a `Cargo.lock` is present.
