# Data Structure: A Watch Is Three Scalars and No Identity

### Scope

- **Purpose**: Describe `Watch`'s layout, and record what its three fields do not include.
- **Responsibility**: The fields, their derives, and the consequences of both for a caller holding one.
- **In Scope**: `Watch`'s representation; the cached `capacity`; the derive list, and the `Copy` that is no longer in it.
- **Out of Scope**: The states a `Watch` moves between (→ [`lifecycle/001`](../lifecycle/001_from_one_observation_to_a_sequence.md)); the checks it performs (→ [`algorithm/001`](../algorithm/001_checking_a_pair_without_touching_it.md)).

### Abstract

`Watch` is three `Copy` scalars — two `Seq` and one `Capacity` — and nothing
else. Twenty-four bytes on this target, no allocation, no lifetime, no borrow of
the pair it was built from.

That last absence is the whole of this instance. `Watch::new` takes a
`&CursorPair` and keeps **none of it**: not the reference, not a pointer, not an
identifier. `Watch::observe` then takes another `&CursorPair` — any
`&CursorPair` — and compares it against the baseline. The type does not carry
which ring it is watching, and so cannot notice when it is handed a different
one.

### Data Structures

```rust
#[ derive( Debug, Clone, PartialEq, Eq ) ]
pub struct Watch
{
  producer : Seq,
  consumer : Seq,
  capacity : Capacity,
}
```

| Field | Type | Written by | Read by |
|---|---|---|---|
| `producer` | `Seq` | `new`, and `observe` **on success only** | `observe`'s D3 comparison; `last` |
| `consumer` | `Seq` | `new`, and `observe` **on success only** | `observe`'s D3 comparison; `last` |
| `capacity` | `Capacity` | `new` **only** | `observe`'s D2 comparison |

**`capacity` is the asymmetric one.** The two `Seq` fields are re-read from the
pair on every `observe`; `capacity` is read once, at `new`, and never again.
`observe` passes `self.capacity` to `check_seqs`, never `pair.capacity()` —
the recipe above prints all three sites, and only the two in `check` and `new`
read the pair's own.

For a `Watch` used the intended way — one watch, one pair, for the pair's
lifetime — that is not just harmless but correct: a ring's capacity does not
change, so re-reading it every observation would be a load that can only ever
return the same value. The caching is the right call for the intended use.

### Regenerate

```sh
cd "$(git rev-parse --show-toplevel)"
S=ring_debug/src/lib.rs
echo '-- the struct, its derive, and its fields --'
command grep -m1 -B1 -A6 -F 'pub struct Watch' $S
echo '-- which capacity does observe compare against? --'
command grep 'self\.capacity\|pair\.capacity()' $S
echo '-- does Watch borrow anything? a lifetime parameter would show here --'
printf 'lifetimes on the struct: %s\n' "$( command grep -cE '^pub struct Watch<' $S || true )"
echo '-- and is the mistake exercised anywhere in the suite? --'
T=ring_debug/tests/debug_test.rs
printf 'Watch::new call sites:              %s\n' "$( command grep -c 'Watch::new' $T )"
printf 'distinct pairs named in the file:   %s\n' "$( command grep -oE 'pair_at\(|CursorPair::new\(' $T | wc -l )"
printf 'sites forking a Watch:              %s\n' "$( command grep -cE '^ +let .*\.clone\(\);' $T || true )"
printf 'sites observing a foreign pair:     %s\n' "$( command grep -c 'observe( &foreign )' $T || true )"
```

Live output:

```
-- the struct, its derive, and its fields --
#[ derive( Debug, Clone, PartialEq, Eq ) ]
pub struct Watch
{
  producer : Seq,
  consumer : Seq,
  capacity : Capacity,
}

-- which capacity does observe compare against? --
  check_seqs( producer, consumer, pair.capacity() )
    let capacity = pair.capacity();
    check_seqs( producer, consumer, self.capacity )?;
-- does Watch borrow anything? a lifetime parameter would show here --
lifetimes on the struct: 0
-- and is the mistake exercised anywhere in the suite? --
Watch::new call sites:              11
distinct pairs named in the file:   30
sites forking a Watch:              1
sites observing a foreign pair:     1
```

### State Machines

| File | Relationship |
|------|--------------|
| [../lifecycle/001_from_one_observation_to_a_sequence.md](../lifecycle/001_from_one_observation_to_a_sequence.md) | The transitions these three fields encode |

### APIs

| File | Relationship |
|------|--------------|
| [../api/001_the_check_surface.md](../api/001_the_check_surface.md) | `Watch`'s three verbs, and precondition B3 |

### Sources

| File | Relationship |
|------|--------------|
| [`src/lib.rs`](../../src/lib.rs) | The struct and its impl block |

### Tests

| Test | Relationship |
|------|--------------|
| `a_failed_observation_leaves_the_baseline_alone` | The write-on-success-only rule, which is what makes the two `Seq` fields meaningful |
| `a_watch_refuses_to_baseline_a_broken_pair` | `new` checks before it constructs, so no `Watch` is born holding a state its own checker rejects |
| `a_watch_follows_an_ordinary_run_quietly` | The intended use — one watch, one pair — which is the case DB5 is *not* about |

### DB5 — a `Watch` has no identity, so observing the wrong pair is silent


`observe( &mut self, pair : &CursorPair )` accepts any pair. Give it a second
ring's pair and every comparison still runs: D3 against the *first* ring's
baseline sequences, D2 against the *first* ring's cached capacity, D1 against the
second ring's own two cursors.

Every one of those comparisons is well-defined, none of them panics, and the
result is a `Result< (), Violation >` indistinguishable from a real verdict about
a real ring. **The failure mode is not a wrong answer but a confident answer to a
question nobody asked** — and in a diagnostic crate that is the expensive kind,
because the caller reached for this type precisely because they did not trust
what they were looking at.

The cost of preventing it is the reason it was not prevented, and the trade is
worth stating rather than implying. Borrowing the pair (`Watch< 'a >` holding
`&'a CursorPair`) makes the mistake unrepresentable and makes `Watch` no longer
`Copy`, no longer storable beside the thing it watches without a self-referential
struct, and no longer usable in the family's ordinary shape where a test owns the
ring and hands out `&` on demand. **`Watch` is a value today and that is a real
property**; the exposure is the price.

No test exercised the mistake, which was the second half of the finding. The
suite built 25 pairs and 8 watches, and every `observe` was handed the same pair
its watch was built from — so the behaviour under a foreign pair was not merely
undocumented but unobserved. The recipe above now reads 28 and 10, and the three
pairs and two watches between those two readings are the two tests this finding
and DB6 produced.

**That half is now closed, and what it pins is worse than the prose predicted.**
`observing_a_foreign_pair_answers_about_neither_ring` builds a second, perfectly
healthy pair, confirms `check` passes it on its own terms, then hands it to a
watch baselined on the first. The result is
`CursorWentBackwards { cursor : Producer, was : Seq( 20 ), now : Seq( 4 ) }` — a
single report whose two sequences come from two different rings. It is not a
wrong number, it is two right numbers about different subjects, which is the
shape of report a reader has no way to distrust.

**Disposition:** declined — `Watch< 'a >` holding `&'a CursorPair` would make the
mistake unrepresentable, and is refused because it also makes `Watch` unstorable
beside the ring it watches without a self-referential struct, which is the shape
every consumer in `ring_debug/tests/debug_test.rs` uses. The exposure is pinned
instead by `observing_a_foreign_pair_answers_about_neither_ring`, so a change to
this behaviour now fails a test rather than passing unnoticed.

### DB6 — `Watch` is `Copy`, and a copy is a silently forked baseline


**What was found.** The `Copy` derive was inherited without comment: the three
fields are `Copy`, so the struct could be, so it was.

But `Watch`'s entire purpose is to be *the* record of what was seen last.
`let mut w2 = w1;` compiled, produced a second independent baseline, and looked
exactly like a move to a reader who had not checked the derive. From that point
the two watches diverge, each reporting D3 against its own history, and the one
left behind reports a backwards move for every subsequent observation of a
healthy ring — because its baseline is stale, not because anything moved
backwards.

**`Clone` alone keeps every legitimate use and makes the fork deliberate**, since
a forked baseline is occasionally what you want: checkpoint a watch before a
suspect phase, compare after. That is what the derive list now says. `let w2 =
w1;` moves; a fork costs a visible `.clone()`.

**The test that was missing is the one that demonstrates the hazard.**
`a_cloned_watch_forks_the_baseline` takes the checkpoint deliberately, lets the
live watch follow the ring forward, and then shows the stale fork reporting
`CursorWentBackwards` against a ring that never went backwards. The finding's
claim is now a test rather than a paragraph, and it exists only because making
the fork explicit made it worth writing down.

**Disposition:** applied — `Watch` derives `Clone` without `Copy`, so a second
baseline costs a visible `.clone()` and `let w2 = w1;` is a move; the divergence
the finding describes is pinned by `a_cloned_watch_forks_the_baseline`, and the
rustdoc's *Not `Copy`, deliberately* section records why `Clone` was kept.
Now prints: `#[ derive( Debug, Clone, PartialEq, Eq ) ]`
