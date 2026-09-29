# Seq::advanced_by

## Representation

Returns this sequence plus `n`. **The family's actual step operation** — 14
production call sites across 7 crates, seven times
[`next`](007_seq_next.md)'s two.

**Every one of the 14 is computing the end of a range**, and that is why the
count is what it is. A ring's write path does not advance a position; it claims
`count` slots, drains `max` items, or indexes `offset` into a batch, and each of
those is `start.advanced_by( k )`. The operation the type makes cheap is exactly
the operation the family performs.

The clearest evidence is two crates independently arriving at the same
three-line function — and disagreeing on one detail:

```sh
cd "$(git rev-parse --show-toplevel)"
command grep -A3 -F 'pub const fn end( self ) -> Seq' ring_claim/src/lib.rs
command grep -A3 -F 'pub const fn end( self ) -> Seq' ring_consume/src/lib.rs
```

Live output:

```
  pub const fn end( self ) -> Seq
  {
    self.start.advanced_by( self.len as u64 )
  }
  pub const fn end( self ) -> Seq
  {
    self.start.advanced_by( self.len )
  }
```

`ring_claim::Claim` and `ring_consume::Available`. Neither crate depends on the
other; both signatures are `pub const fn end( self ) -> Seq`; both exist because
a claim and an availability are the same shape of thing.

**They differ in the width of the length they hold.** `Claim` stores `len :
usize` and casts at the call; `Available` stores `len : u64` and does not. The
cast is where the divergence shows up, and the reason it exists is that a claim's
length is a slot count — bounded by `Capacity`, naturally `usize` — while an
availability's is a sequence distance, which is what
[`distance_to`](009_seq_distance_to.md) returns and is `u64`. Both are right for
their own crate. **The parameter type here is `u64`, so the crate whose count is
`usize` is the one that pays** — a consequence of this function's signature, not
of either caller's design.

## Kind

Associated Function/Method (§ Item Kind Taxonomy : Associated Item Kinds #1)

## Definition

`ring_types/src/id.rs:65`

```rust
#[ must_use ]
pub const fn advanced_by( self, n : u64 ) -> Self
```

Body is `Self( self.0 + n )` (`id.rs:67`).

**The one function in the crate taking a parameter that is not `self`**, and the
parameter is a bare `u64` rather than a newtype. So `seq.advanced_by( capacity
)` compiles, and it is not obviously wrong — a lap *is* a legitimate distance —
which is the loosest point in an otherwise tightly-typed position vocabulary
(→ [`../../pattern/002`](../../pattern/002_a_newtype_that_makes_a_check_unnecessary.md),
whose argument this function is the exception to).

**Its doc comment used to say nothing about overflow at all.** `next`'s used to
say the wrong thing (→ [`../../pitfall/001`](../../pitfall/001_seq_next_wraps_where_its_doc_says_it_saturates.md));
this one was silent, and the silence covered a wider hazard: `next` can only
overflow from `u64::MAX`, while `advanced_by` overflows from anywhere given a
large enough `n`, and `n` is caller-supplied. Both have since been corrected —
this one now states that the reachability argument does not carry over from
`next`, because `n` is caller-supplied and can reach the wrap point in a single
call.

## File Usage

| File | Line(s) | Context |
|------|---------|---------|
| `ring_types/src/id.rs` | 50, 52-57, 59-63, 64-65, 67 | Doc summary — one line, no `# Panics` (50); the overflow paragraph, added since (52-57); doc example covering `+5` and the `+0` identity (59-63); `#[ must_use ]` and **the definition (64-65)**; the body (67) |

Test-only references: `ring_types` — 4 in `tests/types_test.rs`.

## Crate Usage

| Crate | Via File | Purpose | Sites |
|-------|----------|---------|-------|
| `ring_types` | `src/id.rs` | Defining crate | — |
| `ring_mpsc` | `src/lib.rs` | `Batch::sequences`, `get`, `get_mut`, `drop` — offset arithmetic into a drained batch | 4 |
| `ring_claim` | `src/lib.rs` | `Claim::end`; `claim` and `claim_up_to` computing the new producer position | 3 |
| `ring_spsc` | `src/lib.rs` | `Batch::get`, `get_mut`, `drop` | 3 |
| `ring_consume` | `src/lib.rs` | `Available::end` — the same three lines as `ring_claim`'s, minus the cast | 1 |
| `ring_gating` | `src/lib.rs` | `Gate::limit` — how far a producer may run before the gate stops it | 1 |
| `ring_index` | `src/lib.rs` | `run( start, count, capacity )` — folds a whole range to slot indices | 1 |
| `ring_publish` | `src/lib.rs` | `try_publish( start, len )` — the end the publisher is claiming | 1 |

```sh
cd "$(git rev-parse --show-toplevel)"
command grep -rn '\.advanced_by(' ring_*/src | command grep -v '^ring_types/' \
  | command grep -v ':[0-9]*: *//' | sed 's|ring/||;s|/src.*||' | sort | uniq -c
```

Live output:

```
      3 ring_claim
      1 ring_consume
      1 ring_gating
      1 ring_index
      4 ring_mpsc
      1 ring_publish
      3 ring_spsc
```

**The two ring implementations account for half the sites, and both spend them
the same way.** `ring_mpsc` and `ring_spsc` each have a `Batch` whose `get`,
`get_mut` and `Drop` all need `start + offset`; `ring_mpsc` additionally exposes
`sequences()`, an iterator that generates them. Neither crate depends on the
other, so this is the same design solved twice — the same relationship
`Claim::end` and `Available::end` have.

## Caller Tree

- *No caller within `ring_types`*
- *External: `ring_claim::Claim::end`* (`:146`), *`ClaimSite::claim`* (`:406`), *`claim_up_to`* (`:451`)
- *External: `ring_consume::Available::end`* (`:143`)
- *External: `ring_gating::Gate::limit`* (`:305`)
- *External: `ring_index::run`* (`:88`)
- *External: `ring_mpsc::Batch::sequences`* (`:1199`), *`get`* (`:1214`), *`get_mut`* (`:1247`), *`Batch::drop`* (`:1270`)
- *External: `ring_publish::try_publish`* (`:163`)
- *External: `ring_spsc::Batch::get`* (`:1045`), *`get_mut`* (`:1105`), *`Batch::drop`* (`:1137`)

**Two of the fourteen are in a `Drop` impl** (`ring_mpsc:1270`,
`ring_spsc:1137`), the same placement [`next`](007_seq_next.md)'s second caller
has — batch destructors publish the position they consumed to.

## Callee Tree

- *(none)* — `u64::add`

**The most-called arithmetic function in the crate, with the least-specified
contract.** Fourteen callers, a caller-supplied `u64` that no type constrains,
and a doc comment with no `# Panics` section. Nothing in the family currently
passes a value large enough to matter, and nothing checks that it stays that way
(→ [`../../invariant/002`](../../invariant/002_tier_zero_depends_on_nothing.md)
for the crate's other unenforced property).
