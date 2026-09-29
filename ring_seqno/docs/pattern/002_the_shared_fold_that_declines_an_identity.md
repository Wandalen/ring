# Pattern: The Shared Fold That Declines an Identity

### Scope

- **Purpose**: Name the shape by which a fold shared between disagreeing consumers returns `Option` rather than choosing an identity for the empty case, and show the family's own evidence that the choice was right.
- **Responsibility**: State the shape, enumerate every site in the family that resolves this crate's one `None`, and show why the mathematically correct identity is the dangerous one.
- **In Scope**: `slowest`'s `Option` return as an instance of a reusable shape.
- **Out of Scope**: The decision record itself — see [`decisions/001`](../decisions/001_none_rather_than_zero_for_an_empty_set.md). The `Option` type's own mechanics — see [`type/002`](../type/002_the_option_that_slowest_returns.md).

### The Shape

> A fold with no natural identity — or with several, and consumers who need
> different ones — returns `Option` and lets each consumer resolve the empty
> case at its own call site. The shared crate owns the fold; it does not own the
> meaning of nothing.

The test for whether the shape applies is not "is there an identity?" but **"do
the consumers agree on it?"** A fold with a mathematically canonical identity
that two consumers would resolve differently still wants `Option`.

### The Instantiation Here

```rust
// ring_seqno/src/lib.rs:133-136
pub fn slowest( cursors : &[ Seq ] ) -> Option< Seq >
{
  cursors.iter().copied().min()
}
```

Four lines, and the only decision in them is the return type. `min()` over an
iterator already returns `Option`; the crate's contribution is *not unwrapping
it*.

### Five Resolutions of One `None`

This is the empirical case. Every site in all 33 crates that resolves the `None`
this fold produces:

```sh
cd "$(git rev-parse --show-toplevel)"
grep -r 'slowest()\|frontier()' ring_*/src/*.rs | grep -vE ':\s*(///|//!|//)'
```

Live output:

```
ring_barrier/src/lib.rs:    self.frontier().map_or( 0, | frontier | from.distance_to( frontier ) )
ring_barrier/src/lib.rs:    self.frontier().ok_or( RingError::Empty )
ring_consume/src/lib.rs:      .frontier()
ring_gating/src/lib.rs:    self.slowest().map_or( self.capacity.get(), | slowest |
ring_gating/src/lib.rs:    self.slowest().map( | s | s.advanced_by( self.capacity.get() as u64 ) )
```

| Site | Resolves `None` to | Reads as | Position in range |
|------|--------------------|----------|:-----------------:|
| `ring_gating:224` `headroom` | `self.capacity.get()` | unbounded — nobody is reading, write freely | **maximum** |
| `ring_gating:323` `limit` | `None`, propagated by `map` | still undecided; the caller above resolves it | — |
| `ring_barrier:218` `available` | `0` | nothing is published, read nothing | **minimum** |
| `ring_barrier:273` `wait_for` | `Err( RingError::Empty )` | not a quantity at all | **an error** |
| `ring_consume:342` | `0` | same as `available`, re-expanded inline | **minimum** |

**Four distinct resolutions across four crates: maximum, minimum, propagate, and
error.** A fold that had chosen any one of them would have been wrong at three of
the other sites.

`ring_barrier`'s module documentation states the disagreement in its own words and
resolves it — the two answers are not an inconsistency:

> [`Barrier::frontier`] returns `None` for a barrier with no dependencies, and
> [`Barrier::available`] then returns zero — the opposite of `ring_gating`, where
> an empty set means *unbounded*. The asymmetry is not an inconsistency: a
> producer with nobody reading behind it can write freely, while a consumer with
> nothing published in front of it has nothing to read. In both cases the empty
> set means "no constraint from dependencies", and in both cases that resolves to
> the value a dependency-free participant actually has available.

That last sentence is the pattern's justification in one line. The *rule* is
shared — "no constraint from dependencies" — and only its *numeric value* differs.
`Option` is exactly the shape that transmits the rule without the value.

### Why the Mathematically Correct Identity Is the Dangerous One

`min` has a canonical identity: `+∞`. The empty minimum is unambiguously the top
of the range, and a fold that returned `Seq( u64::MAX )` for an empty slice would
be defensible on paper and total.

Trace it through the two consumers:

| Consumer | With `Seq( u64::MAX )` as the empty answer | Verdict |
|----------|--------------------------------------------|---------|
| `ring_gating::headroom` | `free_slots( producer, u64::MAX, cap )` — the producer is far behind the "slowest consumer", so headroom saturates to full capacity | **Accidentally correct** |
| `ring_barrier::available` | `from.distance_to( u64::MAX )` — almost the entire `u64` range is reported readable | **Catastrophically wrong** |

The second row is the argument. A consumer told it may read ~2⁶⁴ sequences from a
barrier with no dependencies will read slots no producer has written — the
uninitialised-read failure the barrier exists to prevent, produced by the barrier
itself. And it would arrive silently: the arithmetic is total, the type is
correct, nothing panics.

Worse, the first row would have hidden it. `ring_gating` — the more heavily
exercised of the two — would keep passing its tests, so the identity would look
validated by the crate that happened not to care.

The pattern's real value is not that `Option` is more expressive. It is that
**`Option` makes the two consumers write down their disagreement**, at the two
lines where it matters, in a form a reader of either crate can see.

A supporting detail: `ring_types` declares `Seq::ZERO` and no `Seq::MAX`
(`grep -n 'const ZERO\|const MAX' ring_types/src/id.rs` returns one line).
The identity that would have been dangerous is not even nameable in the
vocabulary — a mild structural discouragement, though not a deliberate one.

### What `Option` Costs

| Cost | Detail | Paid where |
|------|--------|:----------:|
| Every consumer writes a resolution | Five sites, five `map_or`/`map`/`ok_or` calls | 5 lines |
| The resolutions can drift | `ring_consume:342` re-expands `available`'s body rather than calling it — two sites now own the same `0` (**Finding SQ20**) | already happened |
| A resolution can be wrong and compile | Nothing checks that `ring_barrier`'s `0` is right; the type only forces *a* choice | untested by construction |
| One unreachable-looking branch | `wait_for`'s `ok_or` looks dead — `admits` already gates it | see below |

The second row is the pattern's actual failure mode, observed rather than
hypothesised. Pushing a decision out to consumers means it is made *n* times, and
nothing keeps the *n* copies together. `ring_consume` inlining `available`'s
resolution is exactly that: the `0` is now written in two crates.

The fourth deserves a note, because it looks like dead code and is not.
`wait_for` gates on `admits( from, count )` before calling `frontier()`, so a
`None` frontier normally fails the gate first. But `admits( from, 0 )` is
`0 <= 0` — true on an empty barrier — so `wait_for( seq, 0, … )` on a
dependency-free barrier reaches the `ok_or` and returns `Err( RingError::Empty )`.
Reproduce it with `Barrier::over( &[] ).wait_for( Seq::ZERO, 0, WaitKind::None, 1 )`.
A reader tempted to replace the `ok_or` with an `expect` would be replacing a
correct error with a panic on a zero-length wait.

### Where the Pattern Does Not Apply

| Condition | Then |
|-----------|------|
| One consumer, or all consumers agree | Pick the identity in the fold. `Option` is then pure ceremony at every call site |
| The empty case is genuinely impossible | Take a non-empty type (`&[T; N]`, a `NonEmpty`) and delete the case rather than making callers handle it |
| The identity is enforced elsewhere | If a constructor already guarantees non-empty, the fold should say so in its signature, not in its return |

`ring_seqno` fails all three: it has four consumers, they disagree, empty slices are
real (`GatingSet::new( cap, 0 )` is constructed in tests and admitted by the API),
and nothing upstream forbids them.

### The Generalisation

Stated for the next crate that shares a fold:

> Before choosing an identity for an empty fold, list the consumers and write
> down what each would want. If the list has two distinct answers, return
> `Option`. If it has one, and you are confident the second consumer does not
> exist yet, return `Option` anyway — the cost is one `unwrap_or` at the single
> call site, and the alternative is a value baked into a shared crate that the
> second consumer cannot override.

The asymmetry of cost is the whole argument. Declining an identity costs one line
per consumer. Choosing the wrong one costs a rewrite of the shared crate, or —
as the `Seq( u64::MAX )` trace shows — an uninitialised read that the better-tested
consumer conceals.

### SQ41 — Two Consumers, Two Identities, One Fold

Had `slowest` picked an identity, one of these two would have been wrong:

```
ring_gating   None -> capacity   an ungated ring may publish freely
ring_barrier  None -> 0          a barrier with no dependencies blocks nothing... and reports zero
```

**Finding.** `ring_gating` and `ring_barrier` resolve the same `None` to `capacity` and `0` — the empirical proof the fold was right to decline an identity.

---

### SQ42 — Four Citations in Twenty-Three Lines

The module doc is short and heavily sourced, and one source is about the doc itself:

```
module doc lines: 23
external documents cited: 4
of which correct this crate's own earlier wording: 1
```

**Finding.** Twenty-three lines of module doc cite four external documents, one of which exists to correct this crate's own earlier description.

---

### Algorithms

| File | Relationship |
|------|--------------|
| [../algorithm/002_the_slowest_fold.md](../algorithm/002_the_slowest_fold.md) | The fold, and its four-tier chain |

### Data Structures

| File | Relationship |
|------|--------------|
| [../data_structure/002_the_slice_that_slowest_reads.md](../data_structure/002_the_slice_that_slowest_reads.md) | The parameter side of the same signature |

### Decisions

| File | Relationship |
|------|--------------|
| [../decisions/001_none_rather_than_zero_for_an_empty_set.md](../decisions/001_none_rather_than_zero_for_an_empty_set.md) | The decision this pattern generalises |

### Integrations

| File | Relationship |
|------|--------------|
| [../integration/002_how_the_fold_crossed_four_tiers.md](../integration/002_how_the_fold_crossed_four_tiers.md) | Finding SQ20 — the resolution `ring_consume` copied instead of calling |

### Patterns

| File | Relationship |
|------|--------------|
| [001_the_predicate_beside_its_quantity.md](001_the_predicate_beside_its_quantity.md) | The complementary shape — a decision kept in rather than pushed out |

### Types

| File | Relationship |
|------|--------------|
| [../type/002_the_option_that_slowest_returns.md](../type/002_the_option_that_slowest_returns.md) | `Option< Seq >` itself |

### Sources

| File | Relationship |
|------|--------------|
| `ring_seqno/src/lib.rs:133-136` | The fold that declines |
| `ring_gating/src/lib.rs:222-228` | Resolution to `capacity` — the maximum |
| `ring_gating/src/lib.rs:321-324` | Propagation rather than resolution |
| `ring_barrier/src/lib.rs:216-219` | Resolution to `0` — the minimum |
| `ring_barrier/src/lib.rs:259-267` | Resolution to an error, and the `count == 0` route to it |
| `ring_barrier/src/lib.rs:46-53` | The module doc that states the disagreement and resolves it |
| `ring_consume/src/lib.rs:338-344` | The fourth resolution, re-expanded inline |
| `ring_types/src/id.rs:30` | `Seq::ZERO` exists; `Seq::MAX` does not |

### Tests

| File | Relationship |
|------|--------------|
| `tests/seq_test.rs:125-130` | The empty slice returns `None` |
| `ring_gating/tests/gating_test.rs:182-197` | A consumer's resolution asserted at tier 3 |
| `ring_gating/tests/gating_test.rs:199-209` | The adversarial test separating an ungated ring from a gated one |
