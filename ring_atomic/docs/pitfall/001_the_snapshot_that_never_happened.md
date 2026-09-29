# Pitfall: The Snapshot That Never Happened

### Scope

**Purpose:** Record why a torn `OpCounts` is hard to catch rather than merely
possible — the one consistency check the type offers cannot see it, and every
artifact a reader would consult to learn otherwise is quiescent.

**Responsibility:** What a caller can and cannot verify about a returned
`OpCounts`, and what the crate's examples and tests teach about when reading is
safe.

**In Scope:** `ring_atomic/src/lib.rs:378-424`;
`ring_atomic/tests/atomic_test.rs:259-286`.

**Out of Scope:** That the four reads happen at four moments at all is
[`algorithm/002`](../algorithm/002_counts_is_four_reads_not_one.md) AT3, and the
matching write-side seam in `reset_counts` is AT4 there. That `total` is a stored
field rather than a method is
[`data_structure/002`](../data_structure/002_opcounts_and_the_total_it_stores.md).

---

## The Contract, the Body, and Everything a Reader Would Check

```sh
cd "$(git rev-parse --show-toplevel)"
echo '  -- the whole contract, and the body that answers it --'
awk '/^  \/\/\/ What this cell has been asked to do so far — read as four separate$/{ print } /^  \/\/\/ assert_eq!\( cell\.counts\(\)\.loads, 1 \);$/{ n2 = NR } n2 && NR >= n2 + 2 && NR <= n2 + 17 { print }' ring_atomic/src/lib.rs
echo '  -- the one test whose name promises contention --'
awk '/^    "including the failure case"$/{ n1 = NR } n1 && NR >= n1 + 5 && NR <= n1 + 6 { print } /^  thread::scope\( \| scope \|$/{ print } /^  \} \);$/{ n3 = NR } n3 && NR >= n3 && NR <= n3 + 3 { print }' ring_atomic/tests/atomic_test.rs
echo '  -- where the family reads a counting cell, against where it spawns threads --'
for f in ring_atomic/tests/atomic_test.rs \
         ring_batch/tests/batch_test.rs \
         ring_tls/tests/tls_test.rs
do printf '    %-14s last counts() line %-5s first thread::scope line %s\n' \
   "$( echo "$f" | sed 's|ring/||;s|/tests/.*||' )" \
   "$( command grep -n 'counts()' "$f" | tail -1 | cut -d: -f1 )" \
   "$( command grep -n 'thread::scope' "$f" | head -1 | cut -d: -f1 )"; done
```

Live output:

```
  -- the whole contract, and the body that answers it --
  /// What this cell has been asked to do so far — read as four separate
  ///
  /// # Not a Snapshot
  ///
  /// This is **four independent `Relaxed` loads**, not one atomic read. Under
  /// concurrent use the four fields can come from four different instants, and
  /// [`OpCounts::total`] is the sum of readings that were never simultaneously
  /// true. Assert on a single field, or take the reading while nothing else
  /// touches the cell — never on a *relationship between two fields*.
  ///
  /// Fix(AT3): a two-million-sample probe against a writer whose own loop keeps
  /// `loads >= stores` true at every instant found 11,575 returned structs
  /// reporting `stores > loads`, the widest by 5,456. Concurrency is not
  /// hypothetical here: every `SeqCell` in the family is shared between a
  /// producer and a consumer, and this crate's own contention test drives four
  /// threads against one cell.
  ///
  -- the one test whose name promises contention --
#[ test ]
fn counts_are_exact_under_contention()
  thread::scope( | scope |
  } );

  assert_eq!( cell.counts().fetch_adds, THREADS * EACH );
  assert_eq!( cell.load( Ordering::Acquire ), Seq( ( THREADS * EACH ) as u64 ) );
  thread::scope( | scope |
  } );

  assert_eq!( cell.load( Ordering::Acquire ), Seq( 2000 ) );
}
  -- where the family reads a counting cell, against where it spawns threads --
    ring_atomic    last counts() line 407   first thread::scope line 114
    ring_batch     last counts() line 275   first thread::scope line 340
    ring_tls       last counts() line 233   first thread::scope line 431
```

---

### AT43 — The Only Cross-Field Check `OpCounts` Offers Is Blind to the Only Inconsistency It Has

A careful caller who suspects a returned `OpCounts` might not be self-consistent has
exactly one relation available to test it with. `OpCounts` has five fields and four
of them are independent; the fifth, `total`, is the only one whose value is
constrained by the others. So the natural defensive assertion is
`total == loads + stores + fetch_adds + compare_exchanges`.

That assertion cannot fail. `total` is summed inside `counts()` from the same four
locals it just read — not from a fresh re-read — so it is consistent with the four
values the caller sees by construction, torn or not. Measured against a writer that
holds `loads >= stores` true at every real instant, so that any snapshot reporting
`stores > loads` is one that never happened:

```
  -- one writer holding `loads >= stores` true at every instant --
    snapshots taken                        2000000
    snapshots reporting stores > loads     21740
    widest impossible skew                 1198
  -- and the only cross-field check OpCounts offers, applied to them --
    of those, rejected by the total check  0
```

A second independent run gave 21,475 impossible snapshots, widest skew 902, and
again zero rejected.

**Finding.** Roughly one snapshot in a hundred describes a state the cell was never
in, off by as much as 1,198 operations rather than by one, and the type's own
internal consistency relation certifies every one of them as sound. The check a
suspicious caller would reach for is precisely the check that cannot detect the
problem — not because it is weak, but because `total` is derived from the torn
reads rather than independently of them.

This is what separates the hazard from the mechanism. That the four reads are not
simultaneous ([`algorithm/002`](../algorithm/002_counts_is_four_reads_not_one.md)
AT3) would be a manageable fact if it were detectable. It is not detectable from the
returned value, and nothing else is returned.

The vacuity is now stated where a caller reaching for the check would find it — on
`OpCounts` itself rather than only on the method that fills it — and a test drives
the check over a struct that was never observed at all:

```sh
cd "$(git rev-parse --show-toplevel)"
echo '  -- the consequence, now stated on the type and not only on the method --'
command grep -m1 -A3 -F '/// 2. **The tearing is not detectable from the value.**' ring_atomic/src/lib.rs
echo '  -- and what now exercises it --'
printf '    assertions over hand-built OpCounts : %s\n' \
  "$( command grep -c '  invented\.\|  inconsistent\.' ring_atomic/tests/atomic_test.rs )"
printf '    tests naming the derivation         : %s\n' \
  "$( command grep -c 'fn total_is_derived_from_the_same_four_reads' ring_atomic/tests/atomic_test.rs )"
```

Live output:

```
  -- the consequence, now stated on the type and not only on the method --
/// 2. **The tearing is not detectable from the value.** `total` is the sum of
///    the same four readings, so `loads + stores + fetch_adds +
///    compare_exchanges == total` holds for every torn struct as strongly as
///    for a clean one. The consistency check a suspicious caller would reach
  -- and what now exercises it --
    assertions over hand-built OpCounts : 4
    tests naming the derivation         : 1
```

**Disposition:** applied — as documentation and as a test, not as a fix: there is no
fix that keeps the type. Making the check meaningful means re-reading `total` from
its own counter so it can disagree with the four fields, which turns a derived value
into a fifth independent read and buys a *different* torn struct rather than a
coherent one; making it unnecessary means serializing the four counters, which gives
`CountingSeq` a contention profile of its own and destroys the reason it exists. So
`OpCounts`'s doc in `src/lib.rs` now carries a `# Not a Coherent Observation`
section whose second numbered consequence states in bold that the tearing is not
detectable from the value, spells out why `loads + stores + fetch_adds +
compare_exchanges == total` holds for a torn struct exactly as strongly as for a
clean one, and carries a `Fix(AT43)` line; `total_is_derived_from_the_same_four_reads`
in `tests/atomic_test.rs` asserts the relation over a genuinely observed struct and
then over two hand-built ones — an `invented` struct whose fields no cell ever
produced and which the check certifies anyway, and an `inconsistent` one that shows
nothing rejects a `total` disagreeing with its own fields. What this does not buy:
the type still exposes five fields with no marker on the four that are independent
reads, `counts()` still returns it by value, and a caller who never reads the
rustdoc is in precisely the position this entry describes. The change makes the
vacuity findable; it does not make the tearing detectable. Now prints:
`    assertions over hand-built OpCounts : 4`

---

### AT44 — Every Artifact a Reader Would Consult Says the Snapshot Is Safe

The contract is one line — "What this cell has been asked to do so far" — and "so
far" names a moment. Nothing else in the crate corrects that reading, and three
separate artifacts reinforce it.

The doctest on `counts` constructs a cell, performs one `load`, and asserts
`counts().loads == 1`: single-threaded, so exact. `reset_counts`' contract describes
the same discipline — "for a test that sets up a state through the cell and then
wants to count only what the operation under test does" — which is the quiescent
pattern and is genuinely safe. And in both downstream crates every `counts()` call
in the file precedes every `thread::scope` in it: 267 against 329 in `ring_batch`,
225 against 403 in `ring_tls`. Sixteen call sites, none of them reading a cell with
a live writer behind it.

The remaining artifact is the one that names the hazard's own conditions.
`counts_are_exact_under_contention` spawns four threads doing ten thousand
`fetch_add`s each — and calls `counts()` after the scope has closed and every thread
has joined. It is a correct and worthwhile test: it proves the bookkeeping loses no
increments under contention, which is exactly what its body comment claims. But its
*name* is the only place in the crate where "counts" and "contention" appear
together, and what it establishes is that the totals are exact once everything has
stopped — not that a snapshot taken while things are moving means anything.

**Finding.** The discipline that makes `counts()` safe is real, unanimous across
sixteen call sites, and written down nowhere. A reader has a one-line contract that
implies an instant, a doctest that is exact, a sibling method whose doc describes the
quiescent pattern without naming it as a requirement, and a passing test whose name
says contention is covered. Every signal available points the same way, and the way
is wrong for the case the signals do not exercise.

One clause on the existing sentence closes it: *what this cell has been asked to do
so far — read as four separate values, so meaningful only when nothing else is
touching the cell.* That costs nothing and turns the unanimous convention into a
stated one.

The contract now carries that clause verbatim:

```sh
cd "$(git rev-parse --show-toplevel)"
command grep -m1 -A2 -F '/// What this cell has been asked to do so far' ring_atomic/src/lib.rs
```

Live output:

```
  /// What this cell has been asked to do so far — read as four separate
  /// values, so meaningful only when nothing else is touching the cell.
  ///
```

**Disposition:** applied — `counts`'s contract in `src/lib.rs` now carries
the suggested clause verbatim; the crate's 21 unit tests plus 8 doctests
re-verified passing (`cargo test --all-features`, 2026-09-03). Now prints:
`What this cell has been asked to do so far — read as four separate`

---

### Related

| Instance | Relationship |
|----------|--------------|
| [`algorithm/002`](../algorithm/002_counts_is_four_reads_not_one.md) | The mechanism this is the consequence of, and the same seam on the write side |
| [`data_structure/002`](../data_structure/002_opcounts_and_the_total_it_stores.md) | `total` as a stored field, which is why the consistency check is vacuous |
| [`item/002`](../item/002_counts_the_method_that_is_not_a_snapshot.md) | `counts` as an item off the trait, and what that costs its reach |
| [`pitfall/002`](002_the_wrap_that_reads_as_an_empty_ring.md) | The other place a defensive construct turns an anomaly into a plausible number |

### Sources

| Fact | Where |
|------|-------|
| The one-line contract and the four-read body | `ring_atomic/src/lib.rs:378-424` |
| `total` summed from the four locals, not re-read | `ring_atomic/src/lib.rs:418-422` |
| The contended test reading after the join | `ring_atomic/tests/atomic_test.rs:259-286` |
| Sixteen downstream call sites, all ahead of any spawn | Census above |
| ~21,500 impossible snapshots per 2,000,000, none rejected | Release probe, two runs, quoted above |

### Tests

| Test | Covers |
|------|--------|
| `counts_are_exact_under_contention` | That no increment is lost under four-thread contention, asserted after every thread has joined |
| `total_is_the_sum_of_the_four_and_not_an_independent_counter` | That `total` agrees with the four fields — the relation this instance shows cannot fail |
| `a_failed_compare_exchange_still_counts` | That the counting is complete, single-threaded |
| *(to create)* | Nothing reads `counts()` while a writer is live, so no test distinguishes a snapshot from a tally |
