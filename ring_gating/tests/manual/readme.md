# ring_gating — manual testing plan

`tests/gating_test.rs` carries the producer half of
`docs/feature/178_sequence_barrier_and_gating_set.md`, and its central assertion
— a stalled consumer stops the producer at exactly one lap — is fully
automatable. A stopped cursor, a counted loop, and an exact boundary leave no
judgement in it.

What automation cannot reach here is *how the bound is computed*, and that is
where this crate's real risks live. A `slowest` that read `cursors[0]` passes
every single-consumer test in the file. A `headroom` that recomputed free slots
inline rather than delegating to `ring_seqno` would agree with the tests today and
drift the moment `ring_seqno`'s wrap handling changes. And an `Ordering::Relaxed`
substituted for the gating read produces a suite that passes on x86 — where the
hardware supplies the acquire semantics the code failed to ask for — and a data
race on aarch64, which is the exact shape of bug this family exists to make
impossible.

Each check below is a source reading for that reason. Run from the workspace
root.

The first three were rewritten when the load-and-fold moved out of this crate
into `ring_cursor::slowest`, shared with `ring_barrier`. They had asserted that
this crate iterated its cursors correctly and named `GATING` twice; both are now
false, and both were replaced by the stronger claim the move makes available —
that this crate does none of it at all. A check whose *expectation* still
matched after such a move would have been a check that was not reading anything
load-bearing.

## M1 — the fold is delegated, not restated

`slowest` must fold across the whole set, and the fold itself now lives in
`ring_cursor`. A single-consumer test cannot tell a real fold apart from
`cursors[0].load()`, and neither can a multi-consumer test whose slow consumer
happens to sit at index 0 —
`the_slowest_consumer_sets_the_bound_regardless_of_position_in_the_set` exists
because of that, but it only covers three positions.

```bash
grep -vE "^[[:space:]]*(///|//!)" ring_gating/src/lib.rs \
  | grep -nE "\.iter\(\)|self\.cursors\[|cursors\.get\( 0|\.min\(|\.fold\("
```

The pattern deliberately omits a bare `.map(`: `limit` maps over the `Option`
that `slowest` already returned, which is consuming the delegated answer rather
than computing one. (`headroom` spells its own consumption `map_or`, which a
bare `.map(` does not match in any case, and `frontier` is `ring_barrier`'s
method — it appears nowhere in this crate.) Widening the pattern to catch
`limit` would make this check fail on correct code, which is the way a source
reading gets deleted rather than fixed.

**Expected:** **no output at all.** Not "one correct `.iter()`" — none. This
crate hands `&self.cursors` to `ring_cursor::slowest` and reads the answer; the
iteration, the per-cursor load and the minimum are all one function, one crate
away, shared with `ring_barrier`, which asks the opposite question of the same
kind of slice.

Any hit here is a second copy of that fold. The reason that matters is not
tidiness: `ring_barrier`'s side of it must read at the same ordering, and two
copies is exactly how one of them ends up `Relaxed`.

## M2 — this crate names no ordering at all

The whole family reads cursors to decide whether a slot is safe to touch, and
that decision is stated once, in `ring_cursor::GATING`. Before the fold moved,
the strongest available claim here was "`GATING` twice and no bare `Ordering::`".
Now the claim is stronger and simpler.

```bash
grep -vE "^[[:space:]]*(///|//!)" ring_gating/src/lib.rs \
  | grep -oE "Ordering::[A-Za-z]+|GATING|SeqCell" | sort | uniq -c
```

**Expected:** **no output at all.** No `Ordering::`, no `GATING`, and no
`SeqCell` — this crate no longer performs an atomic read, so it has no ordering
to name and no reason to import the trait that would let it. A hit of any kind
means a load came back into a crate whose job is arithmetic over an answer
someone else read.

## M3 — the free-slot arithmetic is `ring_seqno`'s, not restated

Sequence arithmetic wraps, and the wrap handling lives in one crate on purpose.
`headroom` looks like two lines of subtraction; written as two lines of
subtraction it would be correct until the first `u64` wrap and correct in every
test that never reaches one.

```bash
grep -vE "^[[:space:]]*(///|//!)" ring_gating/src/lib.rs \
  | grep -oE "ring_(seq|cursor)::[a-z_]+" | sort | uniq -c
grep -vE "^[[:space:]]*(///|//!)" ring_gating/src/lib.rs \
  | grep -nE "\.0[[:space:]]*[-+]|saturating_sub|wrapping_sub"
```

**Expected:** `1 ring_cursor::slowest` and `1 ring_seqno::free_slots` from the
first command — the delegated fold and the delegated arithmetic, one call each —
and **no output at all** from the second. Any arithmetic on a `Seq`'s inner
`u64` in this crate is a second implementation of `ring_seqno`.

Note that `slowest` moved crates without changing meaning: it was
`ring_seqno::slowest` over a `Vec< Seq >` this crate built by hand, and is now
`ring_cursor::slowest` over the cursors directly. The `Vec` went with it.

## M4 — the two failures are ordered, and the order is the load-bearing part

`check` tests `BatchTooLarge` before `Full`. Reversed, a claim wider than the
ring on a full ring reports `Full` — retryable — and a caller's retry loop spins
forever on a claim that can never fit. `the_two_failures_are_distinguished_at_the_boundary`
asserts the outcome; this check asserts the structure that produces it, because
a `headroom`-first implementation can be made to pass that one test by special
case.

```bash
grep -vE "^[[:space:]]*(///|//!)" ring_gating/src/lib.rs \
  | grep -nE "RingError::(BatchTooLarge|Full)"
```

**Expected:** two hits, `BatchTooLarge` on the lower line number. The capacity
test is unconditional and comes first; the headroom test is what remains.

## M5 — the empty set returns capacity, not zero

The distinction the module documentation argues. `ring_cursor::slowest` returns
`Option`, and the whole point of that `Option` is discarded by an
`unwrap_or( Seq::ZERO )`, which compiles, reads naturally, and deadlocks every
ungated ring at its first lap.

That `Option` is also why the fold could be shared with `ring_barrier` at all:
the two crates resolve *no dependencies* to opposite values — full headroom
here, nothing readable there — so a fold that picked either would have been
wrong for one of them. Discarding the `Option` in this crate does not just lose
information locally; it silently adopts the other crate's answer.

```bash
grep -vE "^[[:space:]]*(///|//!)" ring_gating/src/lib.rs \
  | grep -nE "map_or|unwrap_or|Seq::ZERO"
```

**Expected:** exactly one hit — `map_or( self.capacity.get(), … )` in
`headroom`. No `Seq::ZERO`, and no `unwrap_or` of any kind: the empty case has
its own answer and it is a capacity, not a position.

## M6 — every declared dependency is actually used

```bash
comm -23 \
  <( grep -E "^ring_" ring_gating/Cargo.toml | cut -d' ' -f1 | sort ) \
  <( grep -vE "^[[:space:]]*(///|//!)" ring_gating/src/lib.rs \
     | grep -oE "ring_[a-z_]+" | sort -u )
```

**Expected:** no output. The comparison is against code lines only —
`ring_barrier` is named several times in this crate's prose, and a check reading
the whole file would count it as used and let a genuinely unused dependency
through beside it.

## M7 — the gate is under pressure in at least one test

The crate's own risk, restated as a reading of its test file rather than its
source. Every assertion about `headroom` on an empty or near-empty ring is
passed by `fn headroom( … ) -> usize { self.capacity.get() }`. Only a test that
holds a consumer still while the producer runs a full lap can fail that.

```bash
grep -c "stalled" ring_gating/tests/gating_test.rs
grep -n "fn a_stalled_consumer_stops_the_producer_at_exactly_one_lap" \
  ring_gating/tests/gating_test.rs
```

**Expected:** a non-zero count from the first, and the named test present from
the second. If that test is ever deleted or weakened to a smaller distance than
one full lap, this crate's suite becomes a suite that cannot fail for the reason
the crate exists.

---

## Run Record

| Date | Checks | Result |
| ---- | ------ | ------ |
| 2026-08-28 | M1–M7 | 7/7 as expected |
| 2026-08-28 | M1–M7 | 7/7 after M1–M3 were rewritten for the fold's move to `ring_cursor::slowest` — M1 and M2 now expect no output at all |
| 2026-08-29 | M1–M7 | 7/7. M1's note was corrected: it justified the `.map(` omission by naming `frontier`, which is `ring_barrier`'s method and appears nowhere here, and `headroom`, whose `map_or` the pattern never matched. The one method the omission protects is `limit`. No pattern or expectation changed |
