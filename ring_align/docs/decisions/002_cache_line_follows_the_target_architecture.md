# `CACHE_LINE` follows the target architecture, using `crossbeam-utils`' table

Status: Accepted. Supersedes [ADR 001](001_cache_line_is_one_unconditional_constant.md).

## Context

ADR 001 fixed `ring_align::CACHE_LINE` at an unconditional 64, to be raised by hand at port time. It named the first
benchmark on Apple Silicon as its revisit trigger.

That benchmark ran on an Apple M4 Pro, whose lines are 128 bytes. There the 64-byte padding separated nothing:
whether two cursors shared a line depended on where the ring landed in memory.

- Batched `ring_mpsc` at 8 producers ran at 91–95 or 137–143 M/s on placement alone.
- `ring_spsc` at 1024 slots collapsed to about 33 M/s in 3 of 16 paired runs.

Both are in [docs/benchmarks/001_cache_line_128.md](../benchmarks/001_cache_line_128.md).

A build can see the target architecture and its `target_feature` set, not the processor. With `-C target-cpu`
set to `apple-m4` and then to `neoverse-n1`, `rustc --print cfg` for AArch64 differs only in `target_feature` values,
and Rust has no `target_cpu` cfg. Line size varies within one architecture: Apple Silicon uses 128, AWS Graviton 64,
both AArch64.

`#[repr(align(..))]` accepts only an integer literal (E0693).

## Decision

`CACHE_LINE` and `CacheAligned`'s alignment follow the table in `crossbeam-utils`' `CachePadded` (0.8.23):

| Architecture | Bytes |
|---|---|
| x86-64, AArch64, arm64ec, powerpc64 | 128 |
| arm, mips, mips32r6, mips64, mips64r6, sparc, hexagon | 32 |
| m68k | 16 |
| s390x | 256 |
| every other | 64 |

Where an architecture has both 64- and 128-byte parts, the table takes 128, because under-padding is the error that
costs. On x86-64 the 128 is not pure padding either: the spatial prefetcher on Intel cores since Sandy Bridge fetches
lines in adjacent pairs.

The table is written twice, as a `cfg!` chain for the constant and a `cfg_attr` chain for the `repr`. Three checks
hold them together:

- A `const` assertion in `src/lib.rs` makes a mismatch a compile error on the target being built. So
  `cargo check --target` covers architectures no test run reaches. When this record was written, it compiled for
  `aarch64-apple-darwin`, `x86_64-unknown-linux-gnu`, `armv7-unknown-linux-gnueabi`, `riscv64gc-unknown-linux-gnu`
  and `s390x-unknown-linux-gnu`, and it failed when the 32-byte row was deliberately mismatched.
- `a_wrapped_value_occupies_exactly_one_line` checks the same agreement at runtime.
- `cache_line_follows_the_architecture_table` restates the table from `std::env::consts::ARCH`, so a wrong row fails
  on the machine running it.

## Alternatives considered

- **One unconditional 64 (ADR 001).** It under-pads Apple Silicon, which is the measured failure.
- **One unconditional 128, the step taken just before this record.** It is right on every target the family runs on.
  But it under-pads s390x (256), over-pads 32-byte targets, and picks by hand what a maintained table already encodes.
- **`cfg(target_vendor = "apple")` selecting 128, with 64 for other AArch64.** It would shrink padding on Graviton. But
  it encodes "Apple's chips have 128-byte lines" as a build fact nobody guarantees, and it under-pads the next 128-byte
  AArch64 part.
- **A build-time override for a known 64-byte machine,** such as `RUSTFLAGS='--cfg ring_cache_line="64"'`. Not taken.
  It saves about 64 bytes per padded cursor, with no measured speed benefit, and a wrong value silently under-pads.
- **A Cargo feature.** It changes a type's layout, so it is not additive. ADR 001's argument holds.
- **A build script that probes the line size.** It measures the build machine, not the target, which breaks
  cross-compilation and CI-built binaries.
- **Querying the line size at runtime.** That cannot drive `repr(align)`.

## Consequences

- On this repo's targets (x86-64 hosts and Apple Silicon) the value is 128, the same as the unconditional raise that
  preceded this record. The M4 measurements apply unchanged.
- 64-byte-line AArch64 and x86-64 parts get twice the padding they need. A `PaddedCursor` is 128 bytes there instead
  of 64, and a ring holds a handful of them.
- Tests that pin a byte size by literal (`ring_claim`'s and `ring_gating`'s) run only on x86-64 and AArch64. The rest
  compare against `CACHE_LINE`.
- The architecture predicates appear twice, in the constant and in the `cfg_attr`s, so a new row is two edits.
- Revisit when a benchmark on a 64-byte-line host shows that the doubled padding costs measurable cache footprint.
  That reopens the override alternative with data.
- Revisit when `crossbeam-utils` changes its table.
