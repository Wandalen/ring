# `CACHE_LINE` is one unconditional 128 for every target, raised by hand at port time

Status: Accepted. Amended 2026-10-04: the value went from 64 to 128 when the revisit trigger below fired.

## Context

`ring_cursor::PaddedCursor` wraps its atomic in `ring_align::CacheAligned`, so the producer and consumer cursors
each get a cache line of their own. The padding is only as good as the number behind it.

The real line size varies. It is 64 bytes on x86-64 and on common AArch64, and 128 on Apple Silicon. The two
directions of error cost very different amounts. A value too large wastes memory and still separates. A value too
small separates nothing. Two cursors 64 bytes apart still share a 128-byte line, and every test still passes.

`#[repr(align(..))]` accepts only an integer literal (E0693), so the padding is fixed when the type compiles and
cannot come from a computed value. `CacheAligned` therefore carries its own `#[repr(align(128))]` literal beside
`CACHE_LINE`, and the two must change together.

The constant also needed a home. It could have been a `const` in `ring_cursor` or in `ring_types`, or a crate of its
own.

## Decision

`ring_align::CACHE_LINE` is `128` on every target, with no `cfg`, no Cargo feature and no runtime query. A port to a
host with larger lines raises the constant and the `repr` literal by hand, in the same edit.

It was `64` until the family was first benchmarked on Apple Silicon, an M4 Pro with 128-byte lines. There the padding
separated nothing. Whether two cursors shared a line depended on where the ring landed in memory, and that changed
between processes: batched `ring_mpsc` at 8 producers ran at 91–95 or 137–143 M/s depending only on placement.
That is the trigger this record named, and the edit is the one it prescribed. 128 is the largest line among the
family's targets, so no target is now under-padded. The before/after measurements, with the machine they ran on, are
in [docs/benchmarks/001_cache_line_128.md](../benchmarks/001_cache_line_128.md).

The cost lands on 64-byte hosts, which get twice the padding separation needs. A `PaddedCursor` is 128 bytes there
instead of 64, and a ring holds a handful of them. On x86-64 the extra line is not pure waste either: the spatial
prefetcher on Intel cores since Sandy Bridge fetches lines in adjacent pairs, which is why `crossbeam-utils` already
pads to 128 there.

Two tests in `ring_align/tests/align_test.rs` guard the edit. `a_wrapped_value_occupies_exactly_one_line` compares
`align_of::<CacheAligned<_>>()` with `CACHE_LINE`, so it fails if the constant and the literal disagree.
`cache_line_must_not_shrink_below_the_current_known_minimum` fails on a decrease and lets a raise through.
`cache_line_is_one_hundred_twenty_eight` pins the chosen value.

The constant lives in its own crate. That gives the concept a name a reader can grep for, a manifest edge
(`ring_cursor` depends on `ring_align`) that says layout is decided elsewhere, and a place for the negative control
`two_unwrapped_fields_share_a_line`, which builds an unpadded struct on purpose.

## Alternatives considered

- **`cfg(target_arch = "aarch64")` selecting 128.** Wrong axis. Graviton, most Android devices and the Raspberry Pi 4
  are 64-byte AArch64. Line size is a property of the microarchitecture, and Rust has no `cfg` for it. The
  unconditional 128 doubles the padding on those targets too, but it cannot under-pad any of them, and an under-pad
  is the error that costs.
- **`cfg(target_vendor = "apple")` with `target_arch = "aarch64"`.** Closer, but still a proxy. It encodes "Apple's ARM
  chips have 128-byte lines" as a build fact that nobody guarantees, and it does nothing for the next vendor that
  ships 128.
- **A Cargo feature such as `cache-line-128`.** A feature that changes a type's layout is not additive. Under feature
  unification, one crate enabling it changes the layout for every crate in the graph, and nothing fails to build.
- **Query the line size at runtime.** Cannot drive the padding, because `repr(align)` needs a literal. A runtime value
  could only be checked against the compiled one, which turns a quietly wrong layout into a panic on a machine the
  binary was not built for.
- **A `const` in `ring_cursor`.** The concept becomes a detail of the cursor code. Any other crate wanting the number
  would depend on `ring_cursor` and pull in `ring_types`, `ring_seqno` and `ring_atomic` with it.
- **A `const` in `ring_types`.** The close call. `ring_types` is already a universal dependency, so the constant would
  be reachable everywhere for free. It lost because `ring_types` holds domain vocabulary and a line size is a hardware fact,
  and because the unpadded negative control would have no natural home there. Both are tidiness arguments.

## Consequences

- On a host with lines larger than 128 every guarantee in `ring_align` would hold as stated and buy nothing. That was
  the 64-byte value's position on Apple Silicon until the amendment, with the suite green throughout.
- Source code reaches the number only through `ring_align`. Two bare `64`s used to stand in for `CACHE_LINE`: the
  `cursor.addr() % 64` doctest on `ring_cursor::PaddedCursor::addr`, and `claim.abs_diff(consume) >= 64` in
  `ring_mpsc::Producer::on_distinct_lines`, which also measured distance by subtraction rather than by line index. The
  amendment routed both through `ring_align`, which cost `ring_mpsc` a manifest edge. The `repr` literal stays the
  second edit site, guarded by `a_wrapped_value_occupies_exactly_one_line`.
- Tests that pin a byte size by literal (`ring_cursor`'s, `ring_claim`'s and `ring_gating`'s `PaddedCursor` sizes and
  strides) change with the constant. That is deliberate: they pin a layout, and a layout change should make them say so.
- A separate crate makes ownership visible but does not enforce it. The `ring_mpsc` copy appeared while `ring_mpsc`
  already depended on `ring_cursor`.
- No automated test compares `CACHE_LINE` with the host. Every layout assertion compares against `CACHE_LINE` itself,
  so the suite is self-consistent at any value. `cache_line_is_one_hundred_twenty_eight` pins the chosen value, and
  the floor test guards only a decrease. The manual host check in `ring_align/tests/manual/readme.md` is the only
  comparison.
- Revisit when the family must run on two line sizes in one binary. That invalidates the design rather than adjusting
  it. Padding would have to be computed by hand instead of by `repr(align)`, which means `unsafe` and a workspace lint
  exemption.
- Revisit when a benchmark on a 64-byte host shows that the doubled padding costs measurable cache footprint. That
  reopens the `cfg` alternatives with data.
