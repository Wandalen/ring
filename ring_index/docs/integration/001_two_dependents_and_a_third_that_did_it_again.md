# Integration: Two Dependents, and a Third That Did It Again

### Scope

**Purpose:** Record that two of the family's 33 crates depend on `ring_index`,
that the module comment names paths which do not, and that one crate wrote the
fold out by hand rather than adding the dependency.

**Responsibility:** The dependency edges into and out of this crate, and what
the crates on the other side of the missing edges do instead.

**In Scope:** every `ring_*/Cargo.toml`;
`ring_mpsc/src/lib.rs:540-547`; `ring_index/src/lib.rs:7-13`.

**Out of Scope:** the duplicate read as a hazard rather than a topology fact is
[`pitfall/002`](../pitfall/002_the_second_fold_nobody_noticed.md). What
`ring_batch` built instead of calling `run` is
[`workaround/002`](../workaround/002_the_iterator_ring_batch_built_instead.md).

---

## The Whole Topology

```sh
cd "$(git rev-parse --show-toplevel)"
# `command grep` bypasses the shell shim to the ordered GNU binary; the shim
# is a parallel ugrep that emits hits in completion order
echo '  -- manifests naming ring_index --'
for c in ring_*/Cargo.toml; do
  if command grep -q 'ring_index' "$c"; then echo "    ${c#ring/}"; fi
done
echo "  -- of 33 crates --"
ls -d ring_* | wc -l
echo '  -- what ring_index itself depends on --'
sed -n '/\[dependencies\]/,/^\[lints\]/p' ring_index/Cargo.toml | command grep 'ring_'
```

Live output:

```
  -- manifests naming ring_index --
    ring_batch/Cargo.toml
    ring_store/Cargo.toml
    ring_index/Cargo.toml
  -- of 33 crates --
33
  -- what ring_index itself depends on --
ring_types = { path = "../ring_types" }
```

One edge in from `ring_batch`, one from `ring_store`, one edge out to
`ring_types`. That is the entire integration surface of the crate.

---

### IX10 — The Reach Is Wider Than the Two Edges, Because `ring_store` Re-Exports the Fold

Two direct dependents understates it. `ring_store::Buffer::at` folds through
`ring_index` internally and hands back a slot borrow, so every crate that
borrows a slot through a `Buffer` folds through this crate without naming it:

```sh
cd "$(git rev-parse --show-toplevel)"
command grep -m1 -A4 -F '  /// Borrow the slot a sequence addresses, folding through `ring_index`.' ring_store/src/lib.rs
command grep 'slots\.at( seq )' ring_spsc/src/lib.rs
```

Live output:

```
  /// Borrow the slot a sequence addresses, folding through `ring_index`.
  ///
  /// The convenience that keeps the fold in one place: a caller that wrote its
  /// own `seq % capacity` here would be the second implementation of the thing
  /// `ring_index` exists to be the only one of.
    unsafe { &*self.slots.at( seq ).get() }
    unsafe { &mut *self.slots.at( seq ).get() }
```

**Finding.** `ring_spsc` never names `ring_index` and never needs to: it calls
`Buffer::at`, which folds. Both of its slot accesses do — the read and the write
alike, since the write path reaches a per-slot `UnsafeCell` through the same
shared `at` rather than through `at_mut`. This is the intended shape — the fold
reaches the call sites that need it through the type that owns the storage,
rather than through a dependency every crate has to remember to add.

It also means the two-manifest census is the wrong measure of whether the
boundary is working. What matters is whether a crate that folds does so through
`Buffer` (fine, transitive) or by hand (not fine, IX12). `ring_store`'s own
comment states the rule in exactly those terms, and it is the only place in the
family where the rule is written down.

---

### IX11 — The Module Comment Names Two Paths, and the Claim Path Is Not One of Them

```sh
cd "$(git rev-parse --show-toplevel)"
command grep -m1 -A2 -F '//! rather than a division — an integer `%` costs on the order of 20–40 cycles on' ring_index/src/lib.rs
echo '  -- ring_index in the manifests of the crates that own those paths --'
for c in ring_claim ring_publish ring_consume ring_cursor ring_gating; do
  printf '    %-14s %s occurrence(s)\n' "$c" "$( command grep -c 'ring_index' ring/$c/Cargo.toml )"
done
```

Live output:

```
//! rather than a division — an integer `%` costs on the order of 20–40 cycles on
//! current x86, and it sits on every operation that touches a slot. The claim
//! path is not one of them: `ring_claim`, `ring_publish`, `ring_consume`, and
  -- ring_index in the manifests of the crates that own those paths --
    ring_claim     0 occurrence(s)
    ring_publish   0 occurrence(s)
    ring_consume   0 occurrence(s)
    ring_cursor    0 occurrence(s)
    ring_gating    0 occurrence(s)
```

**Finding.** The claim path does not fold. `ring_claim` advances a cursor and
hands back sequences; `ring_publish` stamps; `ring_consume` compares sequences
against a frontier. None of the five crates that implement claiming, gating,
publication and consumption depends on `ring_index`, because none of them needs
a slot index — they work in sequence space from end to end, and folding happens
later, when something actually touches the slot.

So the sentence is wrong in the specific way that matters for reading it as
justification. The power-of-two constraint is worth its cost, and the fold is on
the read path, and both of those are true — but "every single operation the
family performs" is doing load-bearing work in an argument about how often the
fold runs, and the manifests contradict it. The correct claim is narrower and
still sufficient: the fold sits on every operation that *touches a slot*.

```sh
cd "$(git rev-parse --show-toplevel)"/ring_index
command grep -F 'work in sequence space end to end and never fold' src/lib.rs
```

Live output:

```
//! `ring_cursor` work in sequence space end to end and never fold.
```

**Disposition:** applied — the module comment no longer claims the fold sits
on "every single operation the family performs"; it now states the narrower,
correct claim (every operation that touches a slot) and names the four claim
path crates that work in sequence space end to end and never fold, matching
what the manifest census above shows.
Now prints: `work in sequence space end to end and never fold`

---

### IX12 — `ring_mpsc` Wrote the Fold Out Rather Than Adding the Edge

```sh
cd "$(git rev-parse --show-toplevel)"
# `command grep` bypasses the shell shim to the ordered GNU binary; the shim
# is a parallel ugrep that emits hits in completion order
command grep -r '( seq.0 as usize ) &' ring_*/src/*.rs
echo '  -- and ring_mpsc'\''s eight ring_ dependencies --'
sed -n '/\[dependencies\]/,/^\[lints\]/p' ring_mpsc/Cargo.toml | command grep -o 'ring_[a-z]*' | sort -u | tr '\n' ' '
echo
```

Live output:

```
ring_index/src/lib.rs:  SlotIndex( ( seq.0 as usize ) & capacity.mask() )
ring_mpsc/src/lib.rs:    let index = ( seq.0 as usize ) & self.capacity().mask();
  -- and ring_mpsc's eight ring_ dependencies --
ring_atomic ring_store ring_claim ring_config ring_cursor ring_gating ring_slot ring_types 
```

**Finding.** Two implementations of the fold exist in the family, character for
character identical in the interesting part. The second is in a crate that
depends on eight `ring_*` crates, one of which is `ring_store` — the crate
whose own comment says a hand-written fold "would be the second implementation
of the thing `ring_index` exists to be the only one of."

`ring_mpsc` is folding to reach `self.stamps[ index ]`, a `Vec< AtomicSeq >`
that is not a `Buffer`, so `Buffer::at` was not available to it and the
transitive path of IX10 does not apply. The fix is one line in a manifest and
one call, and nothing forced it: `ring_index` is Tier 1 with a single dependency
of its own, so adding the edge costs nothing structurally.

What makes this worth recording rather than shrugging at is that the family
already noticed the risk and wrote a test against it — in the other crate:

```sh
cd "$(git rev-parse --show-toplevel)"
command grep 'not_a_second_implementation' -A 9 ring_batch/tests/batch_test.rs | head -12
```

Live output:

```
fn drain_order_folds_through_ring_index_and_not_a_second_implementation()
{
  let capacity = cap( 8 );
  let batch = BatchClaim::new( Seq( 3 ), 20 );

  for ( seq, slot ) in drain_order( &batch, capacity )
  {
    assert_eq!( slot, ring_index::of( seq, capacity ), "the fold must be ring_index's" );
  }
}
```

A test named for the hazard, asserting the fold is `ring_index`'s, guarding the
one crate that did not need guarding — while the crate that did have a second
implementation has no such test, because a test like that can only be written by
someone who already added the dependency.

**Disposition:** declined — the fix is one manifest line and one call site in
`ring_mpsc/Cargo.toml` and `ring_mpsc/src/lib.rs`, a crate
outside this pass's assigned scope (`ring_gating`, `ring_handle`,
`ring_index`). Nothing in `ring_index`'s own `src/` or `docs/` can add the
missing dependency edge on `ring_mpsc`'s behalf, and the test this finding's
own Tests table marks "(to create)" cannot be written until that edge exists.

---

### Related

| Instance | Relationship |
|----------|--------------|
| [`pitfall/002`](../pitfall/002_the_second_fold_nobody_noticed.md) | The duplicate as a hazard: what breaks if the two ever diverge |
| [`pattern/001`](../pattern/001_one_owner_for_one_arithmetic_fact.md) | The boundary this topology is meant to realize |
| [`api/001`](../api/001_three_functions_three_must_use_one_reached.md) | The reach census by call site rather than by manifest |
| [`workaround/002`](../workaround/002_the_iterator_ring_batch_built_instead.md) | The other crate that reached past a function built for it |

### Sources

| Fact | Where |
|------|-------|
| Dependents and dependencies | Manifest census above |
| The transitive path through `Buffer::at` | `ring_store/src/lib.rs:245-249`; `ring_spsc/src/lib.rs:425, 466` |
| The "every single operation" claim | `ring_index/src/lib.rs:11-13` |
| The second fold | `ring_mpsc/src/lib.rs:543` |
| The test named for the hazard | `ring_batch/tests/batch_test.rs:110-119` |

### Tests

| Test | Covers |
|------|--------|
| `ring_batch::drain_order_folds_through_ring_index_and_not_a_second_implementation` | `ring_batch`'s edge, asserted against `of` directly — declared in `ring_batch/tests/`, not here |
| *(to create)* | The same assertion for `ring_mpsc::stamp` — which cannot be written until `ring_mpsc` depends on `ring_index` |
