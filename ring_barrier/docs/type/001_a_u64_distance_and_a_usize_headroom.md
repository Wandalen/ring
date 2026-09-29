# Type: A `u64` Distance and a `usize` Headroom

### Scope

- **Purpose**: Show what the `u64` in this crate's signatures is a width *of*, and why the crate is the only one in the family that never converts between the two integer widths.
- **Responsibility**: Classify every `usize` and `u64` in the source, locate the family's real narrowing sites, and say where the width G15 identifies actually resolves.
- **In Scope**: Integer width, and what each width measures.
- **Out of Scope**: The disagreement itself as `ring_gating` argues it — see [`ring_gating`'s type/001](../../../ring_gating/docs/type/001_a_usize_headroom_and_a_u64_limit.md) § G15.

### Every Integer in the Crate

Six occurrences of a width, in three methods each, and none of them ambiguous:

```sh
cd "$(git rev-parse --show-toplevel)"
grep -vE '^\s*(///|//!|//)' ring_barrier/src/lib.rs | grep -E 'usize|u64'
```

Live output:

```
  pub const fn len( &self ) -> usize
  pub fn cursor( &self, index : usize ) -> Option< &'a PaddedCursor >
  pub fn available( &self, from : Seq ) -> u64
  pub fn admits( &self, from : Seq, count : u64 ) -> bool
  pub fn wait_for( &self, from : Seq, count : u64, kind : WaitKind, spins : usize )
```

| Width | Where | Measures |
|-------|-------|----------|
| `usize` | `len( &self ) -> usize` | **cursors** — a slice length |
| `usize` | `cursor( &self, index : usize )` | **a cursor** — a slice index |
| `usize` | `wait_for( …, spins : usize )` | **attempts** — a loop budget |
| `u64` | `available( &self, from : Seq ) -> u64` | **sequences** |
| `u64` | `admits( &self, from : Seq, count : u64 )` | **sequences** |
| `u64` | `wait_for( …, count : u64, … )` | **sequences** |

The split is clean and it is not a coincidence: **every `usize` here counts
something about the barrier, every `u64` counts something about the ring.** Not
one `usize` is a quantity of items. That is what makes the crate width-pure —
there is no place where a count of sequences and a count of anything else meet.

### The Chain That Produces the `u64`

```rust
// ring_barrier/src/lib.rs:216-219
self.frontier().map_or( 0, | frontier | from.distance_to( frontier ) )
```

```rust
// ring_types/src/id.rs:82-85
pub const fn distance_to( self, later : Self ) -> u64
{
  later.0.saturating_sub( self.0 )
}
```

`Seq` is a `u64` newtype (`ring_types/src/id.rs:25`), so a distance between two
of them is a `u64`, and there is no capacity anywhere in the expression to
suggest otherwise ([`invariant/002`](../invariant/002_capacity_never_enters_the_arithmetic.md)).
The producer side ends in `ring_seqno::free_slots`, which subtracts from a
`Capacity( usize )` (`ring_types/src/capacity.rs:23`) and so returns
`usize`.
Each half is internally consistent; the widths differ because the *inputs*
differ.

The `saturating_sub` is worth naming separately. With no capacity in the
arithmetic, it is the **only** clamp in this crate's entire read path — a
consumer somehow ahead of the frontier gets `0` rather than a wrapped
`u64::MAX`. `ring_gating` has two clamps for the same reason it has a capacity;
this crate has one because it has neither.

### BR21 — The Only Crate in the Family That Never Casts

```sh
cd "$(git rev-parse --show-toplevel)"
for f in ring_*/src/*.rs; do
  n=$( grep -vE '^\s*(///|//!|//)' "$f" | grep -cE 'as usize|as u64' )
  [ "$n" != 0 ] && echo "$n  $f"
done
# The command above can legitimately exit nonzero -- a count of zero, a diff
# that differs -- and the sweep that keeps this output current skips any block
# that exits nonzero, so without this the block freezes instead of refreshing.
:
```

Live output:

```
3  ring_batch/src/lib.rs
5  ring_bench/src/lib.rs
3  ring_claim/src/lib.rs
1  ring_cursor/src/lib.rs
1  ring_debug/src/lib.rs
1  ring_gating/src/lib.rs
2  ring_index/src/lib.rs
7  ring_mpsc/src/lib.rs
1  ring_publish/src/lib.rs
3  ring_seqno/src/lib.rs
8  ring_spsc/src/lib.rs
2  ring_testkit/src/lib.rs
1  ring_trace/src/lib.rs
```

Thirteen source files cast between the two widths, 38 casts in all — `ring_spsc`
8, `ring_mpsc` 7, `ring_bench` 4, and so on down. `ring_barrier` is not among
them: its only textual `as u64` is inside a doctest at `:186`, and its code lines
contain zero casts in either direction.

That is not merely tidy. The casts the family does perform split into two very
different kinds:

| Kind | Site | Safe because |
|------|------|--------------|
| **Masked narrowing** | `ring_index:51` — `SlotIndex( ( seq.0 as usize ) & capacity.mask() )` | The mask is `< capacity`, so the truncated high bits could not have survived it anyway |
| **Widening** | `ring_seqno:52,75`, `ring_gating:323`, `ring_claim:146,439,490`, … | `usize → u64` never loses on any supported target |
| **Bare narrowing** | `ring_seqno:98`, `ring_mpsc:1056,1126`, `ring_spsc:861,920,948` | **Nothing.** Each is `distance_to( … ) as usize` — a `u64` sequence distance truncated to a machine word |

The six bare narrowings are all the same shape, and all of them are what
`ring_gating`'s G15 means by "the cost is at the joint": a distance in sequence
space forced into a slot-space width. On a 64-bit target they are free; on a
32-bit one, a ring more than 4 G sequences into its life truncates.

**This crate is upstream of every one of them and performs none.** `ring_consume`,
the only production holder of a `Barrier`, keeps the width too — `Available` is
`{ start : Seq, len : u64 }` (`ring_consume:99-103`). So the barrier path stays
`u64` from the cursor load all the way to the consumer's run, and narrows exactly
once, at `ring_index:42`, under a mask. The two paths that narrow *without* a
mask are `ring_spsc` and `ring_mpsc` — the two rings that do not use a `Barrier`
at all ([`non_functional_requirement/001`](../non_functional_requirement/001_every_frontier_read_allocates_nothing.md)).

That is the useful reading of G15 from this side: the width disagreement is real,
but the barrier half is the half that does not have to resolve it, and the
crates that resolve it wrongly are the ones on the other route.

### What Would Break the Purity

| Change | Introduces |
|--------|------------|
| `available -> usize` | A bare narrowing here, at the top of the chain, where it is least visible |
| `len -> u64` | A widening now, a narrowing at every `self.dependencies[ i ]` |
| A `capacity` parameter | `usize` into the arithmetic, and with it B1's whole failure mode ([`workaround/001`](../workaround/001_the_check_that_capacity_stays_out.md)) |
| `spins : u64` | A narrowing at the `ring_wait::wait_until` boundary, which takes `usize` |

The last row is the one worth pausing on: `spins` is `usize` not because attempts
are memory-shaped but because `wait_until`'s `0..spins.max( 1 )` is a plain range
counter (`ring_wait:183`). It is a borrowed width, and it is the one `usize` here
whose justification is a downstream signature rather than a domain fact.

### BR48 — The Crate Returns Two Counts in Two Widths and Converts Neither

`available` returns `u64`. `len` returns `usize`. Both are counts, they are
never compared, and no line the crate compiles casts either one — which BR21
records as a family-wide contrast, thirteen source files carrying thirty-eight
casts and this one carrying none. The single ` as u64` the file contains is
inside a `///` example, written for a caller holding a loop counter, and the
census below excludes it deliberately rather than by accident.

The reason it needs none is that the two counts never meet: `len` counts
dependencies and `available` counts sequences, and no expression in the crate
takes both. That is a real property and it is fragile in a specific way — a
future method answering *how many sequences per dependency*, or bounding a batch
by the dependency count, would introduce the family's thirty-ninth cast into
the one crate whose type story is that it has none.

```sh
cd "$(git rev-parse --show-toplevel)"
# -e is required here: the pattern begins with a hyphen, and without it grep
# reads `->` as an option bundle, fails, and this census silently never runs.
grep -E -e "-> (u64|usize)" ring_barrier/src/lib.rs
grep -E -e " as (u64|usize)|try_into|from\(" ring_barrier/src/lib.rs \
  | grep -vE '^ *///' \
  || echo '(no cast or conversion in code)'
printf 'the same expression inside doc examples: %s\n' \
  "$( grep -cE '^ *///.* as (u64|usize)' ring_barrier/src/lib.rs )"
# control: the identical expression over the two crates that cast most
grep -cE " as (u64|usize)" ring_spsc/src/lib.rs ring_mpsc/src/lib.rs
```

Live output:

```
  pub const fn len( &self ) -> usize
  pub fn available( &self, from : Seq ) -> u64
(no cast or conversion in code)
the same expression inside doc examples: 1
ring_spsc/src/lib.rs:8
ring_mpsc/src/lib.rs:7
```

### Types

| File | Relationship |
|------|--------------|
| [002_the_traits_derived_and_the_traits_absent.md](002_the_traits_derived_and_the_traits_absent.md) | The other half of the type's declaration |
| [`ring_gating`'s type/001](../../../ring_gating/docs/type/001_a_usize_headroom_and_a_u64_limit.md) | G15, from the side that carries the capacity |

### APIs

| File | Relationship |
|------|--------------|
| [../api/001_nine_methods_over_one_borrowed_slice.md](../api/001_nine_methods_over_one_borrowed_slice.md) | The nine signatures these widths appear in |

### Algorithms

| File | Relationship |
|------|--------------|
| [../algorithm/001_the_frontier_in_two_delegations.md](../algorithm/001_the_frontier_in_two_delegations.md) | Where `distance_to` sits in the chain |

### Integrations

| File | Relationship |
|------|--------------|
| [../integration/002_the_dependency_that_is_not_ring_seq.md](../integration/002_the_dependency_that_is_not_ring_seq.md) | Why `ring_seqno`, and its `usize` results, are not on this path |

### Invariants

| File | Relationship |
|------|--------------|
| [../invariant/002_capacity_never_enters_the_arithmetic.md](../invariant/002_capacity_never_enters_the_arithmetic.md) | The absence that makes the width `u64` |

### Non-Functional Requirements

| File | Relationship |
|------|--------------|
| [../non_functional_requirement/001_every_frontier_read_allocates_nothing.md](../non_functional_requirement/001_every_frontier_read_allocates_nothing.md) | The two rings on the other route |

### Patterns

| File | Relationship |
|------|--------------|
| [../pattern/002_the_quantity_the_predicate_and_the_wait.md](../pattern/002_the_quantity_the_predicate_and_the_wait.md) | BR11 — the same rung, three return types |

### Workarounds

| File | Relationship |
|------|--------------|
| [../workaround/001_the_check_that_capacity_stays_out.md](../workaround/001_the_check_that_capacity_stays_out.md) | The check that keeps the `usize` out |

### Sources

| File | Relationship |
|------|--------------|
| `ring_types/src/id.rs:25,82-85` | `Seq`, and `distance_to`'s `saturating_sub` |
| `ring_types/src/capacity.rs:23` | `Capacity( usize )`, the other width's origin |
| `ring_index/src/lib.rs:51` | The one masked narrowing on the barrier path |
| `ring_consume/src/lib.rs:99-103` | `Available`, which keeps the `u64` |

### Tests

| File | Relationship |
|------|--------------|
| `tests/barrier_test.rs:234-246` | `available` over four `from` values, all `u64` |
| `tests/barrier_test.rs:248-263` | The two widths asserted in one test |
