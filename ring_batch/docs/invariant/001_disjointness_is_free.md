# Invariant: Disjointness Is Free

### Scope

**Purpose:** Record that no two claims ever overlap, that this holds through both
entry points for the same single reason, and that the test asserting it is
therefore structurally unable to fail on the bug this crate has.

**Responsibility:** The disjointness property, the `fetch_add` it rests on, and
what `concurrent_batch_claims_never_overlap` does and does not constrain.

**In Scope:** `ring_batch/src/lib.rs:221-224`, `:328`;
`ring_batch/tests/batch_test.rs:323-367`.

**Out of Scope:** The capacity bound, which is a different invariant and has no
test, is [`invariant/002`](002_ascending_not_contiguous.md). The race itself is
[`pitfall/001`](../pitfall/001_the_window_between_the_gate_and_the_advance.md).

---

## One Instruction, Two Entry Points

```sh
cd "$(git rev-parse --show-toplevel)"
echo '  -- the two contention tests, and what they call --'
command grep 'thread::scope' ring_batch/tests/batch_test.rs
command grep 'scope.spawn' -A 3 ring_batch/tests/batch_test.rs | command grep -E 'claim\(|claim_gated\('
echo '  -- claim_gated in the suite --'
echo "    total references         : $( command grep -c claim_gated ring_batch/tests/batch_test.rs || true )"
echo "    last one at line         : $( command grep -n claim_gated ring_batch/tests/batch_test.rs | tail -1 | cut -d: -f1 )"
echo "    contention section opens : $( command grep -n 'fn concurrent_batch_claims_never_overlap' ring_batch/tests/batch_test.rs | cut -d: -f1 )"
echo '  -- the advance both entry points share --'
command grep -m1 -A3 -F 'pub fn claim< C : SeqCell >( cursor : &C, count : usize, order : Ordering ) -> BatchClaim' ring_batch/src/lib.rs
command grep -m1 -A1 -F '  Ok( claim( producer, count, order ) )' ring_batch/src/lib.rs
```

Live output:

```
  -- the two contention tests, and what they call --
  let claims : Vec< BatchClaim > = thread::scope
  let per_thread : Vec< Vec< BatchClaim > > = thread::scope
          ( 0..BATCHES ).map( |_| claim( cursor, SIZE, Ordering::AcqRel ) ).collect::< Vec< _ > >()
          ( 0..BATCHES ).map( |_| claim( cursor, 4, Ordering::AcqRel ) ).collect::< Vec< _ > >()
  -- claim_gated in the suite --
    total references         : 13
    last one at line         : 314
    contention section opens : 324
  -- the advance both entry points share --
pub fn claim< C : SeqCell >( cursor : &C, count : usize, order : Ordering ) -> BatchClaim
{
  BatchClaim::new( cursor.fetch_add( count as u64, order ), count )
}
  Ok( claim( producer, count, order ) )
}
```

Thirteen references to `claim_gated`, the last of them ten lines before the
contention section begins and none of them inside it.

---

### BA22 — The Property Holds Through the Racy Path Too, Because It Is `fetch_add`'s Property and Not the Gate's

`claim_gated`'s last line is `Ok( claim( producer, count, order ) )`. Both entry
points hand out sequences through the same `fetch_add`, and `fetch_add` is
atomic — every caller gets a distinct old value, whatever else is happening.
Disjointness therefore does not depend on the gate at all.

Measured on the shape of the crate's own contention test, twenty runs each:

```
--- 4 threads x 500 batches x 8 slots, 20 runs each ---

                                            claim    claim_gated
  sequences claimed twice (worst)               0              0
  final cursor                              16000          16000
  per-thread order inversions                   0              0
```

**Finding.** Sixteen thousand sequences per run, zero claimed twice, through
both entry points including the one with the documented race. The gated column
was run against a ring wide enough that the gate never legitimately refuses, so
every one of its 2000 claims was granted — and every sequence was still unique.

The invariant is real and it is worth having. It is also, in this crate, free:
it costs one atomic instruction and it cannot be broken by anything the gate does
or fails to do.

---

### BA23 — So the Test That Asserts It Cannot Fail on the Bug the Crate Actually Has

The test's own comment states its ambition:

```sh
cd "$(git rev-parse --show-toplevel)"
command grep -m1 -A3 -F '  // The property a claim protocol must never violate. Every claim taken by' ring_batch/tests/batch_test.rs
echo '  -- its two witnesses --'
command grep -m1 -A11 -F '  let mut seen : HashSet< u64 > = HashSet::with_capacity( THREADS * BATCHES * SIZE );' ring_batch/tests/batch_test.rs
```

Live output:

```
  -- its two witnesses --
  let mut seen : HashSet< u64 > = HashSet::with_capacity( THREADS * BATCHES * SIZE );
  for batch in &claims
  {
    assert_eq!( batch.len(), SIZE );
    for seq in batch.sequences()
    {
      assert!( seen.insert( seq.0 ), "sequence {} was claimed twice", seq.0 );
    }
  }

  assert_eq!( seen.len(), THREADS * BATCHES * SIZE );
  assert_eq!( cursor.load( Ordering::Acquire ), Seq( ( THREADS * BATCHES * SIZE ) as u64 ) );
```

**Finding.** "The property a claim protocol must never violate" is precisely
right about protocols in general and precisely inverted for this one. This
protocol cannot violate it. The test is well built — a `HashSet` over individual
sequences really is stronger than comparing ranges, and the final-cursor
assertion really would catch a lost or double-counted advance — and both
witnesses are blind to the failure the crate has, because that failure produces
disjoint sequences that merely point at slots the consumer has not released.

The test also uses `claim`, not `claim_gated`. Both facts point the same way: the
gate is exercised thirteen times, all of them on one thread, and the two tests
that spawn threads exercise the entry point with no gate in it. What the suite
proves about contention is a property of `fetch_add`; what it proves about the
gate is a property of single-threaded execution. Nothing joins the two.

```sh
cd "$(git rev-parse --show-toplevel)"
command grep 'blind to this crate' ring_batch/tests/batch_test.rs
```

Live output:

```
  // fetch_add's own atomicity, but blind to this crate's actual race: a
```

**Disposition:** applied — the test's own comment no longer opens with "The
property a claim protocol must never violate"; it now states what the test
actually checks (disjointness, free from `fetch_add`) and names, in the same
breath, the race it cannot see (→ pitfall/001), instead of reading as a
protocol-wide guarantee.
Now prints: `blind to this crate's actual race`

---

### Related

| Instance | Relationship |
|----------|--------------|
| [`invariant/002`](002_ascending_not_contiguous.md) | The other tested invariant, and the untested one |
| [`pitfall/001`](../pitfall/001_the_window_between_the_gate_and_the_advance.md) | The failure both witnesses are blind to |
| [`algorithm/001`](../algorithm/001_one_fetch_add_whatever_the_count.md) | Why one `fetch_add` is the whole ungated protocol |
| [`decisions/001`](../decisions/001_ordering_is_the_callers_except_where_it_is_not.md) | The missing `loom` seam that would have found the gap |

### Sources

| Fact | Where |
|------|-------|
| `claim`'s body | `ring_batch/src/lib.rs:221-224` |
| `claim_gated` delegating to it | `ring_batch/src/lib.rs:328` |
| The test's comment and witnesses | `ring_batch/tests/batch_test.rs:326-332`, `:355-366` |
| Zero collisions through both entry points | Release probe, quoted above |

### Tests

| Test | Covers |
|------|--------|
| `concurrent_batch_claims_never_overlap` | Disjointness, by `HashSet` and by final cursor position |
| `the_sequences_returned_are_contiguous` | That a single claim's own range has no holes |
| *(to create)* | Disjointness through `claim_gated` under threads — which would pass, and is worth having for the day the advance stops being a `fetch_add` |
