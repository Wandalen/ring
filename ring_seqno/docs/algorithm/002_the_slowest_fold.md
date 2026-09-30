# Algorithm: The `slowest` Fold

### Scope

- **Purpose**: Describe the one non-arithmetic computation in the crate, and account for the four tiers it crossed without being copied — and for the fork that ended that run.
- **Responsibility**: Give the fold, its identity question, and the four-tier chain that carried it, state the mechanism that kept the chain single-sourced for as long as it held, and record what broke it.
- **In Scope**: `ring_seqno::slowest` and every wrapper over it.
- **Out of Scope**: The four binary readings — see [`001`](001_four_readings_of_one_subtraction.md).

### The Fold

```rust
// ring_seqno/src/lib.rs:132-136
#[ must_use ]
pub fn slowest( cursors : &[ Seq ] ) -> Option< Seq >
{
  cursors.iter().copied().min()
}
```

Three tokens of computation. Everything interesting about it is in the return
type and in who calls it.

### It Is One of Two `.min()` Folds in Thirty-Three Crates

```sh
cd "$(git rev-parse --show-toplevel)"
grep -r '\.min()' ring_*/src/*.rs
# → two lines: this crate's fold, and ring_cursor's own
```

Live output:

```
ring_cursor/src/lib.rs:    cursors.iter().map(|c| c.load(GATING)).min()
ring_seqno/src/lib.rs:    cursors.iter().copied().min()
```

**Finding SQ4.** Two folds, and both functions are named `slowest`. For four
tiers this was the family's one uncopied shared decision, and against the rest
of the family that was a genuine anomaly: the cache-line size forked
(`ring_mpsc:860` reimplements `ring_align:138` with a strictly stronger
predicate); the gating `Acquire` decision is restated four times across three
crates; three ordering-constant names are each used by two crates. The fold did
none of that — until `b7e075ca`, which severed tier 2's call to tier 1 and left
the reduction written out twice. The distinction still worth drawing is that
those other forks were drift, and this one was a decision with a recorded
reason; see *The Fold That Was Reimplemented After All* below, and **SQ35** one
directory over.

### The Chain That Carries It

| Tier | Item | Signature | Body |
|:----:|------|-----------|------|
| 1 | `ring_seqno::slowest` | `( &[ Seq ] ) -> Option< Seq >` | `.iter().copied().min()` |
| 2 | `ring_cursor::slowest` | `( &[ PaddedCursor ] ) -> Option< Seq >` | load each at `GATING` into a `Vec< Seq >`, then tier 1 |
| 3a | `ring_barrier::Barrier::frontier` | `( &self ) -> Option< Seq >` | `ring_cursor::slowest( self.dependencies )` |
| 3b | `ring_gating::GatingSet::slowest` | `( &self ) -> Option< Seq >` | `ring_cursor::slowest( &self.cursors )` |

```sh
cd "$(git rev-parse --show-toplevel)"
command grep -m1 -A4 -F 'pub fn slowest( cursors : &[ PaddedCursor ] ) -> Option< Seq >' ring_cursor/src/lib.rs
command grep -m1 -A3 -F '  pub fn frontier( &self ) -> Option< Seq >' ring_barrier/src/lib.rs
command grep -m1 -A3 -F '  pub fn slowest( &self ) -> Option< Seq >' ring_gating/src/lib.rs
```

Live output:

```
pub fn slowest( cursors : &[ PaddedCursor ] ) -> Option< Seq >
{
  cursors.iter().map( | c | c.load( GATING ) ).min()
}

  pub fn frontier( &self ) -> Option< Seq >
  {
    ring_cursor::slowest( self.dependencies )
  }
  pub fn slowest( &self ) -> Option< Seq >
  {
    ring_cursor::slowest( &self.cursors )
  }
```

Each tier's body is one call to the tier below, exactly as the table's last
column says.

### Why This One Did Not Fork for Four Tiers

The mechanism is visible in the signature column above: **the argument type
changes at every tier.**

- Tier 1 takes plain `Seq` values.
- Tier 2 takes `PaddedCursor`s, which must be *loaded* before they are values — and loaded at a specific ordering, which is the decision `ring_cursor` owns.
- Tier 3 takes `&self`, because the slice is a field.

Each of those is a reason to write a wrapper. Nobody at tier 2 could call tier 1
directly — the types do not line up — so they wrote an adapter, and the adapter
called down instead of reimplementing. The same at tier 3.

The mechanism held for as long as adapting was cheaper than reimplementing. It
stopped holding when it wasn't: the adapter's own cost — a `Vec` per call, in a
CAS retry loop — came to exceed the three tokens it was adapting to, and tier 2
inlined the fold rather than keep paying it. A type change is a reason to wrap,
not a guarantee against copying.

Now the contrasting case. `ring_align::on_distinct_lines( a : usize, b : usize )`
takes two plain integers and returns a `bool`. Every prospective caller in the
family already has two `usize` addresses. There is no adaptation to perform, so
nobody wrote a wrapper — and `ring_mpsc`, which does not depend on `ring_align`
at all, wrote its own three-line version instead.

**A free function crosses a manifest boundary when each tier has a reason to
wrap it, and a type change is what supplies that reason.** The function that
needed adaptation survived intact through four crates. The function that was
directly callable acquired a divergent second copy.

This sharpens the rule stated in
[`ring_cursor` `integration/002`](../../../ring_cursor/docs/integration/002_who_reads_a_cursor.md),
which observed that a decision crosses on a *method* and not as a free function
or a constant. `slowest` is a free function that crossed anyway; the wrapping
chain is how.

### The Identity Question

`min()` over an empty slice has no answer, and the crate declines to invent one:

> Returning `None` rather than `Seq::ZERO` for an empty set keeps "no consumers"
> distinguishable from "a consumer at the start" — the two call for opposite
> decisions, since an ungated ring may publish freely.

That is not a stylistic preference, and the two tier-3 consumers prove it by
resolving the same `None` to opposite values:

| Consumer | `None` means | Resolves to | Source |
|----------|--------------|-------------|--------|
| `GatingSet::headroom` | nobody is reading, so nothing can be lost | `capacity.get()` — **full headroom** | `ring_gating:224` |
| `Barrier::available` | nothing has been published, so nothing is readable | `0` — **nothing** | `ring_barrier:218` |

A fold returning `Seq::ZERO` would have handed both of them `0`, which is right
for the barrier and catastrophically wrong for the gating set: a producer would
believe a ring nobody reads is permanently full. See
[`decisions/001`](../decisions/001_none_rather_than_zero_for_an_empty_set.md)
and [`type/002`](../type/002_the_option_that_slowest_returns.md).

### The Cost Was Paid One Tier Up, and Is Not Paid at All Now

`slowest` itself allocates nothing, and never did. Tier 2 did:

```rust
// ring_cursor::slowest in ring_cursor/src/lib.rs, before commit b7e075ca
let positions : Vec< Seq > = cursors.iter().map( | c | c.load( GATING ) ).collect();
```

That `Vec` was heap-allocated on every call, and the call sits in the **loop
condition** of `ring_claim::claim`'s compare-exchange retry — recorded as F1 in
[`ring_cursor` `nfr/002`](../../../ring_cursor/docs/non_functional_requirement/002_the_gating_read_allocates_nothing.md).
It was noted here because the signature *this* crate chose is what made the
collection necessary: `&[ Seq ]` cannot be produced from `&[ PaddedCursor ]`
without materialising the loads somewhere.

This section proposed the fix that was not taken. A `slowest` taking an iterator
would have removed the allocation without changing this crate's semantics at all
— and it was declined here because it changes tier 2's signature and needs its
own verification run. What happened instead is that tier 2 stopped calling tier 1:
`ring_cursor::slowest` is now `cursors.iter().map( | c | c.load( GATING ) ).min()`,
which allocates nothing because it folds the values as it loads them, and reaches
nothing in this crate to do it.

**That is a worse outcome for this crate than the alternative it declined**, and
worth stating plainly rather than reading as a fix that landed. The iterator
signature would have kept one definition of *what the minimum of a set of
positions means*. Severing the call duplicated it: the identical reduction now
exists twice, four lines apart across a crate boundary, with nothing keeping the
two equal. `ring_seqno::slowest` has no caller outside its own tests as a result —
see [`integration/002`](../integration/002_how_the_fold_crossed_four_tiers.md),
whose whole subject is the chain this severed. The allocation is gone either way;
so is the tiering that made this crate worth depending on for this function.
See [`non_functional_requirement/001`](../non_functional_requirement/001_every_reading_is_allocation_free.md).

### The Only Function Here That Cannot Be `const`

Four of this crate's five functions are const-legal and none is declared `const fn`
— [`item/001`](../item/001_the_three_capacity_readings.md) records that with the
compile that establishes it. `slowest` is the genuine exception: `Iterator::min`
is not a `const` operation, so it could not carry the annotation even if the
other four did.

### SQ4 — The Fold That Was Reimplemented After All

This was once the one shared decision nobody had copied. It is not any more:

```
ring_cursor/src/lib.rs:   cursors.iter().map( | c | c.load( GATING ) ).min()
ring_seqno/src/lib.rs:     cursors.iter().copied().min()
```

**Finding.** Two `.min()` folds now exist across the 33 crates, and both functions are named `slowest`. The fork was deliberate and is recorded one directory over as **SQ35**: `ring_cursor::slowest` used to delegate here, and `b7e075ca` made it fold the atomic loads in place instead — the only way to delete the heap allocation this crate's `&[ Seq ]` parameter forced on it inside a lock-free retry loop. The two bodies differ by exactly that load: `map( | c | c.load( GATING ) )` where this one has `copied()`. What the pair now shows is not a decision that held, but the cost of a parameter type — and the fact that this crate's `slowest` has had no caller outside its own tests ever since.

---

### Algorithms

| File | Relationship |
|------|--------------|
| [001_four_readings_of_one_subtraction.md](001_four_readings_of_one_subtraction.md) | The four that are one subtraction |

### Decisions

| File | Relationship |
|------|--------------|
| [../decisions/001_none_rather_than_zero_for_an_empty_set.md](../decisions/001_none_rather_than_zero_for_an_empty_set.md) | The identity question in full |

### Integrations

| File | Relationship |
|------|--------------|
| [../integration/002_how_the_fold_crossed_four_tiers.md](../integration/002_how_the_fold_crossed_four_tiers.md) | The chain as a dependency question, with the counter-case |

### Items

| File | Relationship |
|------|--------------|
| [../item/002_the_two_readings_without_a_capacity.md](../item/002_the_two_readings_without_a_capacity.md) | `slowest`'s signature, constness and coverage |

### Non-Functional Requirements

| File | Relationship |
|------|--------------|
| [../non_functional_requirement/001_every_reading_is_allocation_free.md](../non_functional_requirement/001_every_reading_is_allocation_free.md) | Where the allocation actually is |

### Types

| File | Relationship |
|------|--------------|
| [../type/002_the_option_that_slowest_returns.md](../type/002_the_option_that_slowest_returns.md) | What the `Option` promises a caller |

### Sources

| File | Relationship |
|------|--------------|
| `ring_seqno/src/lib.rs:117-136` | The fold and its doc |
| `ring_cursor/src/lib.rs:120-123` | Tier 2, and the `Vec` |
| `ring_barrier/src/lib.rs:191-194` | Tier 3a |
| `ring_gating/src/lib.rs:197-200` | Tier 3b |
| `ring_gating/src/lib.rs:224` | `None` → full headroom |
| `ring_barrier/src/lib.rs:218` | `None` → nothing readable |
| `ring_align/src/lib.rs:138` | The counter-case: a free function nobody wrapped |
| `ring_mpsc/src/lib.rs:860-866` | What happened to it |

### Tests

| File | Relationship |
|------|--------------|
| `tests/seq_test.rs:113-121` | The minimum, wherever it sits in the slice |
| `tests/seq_test.rs:125-130` | `None` is not `Some( ZERO )` |
| `ring_seqno/src/lib.rs:125-131` | The doctest — `slowest`'s only coverage in this crate besides the two above |
