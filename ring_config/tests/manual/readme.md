# ring_config manual testing plan

`tests/config_test.rs` asserts the builder behaves. This plan covers the one
thing a passing test cannot establish. It checks that a *caller* can tell a
clamped value from an honoured one. A builder that silently corrects an impossible input is
only safe if the correction is documented where the caller will read it.

Run from the workspace root.

## M1: every clamp is documented on the setter that performs it

Two setters silently correct their input: `with_producers` raises 0 to 1, and
`with_batch` clamps into `1..=capacity`. A caller writing
`.with_batch( user_supplied )` has no way to know their value was changed.
Since the checked setters landed, each clamping setter also names its checked
counterpart in its own doc, so a caller who wants the rejection reads it where
the clamp is described.

```bash
grep -nE -B 14 "pub (const )?fn with_producers" ring_config/src/lib.rs
grep -nE -B 16 "pub (const )?fn with_batch" ring_config/src/lib.rs
```

Note the `(const )?`. Every setter here is `const fn`, so a pattern matching a
bare `pub fn` finds nothing and reads as "the method is missing" rather than
"my pattern is wrong".

**Expected:** each doc states the clamp, says *why* it clamps rather than
erroring, and shows the clamped case in its own example, not only the ordinary
one.

## M2: no setter can produce an unbuildable configuration

The point of clamping instead of erroring is that a builder chain has no `?` in
the middle. That is only worth it if the resulting record is always valid.

```bash
grep -nE "pub (const )?fn " ring_config/src/lib.rs
```

**Expected:** every `with_*` returns `Self`, never `Result`. Only `new` and the
checked `try_with_*` setters return a `Result` — capacity for `new`, the values
the clamps would silently correct for `try_with_producers` and
`try_with_batch`. A chain written on the clamping setters still carries no `?`
in its middle, and every configuration the setters *accept* is buildable, which
is the invariant this check guards. Clamping 7 to 8 would silently give a
caller a ring of a different size than they asked for.

## M3: the fields are private, so a record cannot be assembled invalidly

```bash
grep -n -A 10 "pub struct RingConfig" ring_config/src/lib.rs
```

**Expected:** no `pub` on any field. A caller cannot write
`RingConfig { capacity, batch: 9999, .. }` and route around the clamp.

## M4: the tick-safety reading matches what a system would need

`is_tick_safe` must be exactly "non-blocking", not a hand-maintained list that
drifts from `WaitKind`.

```bash
grep -nE -A 6 "pub (const )?fn is_tick_safe" ring_config/src/lib.rs
```

**Expected:** the body delegates to `WaitKind::is_non_blocking` rather than
matching variants itself. A local `match` would silently answer wrongly the
first time a wait kind is added.

## M5: the doc examples are the API's first reader

```bash
cd ring_config && cargo test --doc
```

**Expected:** every example passes and reads as an explanation on its own.

## Run Record

| Date | Check | Result | Note |
| ---- | ----- | ------ | ---- |
| 2026-08-28 | M1 | ✅ | `with_producers` states the 0→1 clamp *and* why (keeps the chain infallible), and its example shows both the clamped and the honoured case. `with_batch` states the `1..=capacity` clamp and why, and its example shows all three: 0→1, 8→8, 999→16. |
| 2026-08-28 | M2 | ✅ | All four `with_*` return `Self`. Only `new` returns `Result`. |
| 2026-08-28 | M3 | ✅ | All five fields private (`capacity`, `wait`, `overflow`, `producers`, `batch`), so struct-literal construction cannot route around the clamp. |
| 2026-08-28 | M4 | ✅ | Body is `self.wait.is_non_blocking()`, which is delegation, not a local match. |
| 2026-08-28 | M5 | ✅ | 13 doc tests pass. |
| 2026-11-21 | M1, M2, M5 | ✅ | Re-run after the checked setters (`try_with_producers`, `try_with_batch`) landed. M1: both clamping setters state their clamp and now point at their checked counterpart. M2: the four `with_*` still return `Self`; `new`, `try_with_producers` and `try_with_batch` return `Result`. M5: 15 doc tests pass in `ring_config`, two of them added by the checked setters; `ring_types`' 20 unchanged. |
