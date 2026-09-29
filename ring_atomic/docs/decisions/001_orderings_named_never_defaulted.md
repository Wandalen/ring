# Decisions: Orderings Named, Never Defaulted

### Scope

**Purpose:** Record the crate's central stated decision about memory ordering, the
scope on which it actually holds, and the ordering choices the crate makes for
itself outside that scope.

**Responsibility:** The `named, not defaulted` paragraph, the nine ordering
literals in the crate body, and the question of whether a different one would help.

**In Scope:** `ring_atomic/src/lib.rs:8-17`, `:412-415`, `:450`, `:478`,
`:484`, `:490`, `:497`.

**Out of Scope:** The tear those literals permit is
[`algorithm/002`](../algorithm/002_counts_is_four_reads_not_one.md) AT3, read as a
hazard in [`pitfall/001`](../pitfall/001_the_snapshot_that_never_happened.md). The
trait-versus-struct decision is
[`decisions/002`](002_a_trait_because_the_criteria_needed_two.md).

---

## The Claim, and the Literals

```sh
cd "$(git rev-parse --show-toplevel)"
echo '  -- the decision, as the crate states it --'
command grep -m1 -A4 -F '//! The orderings are therefore **named, not defaulted**: [`SeqCell`]'"'"'s methods' ring_atomic/src/lib.rs
echo '  -- every ordering the crate body chooses without asking --'
command grep -E 'Ordering::(Relaxed|Acquire|Release|AcqRel|SeqCst)' ring_atomic/src/lib.rs | command grep -v '///'
```

Live output:

```
  -- the decision, as the crate states it --
//! The orderings are therefore **named, not defaulted**: [`SeqCell`]'s methods
//! take an explicit [`Ordering`], and this crate never picks one on a caller's
//! behalf. A helper that quietly chose `SeqCst` would make every operation
//! correct and every benchmark meaningless, which for a workstream whose whole
//! output is a measured verdict is the worse failure. That governs the cells
  -- every ordering the crate body chooses without asking --
    let loads = self.loads.load( Ordering::Relaxed );
    let stores = self.stores.load( Ordering::Relaxed );
    let fetch_adds = self.fetch_adds.load( Ordering::Relaxed );
    let compare_exchanges = self.compare_exchanges.load( Ordering::Relaxed );
      counter.store( 0, Ordering::Relaxed );
    self.loads.fetch_add( 1, Ordering::Relaxed );
    self.stores.fetch_add( 1, Ordering::Relaxed );
    self.fetch_adds.fetch_add( 1, Ordering::Relaxed );
    self.compare_exchanges.fetch_add( 1, Ordering::Relaxed );
```

---

### AT13 — The Decision Holds on the Caller's Cell and Is Silent About the Crate's Own

Read strictly, the claim is true: every `SeqCell` method takes an `Ordering`
parameter and forwards it, and no method in the crate substitutes one for a value
the caller supplied. The sentence is accurate about what it says.

It is the paragraph above it that overreaches. That paragraph gives the reason for
the crate existing — "the family's memory model is whatever the sum of those call
sites turns out to be. Concentrating them here makes the model one thing that can
be read in one place" — and the crate then makes nine ordering decisions of its
own, in the same file, none of them named as decisions anywhere.

**Finding.** Every one of the nine is `Relaxed`, on the counting shim's own
bookkeeping, and every one is defensible: the counters exist only to be totalled
afterwards, and a fence per increment would make the shim's cost even less like the
production cell's than it already is
([`algorithm/001`](../algorithm/001_one_intrinsic_or_two.md) AT2). None of that is
written down. The crate that exists to make the family's ordering decisions
readable in one place documents the one it delegates and not the nine it keeps.

The gap matters because these nine are the ones with a reachable consequence. The
`Relaxed` at `:412-415` is what makes `counts` capable of returning a state that
never existed; the `Relaxed` at `:450` does the same on the write side. A reader
who takes "this crate never picks one" at face value will not think to look for an
ordering decision behind that behaviour, because they have been told there is none.

The module doc now names the nine directly, right after the paragraph AT13
quotes:

```sh
cd "$(git rev-parse --show-toplevel)"
command grep -m1 -A4 -F "//! callers hold; [\`CountingSeq\`]'s own bookkeeping counters are \`Relaxed\`" ring_atomic/src/lib.rs
```

Live output:

```
//! callers hold; [`CountingSeq`]'s own bookkeeping counters are `Relaxed`
//! throughout, decided once here rather than per call, because they exist
//! only to be totalled afterwards and no ordering closes the read/read tear
//! between them (see [`CountingSeq::counts`]).
//!
```

**Disposition:** applied — the module doc's `named, not defaulted` paragraph
in `src/lib.rs` now states inline that `CountingSeq`'s nine bookkeeping
orderings are `Relaxed`, decided once rather than per call; the crate's 21
unit tests plus 8 doctests re-verified passing (`cargo test --all-features`,
2026-09-03). Now prints:
`throughout, decided once here rather than per call, because they exist`

---

### AT14 — And No Ordering Would Fix It, Which Is the Part Worth Recording

The natural response to AT13 — strengthen the counters — does not work. The same
four-read snapshot experiment, with every counter operation at `SeqCst` instead of
`Relaxed`, five runs each:

```
  -- five runs of the same experiment at each ordering --
  Relaxed   snapshots 2000000  reporting stores > loads   6014  widest    913
  Relaxed   snapshots 2000000  reporting stores > loads   7323  widest 157823
  Relaxed   snapshots 2000000  reporting stores > loads  14260  widest   4188
  Relaxed   snapshots 2000000  reporting stores > loads   1632  widest      4
  Relaxed   snapshots 2000000  reporting stores > loads  15501  widest    258
  SeqCst    snapshots 2000000  reporting stores > loads  14615  widest   2286
  SeqCst    snapshots 2000000  reporting stores > loads  18040  widest   1940
  SeqCst    snapshots 2000000  reporting stores > loads  17700  widest 810910
  SeqCst    snapshots 2000000  reporting stores > loads  10511  widest   6878
  SeqCst    snapshots 2000000  reporting stores > loads      1  widest     38
```

**Finding.** Ten runs, ten non-zero counts. The rate varies by four orders of
magnitude with scheduling and shows no ordering effect whatsoever; `SeqCst`'s runs
are not lower than `Relaxed`'s, and its widest observed skew is the largest of the
twenty numbers here.

That is the expected result once stated plainly: ordering constrains how one
thread's operations become visible relative to each other, not whether four
separately-issued loads observe one instant. Nothing weaker or stronger than
`Relaxed` makes four reads into one. The fix, if the tear ever needs fixing, is
structural — a lock, or four counters packed into one word — and both are
[`workaround`](../workaround/) territory rather than an ordering choice.

So the `Relaxed` is correct, correct for a reason the crate does not give, and
correct against an alternative the crate does not mention. The decision worth
recording is not which ordering was picked but that the question is not an
ordering question at all — and a reader arriving at AT3 with the module comment in
mind has every reason to think it is.

**Disposition:** declined — `SeqCst` was measured against the same tear and
showed no improvement (ten runs, ten non-zero counts, no ordering effect, and
`SeqCst`'s own widest skew was the largest of the twenty numbers recorded); the
finding's own text already frames the real fix as structural —
`workaround/` territory — not an ordering choice this doc can apply.

---

### Related

| Instance | Relationship |
|----------|--------------|
| [`decisions/002`](002_a_trait_because_the_criteria_needed_two.md) | The other decision the module comment records, and the criteria behind it |
| [`algorithm/002`](../algorithm/002_counts_is_four_reads_not_one.md) | The tear these nine literals permit |
| [`pitfall/001`](../pitfall/001_the_snapshot_that_never_happened.md) | The same, read as what a caller runs into |
| [`invariant/002`](../invariant/002_every_increment_survives.md) | The property `Relaxed` does preserve, and which the suite tests |

### Sources

| Fact | Where |
|------|-------|
| The `named, not defaulted` paragraph | `ring_atomic/src/lib.rs:13-17` |
| The concentration argument above it | `ring_atomic/src/lib.rs:8-11` |
| The nine literals | Census above |
| `SeqCst` does not close the window | Release probe, five runs each, quoted above |

### Tests

| Test | Covers |
|------|--------|
| `counts_are_exact_under_contention` | That `Relaxed` loses no increment — the property ordering does govern |
| *(to create)* | Nothing asserts that the counters are `Relaxed` deliberately, so the choice is invisible to both a reader and a regression |
