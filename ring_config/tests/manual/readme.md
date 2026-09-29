# ring_config — manual testing plan

`tests/config_test.rs` asserts the builder behaves. This plan covers the one
thing a passing test cannot establish: that a *caller* can tell a clamped value
from an honoured one. A builder that silently corrects an impossible input is
only safe if the correction is documented where the caller will read it.

Run from the workspace root.

## M1 — every clamp is documented on the setter that performs it

Two setters silently correct their input: `with_producers` raises 0 to 1, and
`with_batch` clamps into `1..=capacity`. A caller writing
`.with_batch( user_supplied )` has no way to know their value was changed.

```bash
grep -nE -B 14 "pub (const )?fn with_producers" ring_config/src/lib.rs
grep -nE -B 16 "pub (const )?fn with_batch" ring_config/src/lib.rs
```

Note the `(const )?` — every setter here is `const fn`, so a pattern matching a
bare `pub fn` finds nothing and reads as "the method is missing" rather than
"my pattern is wrong".

**Expected:** each doc states the clamp, says *why* it clamps rather than
erroring, and shows the clamped case in its own example — not only the ordinary
one.

## M2 — no setter can produce an unbuildable configuration

The point of clamping instead of erroring is that a builder chain has no `?` in
the middle. That is only worth it if the resulting record is always valid.

```bash
grep -nE "pub (const )?fn " ring_config/src/lib.rs
```

**Expected:** every `with_*` returns `Self`, never `Result`. Only `new` returns
a `Result`, because capacity is the one field that cannot be sensibly corrected
— clamping 7 to 8 would give a caller a ring of a different size than they asked
for, silently.

## M3 — the fields are private, so a record cannot be assembled invalidly

```bash
grep -n -A 10 "pub struct RingConfig" ring_config/src/lib.rs
```

**Expected:** no `pub` on any field. A caller cannot write
`RingConfig { capacity, batch: 9999, .. }` and route around the clamp.

## M4 — the tick-safety reading matches what a system would need

`is_tick_safe` is what feature 183 branches on. It must be exactly
"non-blocking", not a hand-maintained list that drifts from `WaitKind`.

```bash
grep -n -A 6 "pub fn is_tick_safe" ring_config/src/lib.rs
```

**Expected:** the body delegates to `WaitKind::is_non_blocking` rather than
matching variants itself. A local `match` would silently answer wrongly the
first time a fifth wait kind is added.

```bash
grep -nE -A 6 "pub (const )?fn is_tick_safe" ring_config/src/lib.rs
```

## M5 — the doc examples are the API's first reader

```bash
cd ring_config && cargo test --doc
```

**Expected:** every example passes and reads as an explanation on its own.

## Run Record

| Date | Check | Result | Note |
| ---- | ----- | ------ | ---- |
| 2026-08-28 | M1 | ✅ | `with_producers` states the 0→1 clamp *and* why (keeps the chain infallible), and its example shows both the clamped and the honoured case. `with_batch` states the `1..=capacity` clamp and why, and its example shows all three: 0→1, 8→8, 999→16. |
| 2026-08-28 | M2 | ✅ | All four `with_*` return `Self`. Only `new` returns `Result`. |
| 2026-08-28 | M3 | ✅ | All five fields private (`capacity`, `wait`, `overflow`, `producers`, `batch`) — the clamp cannot be routed around by struct-literal construction. |
| 2026-08-28 | M4 | ✅ | Body is `self.wait.is_non_blocking()` — delegation, not a local match. |
| 2026-08-28 | M5 | ✅ | 13 doc tests pass. |

The first run of M1/M2/M4 reported "method missing" because the grep pattern was
`pub fn` and every method here is `pub const fn`. The commands above carry
`(const )?` for that reason — the finding was in the check, not the crate.
