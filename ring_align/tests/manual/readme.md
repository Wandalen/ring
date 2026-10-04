# ring_align manual testing plan

`tests/align_test.rs` asserts the padding works. This plan covers what the tests
cannot decide: whether the constant is right for the machine this runs on, and
whether the crate's stated no-`unsafe` claim holds.

Run from the workspace root.

## M1: the constant matches the host's real cache line

`CACHE_LINE` is 128, the largest line among the family's targets: Apple Silicon
uses 128, x86-64 and common AArch64 use 64. A value too small is the failure
that matters. Two cursors 64 bytes apart still share a 128-byte line, so the
padding silently buys nothing while every test still passes. A value too large
only spends memory.

```bash
getconf LEVEL1_DCACHE_LINESIZE   # Linux
sysctl -n hw.cachelinesize       # macOS
```

**Expected:** `64` or `128`, either way no larger than `CACHE_LINE`. A larger
reading means the constant is wrong for this machine and the padding is
decorative. Record that in the Run Record rather than adjusting the test.

## M2: the crate really contains no `unsafe`

The module doc claims none is needed because `#[repr(align(128))]` is a safe
attribute. The workspace denies `unsafe_code`, so this should be structurally
impossible. Check the claim directly anyway, rather than trusting that the lint
is wired up.

```bash
grep -rn "unsafe" ring_align/src/
grep -rn "allow.*unsafe_code" ring_align/src/ ring_align/Cargo.toml
```

**Expected:** the only `unsafe` hits are inside doc comments saying none is
needed; no `unsafe` block, function, impl, or trait. No `allow` hits at all.
Scope both greps to `src/` and the manifest. Grepping the crate root makes this
plan file match its own search string and report a hit that is not there.

The crate is also absent from
`bench_harness/gate/declared/ring/unsafe_allowlist.txt`, the list of crates
permitted to opt out of the lint.

## M3: the padding is observable on real addresses, not just in `size_of`

`size_of` says a wrapped value is one line. That does not by itself prove two of
them in a struct land on different lines, because the compiler could in
principle lay them out otherwise. The test asserts it on real addresses, with a negative
control.

```bash
cd ring_align && cargo test --test align_test two_wrapped_fields_land_on_different_lines two_unwrapped_fields_share_a_line -- --nocapture
```

**Expected:** both pass. The second is the one that matters. It proves the
first measures the padding rather than something the layout would have done
anyway.

## M4: the doc examples are the API's first reader

```bash
cd ring_align && cargo test --doc
```

**Expected:** every example passes. In particular `on_distinct_lines`'s example
must not take addresses of stack locals. Two adjacent locals share a line, so
such an example asserts something false about the machine while looking
reasonable.

## Run Record

| Date | Check | Result | Note |
| ---- | ----- | ------ | ---- |
| 2026-08-28 | M1 | ✅ | `getconf LEVEL1_DCACHE_LINESIZE` reports 64 on this host, so the constant is right here. Still wrong for Apple Silicon (128). The module doc says so, and says a port raises the constant rather than making it per-crate conditional. |
| 2026-08-28 | M2 | ✅ | Two `unsafe` hits in `src/`, both inside the module doc comment stating none is needed. No `unsafe` block/fn/impl, no `allow`. The check's original command grepped the crate root and so matched this plan file's own search string. The command above now scopes to `src/` and the manifest. |
| 2026-08-28 | M3 | ✅ | Both pass, including the negative control: the unpadded pair really does share a line, so the padded assertion is measuring the padding. |
| 2026-08-28 | M4 | ✅ | 7 doc tests pass. `on_distinct_lines`'s example uses plain integer addresses (0/63, 63/64, 128/130), not stack locals. Stack locals were a real failure earlier in the stage and are the reason the check is written the way it is. |
| 2026-10-04 | M1 | ✅ | `sysctl -n hw.cachelinesize` reports 128 on an Apple M4 Pro, the host the `ring_spsc` and `ring_mpsc` benchmarks ran on. With `CACHE_LINE` at 64 the padding was decorative here; it is now 128. |
