# item

Per-method contracts and coverage for all nine methods, split by tier: the three
that read cursors, and the six that do not. The recurring finding across both is
how little of the surface anything outside the tests actually calls.

### Overview Table

| ID | Name | Covers |
|----|------|--------|
| 001 | [The Three Barrier Readings](001_the_three_barrier_readings.md) | `frontier`, `available`, `admits` — contracts, callers, and the test that restates a body |
| 002 | [The Five Accessors and the Wait](002_the_five_accessors_and_the_wait.md) | `over`, `dependencies`, `len`, `is_empty`, `cursor`, `wait_for` — and the `WaitKind` that never reaches the wait |

### Callers, in All of `src/`

| Method | Library callers |
|--------|----------------:|
| `over` | 0 |
| `dependencies` | 0 |
| `len` / `is_empty` | 0 |
| `cursor` | 0 |
| `frontier` | 3 |
| `available` | 1 (internal) |
| `admits` | 1 (internal) |
| `wait_for` | 0 |

```sh
cd "$(git rev-parse --show-toplevel)"
for m in over frontier available admits wait_for dependencies; do
  printf '%-14s' ".$m"
  for f in ring_*/src/*.rs; do
    n=$( grep -vE "^[[:space:]]*//" "$f" | grep -c "\.$m(" )
    [ "$n" != 0 ] && printf ' %s=%s' "$(basename $(dirname $(dirname $f)))" "$n"
  done
  echo
done
```

**Expected output:**

```
.over
.frontier      ring_barrier=2 ring_consume=1
.available     ring_barrier=1 ring_consume=3 ring_core=2 ring_mpsc=1
.admits        ring_barrier=1
.wait_for
.dependencies
```

`frontier` is the only method a library outside this crate calls, and the crate
that calls it does so to recompute `available` by hand.

**The `.available` row is six hits wider than the table above, and the table is
the correct one.** This census matches a method *name*, never a receiver, so it
cannot tell `Barrier::available` from the three unrelated methods that share the
spelling — `ring_consume` (`src/lib.rs:336`) and `ring_mpsc` (`:988`) each define
their own `available` and call it on `self`, and `ring_core`'s two hits are
`consumer.available()` on a `ConsumerInner` variant. That is BR11 below, seen
from the caller side rather than the definition side.

**`frontier` is the one row this recipe can be trusted on unaided**, and for the
reason BR11 gives: it is the single name this crate invented, so it has exactly
one definition family-wide and every hit necessarily resolves to it. Anywhere
else in the table, a name-matched count is an upper bound and the receiver has to
be checked by hand — the same limitation `ring_types`' `.next()` census hits
(→ [`../../../ring_types/docs/item/associated_function/007_seq_next.md`](../../../ring_types/docs/item/associated_function/007_seq_next.md)).

### Findings Recorded Here

| ID | Subject | Tier | Finding |
|----|---------|------|---------|
| BR3 | `ring_barrier` | n/a — coverage | `available` has exactly 1 library caller (`admits`), `admits` exactly 1 (`wait_for`), and `wait_for` none outside tests — the readings form a chain hanging from the test suite |
| BR4 | `ring_barrier` | n/a — coverage | `wait_for` has 6 call sites, all in `tests/barrier_test.rs`; zero anywhere else in the family |
| BR12 | `ring_barrier` | n/a — coverage | `wait_for` is tested with `None` ×3, `Spin` ×2, `Yield` ×1 and `Park` ×0 — `Park` being the only variant that sleeps |
| BR16 | `ring_barrier` | n/a — coverage | `.dependencies()` has exactly one call site in the entire family, in this crate's own tests |
| BR18 | `ring_barrier` | n/a — coverage | `admits_and_available_never_disagree` asserts `admits( f, c ) == ( c <= available( f ) )` — literally `admits`'s body — across 20,736 cases; it cannot fail today, and exists to guard a future hoisted-frontier optimization |
| BR38 | `ring_barrier` | n/a — observation | `frontier`, `available` and `admits` are one fold plus two arithmetic steps, layered so the cheapest question costs the most expensive one — `admits( from, 0 )` is a constant `true` and still folds every cursor, which is why `ring_consume` re-derives rather than paying twice |
| BR39 | `ring_barrier` | n/a — coverage | `dependencies()` appears twice in the whole family, both in this crate — one doctest, one test assertion, no consumer; the accessor consumers reach for is `cursor( index )`, and the collection form survives because it is what a container would expose |
