# non_functional_requirement

This crate exists for its non-functional properties. Functionally it is a boxed
slice with an index helper; what makes it worth a crate is that it allocates
exactly once, addresses in constant time, and does neither by convention — both
are consequences of the implementation rather than rules someone must follow. The
two instances measure them.

Both measurements come out clean and both find the same kind of hole: the
strongest property has the weakest test. One allocation, exactly sized, none
thereafter — asserted nowhere. Constant-time addressing with the two linear
operations unreachable from any hot path — true because nothing calls them,
which is a fact about the family rather than a property of the type. And the
predicate that would report quiescence costs full capacity precisely when it
succeeds, which nothing says.

### Overview Table

| ID | Name | Covers |
|----|------|--------|
| 001 | [Allocate Once, Then Never Again](001_allocate_once_then_never_again.md) | BF42, BF43 — the allocation claim measured end to end, and the boundary it stops at |
| 002 | [Bounded Work per Operation](002_bounded_work_per_operation.md) | BF44, BF45 — a clean cost split, and a predicate cheapest when the answer is no |

### The Measurements

| Property | Measured | Asserted by a test? |
|----------|----------|--------------------|
| One allocation per buffer | 1 at capacity 1, 16, 1024, and at `BytesSlot< 4096 >` × 256 | No |
| Array sized exactly `capacity * size_of::< S >()` | 8, 128, 8192, 1050624 bytes — no overhead, no padding | No |
| Zero allocations after construction | 0 across 10,000 `at_mut`/`at`, one sweep, one iteration, one `clear` | No |
| `clear` costs exactly `capacity` | 1024 `Slot::clear` calls at capacity 1024 | Reach only |
| `all_empty` costs at most `capacity` | 1024 calls when empty, 1 when slot 0 is occupied | Reach only |
| Addressing costs `O(1)` | A mask and a slice index, by inspection | Indirectly |

### Where Allocate-Once Hands Off

The property belongs to the slot array, not to the traffic. A
`Buffer< TypedSlot< String > >` allocates once for its slots and once more per
payload; a `Buffer< BytesSlot< N > >` allocates once and never again, because the
payload lives inline in `[ u8; N ]`.

That is the argument for the second slot shape, and it is made in neither crate.
`ring_store` states the allocation claim without saying which shape preserves
it; `ring_slot` defines both shapes without saying that this is what the byte
array buys. The one place it could have been settled — a benchmark comparing the
two — never instantiates `BytesSlot` at all (`ring_slot`'s SL4).

### Regenerate

```sh
cd "$(git rev-parse --show-toplevel)"

# the requirement as the crate states it
command grep -m1 -A3 -F '//! What is left is small enough to state completely: `capacity` slots allocated' ring_store/src/lib.rs

# the two capacity-bounded operations
awk '/^  pub fn clear\( &mut self \)$/{ n1 = NR } n1 && NR >= n1 && NR <= n1 + 6 { print } /^  \/\/\/ assert!\( buffer\.all_empty\(\) \);$/{ n2 = NR } n2 && NR >= n2 + 2 && NR <= n2 + 6 { print }' ring_store/src/lib.rs

# the four constant-time addressers
awk '/^  \/\/\/ condition to handle\.$/{ n1 = NR } n1 && NR >= n1 + 1 && NR <= n1 + 5 { print } /^  pub fn get_mut\( &mut self, index : SlotIndex \) -> &mut S$/{ n2 = NR } n2 && NR >= n2 && NR <= n2 + 3 { print } /^  \/\/\/ assert_eq!\( buffer\.get\( SlotIndex\( 2 \) \)\.get\(\), Some\( &1 \) \);$/{ n3 = NR } n3 && NR >= n3 + 2 && NR <= n3 + 13 { print }' ring_store/src/lib.rs
```

Allocation counts, byte totals and per-sweep call counts come from a release
probe running a counting global allocator and a counting `Slot` implementation;
all of them are quoted in the instances.

### Findings Recorded Here

| ID | Subject | Tier | Finding |
|----|---------|------|---------|
| BF42 | `ring_store` | n/a — coverage | One allocation per buffer, sized exactly `capacity * size_of::< S >()` at every capacity and both shapes, and zero across ten thousand operations — the crate's strongest property, asserted by no test |
| BF43 | `ring_store` | n/a — doc gap | Allocate-once covers the slot array and not the payloads, so only `BytesSlot` preserves it end to end; that this is the reason the second slot shape exists is stated in neither crate |
| BF44 | `ring_store` | n/a — observation | The four addressers are constant-time and the two `capacity`-bounded sweeps have no library caller, so a ring's per-message path is constant-time because nothing on it reaches the linear operations |
| BF45 | `ring_store` | n/a — doc gap | `all_empty` short-circuits, so confirming emptiness always costs full capacity while finding an occupied slot can cost one call, and the `true` it returns is an unsynchronized snapshot — neither is documented |
