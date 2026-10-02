# ring_atomic manual testing plan

`tests/atomic_test.rs` asserts the cells behave and the counts are exact. This
plan covers the crate's reason to exist, which no test can reach: the
memory model must be **readable in one place**, and every ordering decision must
be the caller's rather than this crate's.

A helper that quietly chose `SeqCst` would make every test above pass and every
benchmark meaningless. For a family whose output is a measured comparison
between candidate rings, that is the worse failure. It is also
invisible to a test suite, because a program with the wrong orderings is still a
correct program.

Run from the workspace root.

## M1. The sequence path never picks an ordering

```bash
grep -nE "self\.(cell|0)\.[a-z_]+\([^)]*Ordering::" ring_atomic/src/lib.rs
```

**Expected:** no output. Every operation on the underlying `AtomicU64` forwards
the `order` its caller passed; none substitutes a literal.

## M2. The ordering literals that do exist are only on the bookkeeping

M1 alone is not enough. The file does contain `Ordering::` literals, and a
reader has to see that each one acts on a *counter*, never on the sequence.

```bash
grep -nE "Ordering::[A-Za-z]+" ring_atomic/src/lib.rs \
  | grep -vE "^[0-9]+:[[:space:]]*(///|//!)"
```

**Expected:** every hit is on `self.loads`, `self.stores`, `self.fetch_adds`,
`self.compare_exchanges` or the reset loop's `counter`, and every one is
`Relaxed`. A `Relaxed` counter cannot reorder the sequence operation it sits
beside, because the sequence operation carries its own ordering.

## M3. Every trait method takes the ordering explicitly

```bash
grep -nE "fn (load|store|fetch_add|compare_exchange)" ring_atomic/src/lib.rs
```

**Expected:** every signature, on the trait and on both impls, names
`order : Ordering`, or `success`/`failure` for compare-exchange. No default, no
`_seqcst` convenience wrapper.

## M4. `CountingSeq` delegates rather than reimplements

The whole value of the shim is that an assertion made against it is a statement
about production. That holds only if it performs the *same* operation.

```bash
sed -n '/^impl SeqCell for CountingSeq/,/^}/p' ring_atomic/src/lib.rs
```

**Expected:** each method increments its counter and then calls `self.cell.<the
same method>` with the caller's ordering unchanged. No second implementation of
the atomic operation, no altered ordering, no early return.

## M5. The two negative acceptance criteria are named where the shim is defined

`CountingSeq` looks like test-only code shipped in a production crate. It is
not. It is the only way two acceptance criteria can be asserted at all. If that
is not written down, the next reader deletes it.

```bash
grep -n -B 2 -A 14 "Why a trait rather than a struct" ring_atomic/src/lib.rs
```

**Expected:** the module documentation names both criteria, "zero atomic
operations" and "one fence, not 64", and says why neither is assertable against
a bare `AtomicU64`.

## M6. The doc examples are the API's first reader

```bash
cargo test -p ring_atomic --doc
```

**Expected:** every example passes and explains the operation rather than
merely exercising it.

## Run Record

| Date | Check | Result | Note |
| ---- | ----- | ------ | ---- |
| 2026-08-28 | M1 | ✅ | No output. The sequence path forwards `order` in all four operations, in both impls. |
| 2026-08-28 | M2 | ✅ | 9 hits, all `Relaxed`, all on bookkeeping: 4 counter loads in `counts()`, 1 in the `reset_counts` loop, 4 counter increments in the `SeqCell` impl. None touches `self.cell`. |
| 2026-08-28 | M3 | ✅ | 12 signatures, every one naming the ordering explicitly. |
| 2026-08-28 | M4 | ✅ | All four delegate to `self.cell.<same>` with the ordering passed through unchanged. |
| 2026-08-28 | M5 | ✅ | The "Why a trait rather than a struct" section names both criteria and both features. |
| 2026-08-28 | M6 | ✅ | 8 doc tests pass. |

M1 was drafted as "no `Ordering::` literal anywhere in executable code" and
reported 9 hits, which read as a violation of the crate's central promise. It is
not a violation. Every hit is on a `Relaxed` counter, and the sequence itself
never gets an ordering chosen for it. Splitting the check into M1 (the sequence path, which
must be empty) and M2 (everything else, which must all be bookkeeping) is the
correction. The blunt form would have failed forever or been silenced with an
exception, and neither would have said anything true.
