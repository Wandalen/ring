# ring_debug manual testing

Each stage states a **prediction before it is run**, so a wrong prediction is
recorded as a finding rather than quietly corrected into agreement with the
result.

M1 to M3 check that the crate stays opt-in: nothing in the family depends on
it, mentions it from `src/`, or runs any of it implicitly. These are **not
testable from inside the crate**. A test here cannot observe what other crates
depend on. They are greps, and this is where they live.

Run from the repository root.

---

## M1. Does anything in the family depend on this crate?

```bash
grep -rln 'ring_debug' */Cargo.toml
```

**Prediction:** exactly one line, `ring_debug/Cargo.toml`.

---

## M2. Does any family `src/` mention this crate?

```bash
command grep -rn --include=*.rs 'ring_debug' */src/ | command grep -v '^ring_debug/'
```

**Prediction:** no functional dependency, so the dependency arrow points one
way. Every line printed is a doc comment that cites this crate, not a `use` or
a call. This grep is a text search that cannot tell a citation from a real
dependency, so "empty" is not the invariant. M1 (one manifest names the crate)
is what guarantees that no functional dependency exists.

---

## M3. Does anything here run implicitly?

```bash
grep -n 'impl Drop\|\<static \|lazy_static\|OnceLock\|\<ctor\>' ring_debug/src/lib.rs
```

**Prediction:** no match. Nothing runs implicitly by construction. The only
`const` is `OBSERVE`, which is an `Ordering`, not state.

The pattern is word-bounded. A bare substring match on `ctor` is satisfied by
any comment containing "contradictory", "factor", "director" or "victor", and a
bare `static ` by "ecstatic" or "hydrostatic". `\<ctor\>` still catches
`#[ctor]`/`ctor::ctor`-style real usage, and `\<static ` still catches a real
`static FOO: ...` declaration.

---

## M4. Can the strongest check be pointed at a live `ring_core::Ring`?

```bash
grep -n 'pub fn cursors\|-> *&*CursorPair' ring_core/src/lib.rs \
  ring_spsc/src/lib.rs ring_mpsc/src/lib.rs
grep -n 'pub fn position' ring_core/src/lib.rs
```

**Prediction:** no. `ring_core` has no `CursorPair` at all, and the backends keep
theirs private. Both greps come back empty. `ring_core`'s ends expose
`free_capacity`, `is_full`, `len`, `is_empty` and nothing else positional. This
confirms from the source the first known limitation in
[the crate readme](../../readme.md).

---

## M5. Is the crate fully covered?

```bash
cargo tarpaulin -p ring_debug --all-features --out Stdout 2>&1 \
  | grep 'ring_debug/src'
```

**Prediction:** 100%. The public API is small and every branch has a named test.

---

## M6. Are the three values reachable from a `ring_spsc` ring, even though a `CursorPair` is not?

The interesting question M4 leaves open. `ring_spsc::Producer` and `Consumer`
both expose `position() -> Seq`, and `Ring::capacity()` exposes the third value.
So the *data* `check` needs may be reachable from a backend even where the
*type* it takes is not.

```bash
# Written as tests/zz_probe_reach.rs, run, then deleted — see below.
grep -n 'pub fn position' ring_spsc/src/lib.rs
grep -n 'pub fn position\|pub fn capacity' ring_mpsc/src/lib.rs
```

**Prediction:** the data is reachable from `ring_spsc` and the API shape is the
only obstacle. That would mean `check` taking `&CursorPair` is a narrower
signature than it needs to be, and a public `(Seq, Seq, Capacity)` form would
make the crate usable one layer lower than it is now.

**Result: the prediction is half right, and the half that is wrong is the more
useful half.**

`ring_spsc` exposes `position()` on both ends, so for the single-producer
backend the three values are reachable and the signature is the only obstacle.
`ring_mpsc` does **not**. Its producer end exposes `claimed()` and its consumer
`position()`, and `claimed` is the claim cursor, not the publish cursor. The
publish side is `committed()` and `published_through()` on the `Ring`, and
`published_through` returns `Option< Seq >` because there may be no published
record at all.

So the two backends do not present the same pair of sequences, and a
`(Seq, Seq, Capacity)` entry point would silently invite the `ring_mpsc` caller
to pass `claimed()`. That cursor is legitimately ahead of the publish cursor, so
the check would report D1 (a consumer ahead of its producer) on a healthy ring.
**The narrow signature prevents this.** Taking a `CursorPair` means the caller
cannot assemble a wrong pair, and the reachability cost is the price of that.

This is why the first known limitation in [the crate readme](../../readme.md)
proposes that `ring_core` forward a `position()`, rather than a looser
signature here.

---

## Run Record

| Stage | Question | 2026-08-28 |
|---|---|---|
| M1 | Nothing depends on this crate | ✅ one line |
| M2 | No family `src/` mentions it | ✅ empty |
| M3 | Nothing runs implicitly | ✅ no match |
| M4 | Strongest check unreachable from `ring_core` | ✅ confirmed from source |
| M5 | Fully covered | ✅ 59/59 |
| M6 | Data reachable from a backend | ⚠️ only from `ring_spsc`; `ring_mpsc` differs, see above |

**Environment:** `2026-08-28`, Linux 6.8.0, `cargo nextest`, `cargo tarpaulin`.
22 tests + 3 doc tests, all passing. Re-verified `2026-09-11`: 28 tests + 3 doc
tests (1 ignored), all passing. The `checked_sub` refactor and its regression
coverage added 4. Two more covered a watch recovering from a fault
(`a_watch_that_faulted_reports_ok_once_the_ring_recovers`) and the untested
genuinely-full `check_ends` boundary (`check_ends_on_a_genuinely_full_ring`). No
stage's substantive verdict changed.
