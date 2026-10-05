# `CACHE_LINE` follows the build target: `crossbeam-utils`' table, with 64 for non-Apple AArch64

Status: Accepted. Supersedes [ADR 001](001_cache_line_is_one_unconditional_constant.md).

## Context

ADR 001 fixed `ring_align::CACHE_LINE` at an unconditional 64, to be raised by hand at port time. It named the first
benchmark on Apple Silicon as its revisit trigger.

That benchmark ran on an Apple M4 Pro, whose lines are 128 bytes. There the 64-byte padding separated nothing:
whether two cursors shared a line depended on where the ring landed in memory.

- Batched `ring_mpsc` at 8 producers ran at 91–95 or 137–143 M/s on placement alone.
- `ring_spsc` at 1024 slots collapsed to about 33 M/s in 3 of 16 paired runs.

Both are in [docs/benchmarks/001_cache_line_128.md](../benchmarks/001_cache_line_128.md).

A build can see the target architecture, its vendor and its `target_feature` set. It cannot see the processor. With
`-C target-cpu` set to `apple-m4` and then to `neoverse-n1`, `rustc --print cfg` for AArch64 differs only in
`target_feature` values, and Rust has no `target_cpu` cfg.

Line size varies within AArch64: Apple Silicon uses 128, Arm's own cores 64 (Neoverse in AWS Graviton and Ampere,
Cortex-A76 in the Raspberry Pi 5). `ring_align`'s manual check of 2026-08-28 recorded 64 on the workspace host, which
the test files describe as an ARM Neoverse-N1.

`#[repr(align(..))]` accepts only an integer literal (E0693).

## Decision

`CACHE_LINE` and `CacheAligned`'s alignment follow `crossbeam-utils`' `CachePadded` table (0.8.23), with one change:
AArch64 takes 128 only when `target_vendor = "apple"`.

| Target | Bytes |
|---|---|
| x86-64, powerpc64, and AArch64 with `target_vendor = "apple"` | 128 |
| arm, mips, mips32r6, mips64, mips64r6, sparc, hexagon | 32 |
| m68k | 16 |
| s390x | 256 |
| every other, including non-Apple AArch64 and arm64ec | 64 |

On x86-64 the line is 64 bytes, but the spatial prefetcher on Intel cores since Sandy Bridge fetches lines in adjacent
pairs, so that row keeps `crossbeam-utils`' 128.

The vendor stands in for "Apple Silicon". Apple's targets (`aarch64-apple-darwin`, `aarch64-apple-ios`) only ever run
on Apple's cores. Every other AArch64 target gets the 64-byte line that Arm's own cores use.

The known miss is accepted: Linux on Apple Silicon builds as `aarch64-unknown-linux-gnu` and gets 64 on a 128-byte
chip.

The table is written twice, as a `cfg!` chain for the constant and a `cfg_attr` chain for the `repr`. Three checks hold
them together:

- **A `const` assertion in `src/lib.rs`** makes a mismatch a compile error on the target being built. So
  `cargo check --target` covers targets no test run reaches. When this record was written:
  - It compiled for `aarch64-apple-darwin`, `aarch64-apple-ios`, `aarch64-unknown-linux-gnu`,
    `x86_64-unknown-linux-gnu`, `x86_64-apple-ios`, `armv7-unknown-linux-gnueabi`, `riscv64gc-unknown-linux-gnu` and
    `s390x-unknown-linux-gnu`.
  - It failed when the 32-byte row was deliberately mismatched.
  - A separate compile-time probe confirmed the value each target gets: 128, 128, 64, 128, 32, 64 and 256 for the
    Apple macOS and iOS, Linux AArch64, x86-64, armv7, riscv64 and s390x targets.
- **`a_wrapped_value_occupies_exactly_one_line`** checks the same agreement at runtime.
- **`cache_line_follows_the_architecture_table`** restates the table from `std::env::consts::ARCH` and `OS`, so a wrong
  row fails on the machine running it.

## Alternatives considered

- **One unconditional 64 (ADR 001).** It under-pads Apple Silicon, which is the measured failure.
- **One unconditional 128, the step taken just before this record.** It is right on Apple Silicon and x86-64. But it
  under-pads s390x (256), doubles the padding on Arm's own 64-byte cores, and picks by hand what a maintained table
  already encodes.
- **`crossbeam-utils`' table unchanged, 128 for all AArch64.** It never under-pads Linux on Apple Silicon. But it
  doubles the padding on every Arm server and board, a cost paid on the common AArch64 deployment for the rare one.
- **A build-time override for a known machine,** such as `RUSTFLAGS='--cfg ring_cache_line="64"'`. Not taken.
  - It saves about 64 bytes per padded cursor where the table is too large, with no measured speed benefit.
  - A wrong value silently under-pads.
- **A Cargo feature.** It changes a type's layout, so it is not additive. ADR 001's argument holds.
- **A build script that probes the line size.** It measures the build machine, not the target, which breaks
  cross-compilation and CI-built binaries.
- **Querying the line size at runtime.** That cannot drive `repr(align)`.

## Consequences

- On Apple Silicon under macOS and on x86-64 the value is 128, the same as the unconditional raise that preceded this
  record. The M4 measurements apply unchanged.
- Arm's own AArch64 cores get 64, their line size, so a `PaddedCursor` there is 64 bytes. No AArch64 host other than
  the M4 has been benchmarked with either value.
- Linux on Apple Silicon is under-padded: its cursors can share a 128-byte line, as they did on the M4 before ADR 002.
- x86-64 parts get twice the line they have, for the prefetcher.
- Tests that pin a byte size by literal (`ring_claim`'s and `ring_gating`'s) run only on x86-64 and Apple AArch64.
  The rest compare against `CACHE_LINE`.
- The predicates appear twice, in the constant and in the `cfg_attr`s, so a new row is two edits.
- Revisit when a benchmark on Arm's own cores shows that 64 lets cursors interfere, for example through a pair-fetching
  prefetcher like x86-64's.
- Revisit when Linux on Apple Silicon becomes a target the family ships to.
- Revisit when `crossbeam-utils` changes its table.
