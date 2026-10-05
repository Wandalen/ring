# `CACHE_LINE` is one unconditional 64 for every target, raised by hand at port time

Status: Superseded by [ADR 002](002_cache_line_follows_the_target_architecture.md), after the revisit trigger below
fired on an Apple M4 Pro.

## Context

`ring_cursor::PaddedCursor` wraps its atomic in `ring_align::CacheAligned`, so the producer and consumer cursors
each get a cache line of their own. The padding is only as good as the number behind it.

The real line size varies. It is 64 bytes on x86-64 and on common AArch64, and 128 on Apple Silicon. The two
directions of error cost very different amounts. A value too large wastes memory and still separates. A value too
small separates nothing. Two cursors 64 bytes apart still share a 128-byte line, and every test still passes.

`#[repr(align(..))]` accepts only an integer literal (E0693), so the padding is fixed when the type compiles and
cannot come from a computed value. `CacheAligned` therefore carries its own `#[repr(align(64))]` literal beside
`CACHE_LINE`, and the two must change together.

The constant also needed a home. It could have been a `const` in `ring_cursor` or in `ring_types`, or a crate of its
own.

## Decision

`ring_align::CACHE_LINE` is `64` on every target, with no `cfg`, no Cargo feature and no runtime query. A port to a
host with larger lines raises the constant and the `repr` literal by hand, in the same edit.

Two tests in `ring_align/tests/align_test.rs` guard the edit. `a_wrapped_value_occupies_exactly_one_line` compares
`align_of::<CacheAligned<_>>()` with `CACHE_LINE`, so it fails if the constant and the literal disagree.
`cache_line_must_not_shrink_below_the_current_known_minimum` fails on a decrease and lets a raise through.

The constant lives in its own crate. That gives the concept a name a reader can grep for, a manifest edge
(`ring_cursor` depends on `ring_align`) that says layout is decided elsewhere, and a place for the negative control
`two_unwrapped_fields_share_a_line`, which builds an unpadded struct on purpose.

## Alternatives considered

- **`cfg(target_arch = "aarch64")` selecting 128.** Wrong axis. Graviton, most Android devices and the Raspberry Pi 4
  are 64-byte AArch64. Line size is a property of the microarchitecture, and Rust has no `cfg` for it. This would
  double the padding on every server ARM target to fix one desktop target.
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
  be reachable everywhere for free. It lost because `ring_types` holds domain vocabulary and 64 is a hardware fact,
  and because the unpadded negative control would have no natural home there. Both are tidiness arguments.

## Consequences

- On a 128-byte-line host every guarantee in `ring_align` holds as stated and buys nothing. The contention the crate
  exists to remove is back, and the suite stays green.
- The "one edit site" premise is false today. Two bare `64`s stand in for `CACHE_LINE`: the `cursor.addr() % 64`
  doctest on `ring_cursor::PaddedCursor::addr`, and `claim.abs_diff(consume) >= 64` in
  `ring_mpsc::Producer::on_distinct_lines`. `ring_mpsc` does not depend on `ring_align`, and `ring_cursor` re-exports
  `SeqCell` but not `CACHE_LINE`, so reaching the constant from there costs a manifest edit. A port to 128 must also
  edit `ring_mpsc`.
- The `ring_mpsc` copy is running code, and it measures distance by subtraction rather than by line index. On a
  128-byte port it keeps passing while asserting half the separation. Its two cursors are separate allocations, so
  the check is close to vacuous in either form.
- A separate crate makes ownership visible but does not enforce it. The `ring_mpsc` copy appeared while `ring_mpsc`
  already depended on `ring_cursor`.
- No automated test compares `CACHE_LINE` with the host. Every layout assertion compares against `CACHE_LINE` itself,
  so the suite is self-consistent at any value. `cache_line_is_sixty_four` pins the chosen value, and the floor test
  guards only a decrease. The manual host check in `ring_align/tests/manual/readme.md` is the only comparison.
- Revisit when the family targets Apple Silicon for a benchmark or a shipped build. Raise the constant and the `repr`
  literal to 128 together, and fix the `ring_mpsc` copy in the same change.
- Revisit when the family must run on two line sizes in one binary. That invalidates the design rather than adjusting
  it. Padding would have to be computed by hand instead of by `repr(align)`, which means `unsafe` and a workspace lint
  exemption.
- Revisit when a benchmark shows that doubling the padding to 128 costs measurable cache footprint. That reopens the
  `cfg` alternatives with data.
