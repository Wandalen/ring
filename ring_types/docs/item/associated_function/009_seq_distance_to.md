# Seq::distance_to

## Representation

Returns how many publications separate this sequence from a later one, or `0`
when `later` is not actually later. **11 production call sites across 4 crates**,
and the only arithmetic function in the crate that cannot overflow.

`later.0.saturating_sub( self.0 )` — the one place the crate reaches for a
checked operation rather than a bare operator, and the choice is the item's whole
story. [`next`](007_seq_next.md) and [`advanced_by`](008_seq_advanced_by.md) use
plain `+` and are documented (one wrongly, one not at all) as relying on the
overflow point being unreachable. This function does not rely on anything: an
argument in the wrong order returns `0` instead of wrapping to ~1.8×10¹⁹.

**That asymmetry is defensible and worth stating.** Overflow on `+` needs a
sequence near `u64::MAX`, which no workload reaches. Underflow on `-` needs only
the two arguments in the wrong order, which a caller can do on their first
attempt. The crate hardened the reachable failure and left the unreachable one to
a comment.

**The `0` is not an error value, and the doc says so directly:** "the caller that
needs the direction has already compared the two, and every caller that does not
wants a count." `Seq` is `Ord`, so a caller who cares writes `if a < b` first.

## Kind

Associated Function/Method (§ Item Kind Taxonomy : Associated Item Kinds #1)

## Definition

`ring_types/src/id.rs:82`

```rust
#[ must_use ]
pub const fn distance_to( self, later : Self ) -> u64
```

Body is `later.0.saturating_sub( self.0 )` (`id.rs:84`).

**The only function in the crate returning a raw `u64` rather than a newtype or
a `bool`.** A distance between two `Seq`s is deliberately not a `Seq` — it is a
count, and calling it a position would let it be compared against one. Nothing
enforces that distinction; the return type is plain and the discipline is in the
name.

**Receiver order is the trap and the signature encodes it.**
`earlier.distance_to( later )` reads correctly and `later.distance_to( earlier )`
returns `0` silently. The doc example asserts both directions —
`Seq( 4 ).distance_to( Seq( 10 ) ) == 6` and `Seq( 10 ).distance_to( Seq( 4 ) ) == 0`
— which is the only documentation the `0` convention gets.

## File Usage

| File | Line(s) | Context |
|------|---------|---------|
| `ring_types/src/id.rs` | 70-71, 73-74, 76-80, 81-82, 84 | Doc summary naming the `0` convention (70-71); the rationale for saturating rather than signed (73-74); doc example asserting both directions (76-80); `#[ must_use ]` and **the definition (81-82)**; the body (84) |

Test-only references: `ring_types` — 5 in `tests/types_test.rs`.

## Crate Usage

| Crate | Via File | Purpose | Sites |
|-------|----------|---------|-------|
| `ring_types` | `src/id.rs` | Defining crate | — |
| `ring_seqno` | `src/lib.rs` | `laps_between`, `may_claim`, `free_slots`, `pending` — four of the crate's five free functions | 4 |
| `ring_spsc` | `src/lib.rs` | `occupancy`, `available`, `drain`, `drain_up_to` | 4 |
| `ring_mpsc` | `src/lib.rs` | `available`, `drain_up_to` | 2 |
| `ring_barrier` | `src/lib.rs` | `Barrier::available( from )` | 1 |

```sh
cd "$(git rev-parse --show-toplevel)"
command grep -rn '\.distance_to(' ring_*/src | command grep -v '^ring_types/' \
  | command grep -v ':[0-9]*: *//' | sed 's|ring/||;s|/src.*||' | sort | uniq -c
```

Live output:

```
      1 ring_barrier
      2 ring_mpsc
      4 ring_seqno
      4 ring_spsc
```

**`ring_seqno` is this function wearing four names.** Four of its five free
functions are `distance_to` plus a `Capacity` and a comparison:

```sh
cd "$(git rev-parse --show-toplevel)"
awk '/^pub fn (laps_between|may_claim|free_slots|pending)\(/,/^}/' ring_seqno/src/lib.rs
```

Live output:

```
pub fn laps_between(earlier: Seq, later: Seq, capacity: Capacity) -> u64 {
    earlier.distance_to(later) / capacity.get() as u64
}
pub fn may_claim(producer: Seq, consumer: Seq, capacity: Capacity) -> bool {
    consumer.distance_to(producer) < capacity.get() as u64
}
pub fn free_slots(producer: Seq, consumer: Seq, capacity: Capacity) -> usize {
    let in_flight = consumer.distance_to(producer);
    (capacity.get() as u64).saturating_sub(in_flight) as usize
}
pub fn pending(producer: Seq, consumer: Seq) -> u64 {
    consumer.distance_to(producer)
}
```

**`pending` is a rename and nothing else** — its body is the call, unmodified.
That is the crate's purpose stated as plainly as it can be: the value of
`ring_seqno` is not arithmetic, it is *parameter names*. `distance_to`'s two
arguments are both `Seq` and their order is the trap; `pending( producer,
consumer )` cannot be called backwards without the mistake being legible at the
call site. The fifth function, `slowest( cursors : &[ Seq ] ) -> Option< Seq >`,
is a minimum over a slice and does not use this item at all.

**Three of the four consumers bypass it anyway.** `ring_spsc` (4 sites),
`ring_mpsc` (2) and `ring_barrier` (1) call `distance_to` directly rather than
through `ring_seqno`. Seven of eleven sites are the fold `ring_seqno` exists to
centralise, performed elsewhere — which is the same duplication
[`mask`](003_capacity_mask.md)'s two call sites show, and safe for the same
reason: saturation makes a misordered call return `0` rather than a wrong large
number.

## Caller Tree

- *No caller within `ring_types`*
- *External: `ring_seqno::laps_between`* (`:47`), *`may_claim`* (`:70`), *`free_slots`* (`:92`), *`pending`* (`:109`)
- *External: `ring_spsc::…::occupancy`* (`:612`), *`available`* (`:861`), *`drain`* (`:920`), *`drain_up_to`* (`:948`)
- *External: `ring_mpsc::…::available`* (`:1056`), *`drain_up_to`* (`:1126`)
- *External: `ring_barrier::…::available`* (`:218`)

**Three crates have a method named `available` and all three are this
function.** `ring_spsc:861`, `ring_mpsc:1056` and `ring_barrier:218` each compute
"how many items can I read" as a distance between two cursors. They differ in
which cursors and in the return type — `usize` for the two rings, `u64` for the
barrier — but not in the arithmetic.

## Callee Tree

- *External: `u64::saturating_sub`* (`id.rs:84`) — the crate's only checked-arithmetic call

**One callee, and it is the reason this item is the safe one of the three.**
`saturating_sub` is a `core` intrinsic on `u64`, `const`, and branch-free on
every target the family runs on. The cost of the safety is nothing measurable;
the benefit is that the most misuse-prone signature in the crate — two arguments
of the same type, order-dependent — has no wrong answer, only an uninformative
one.
