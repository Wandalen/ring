# Capacity::get

## Representation

Returns the validated slot count. **The most-called function in the crate** — 29
production call sites across 11 sibling crates, against 2 for
[`new`](001_capacity_new.md) and 2 for [`mask`](003_capacity_mask.md).

It is also the one accessor here that is not redundant.
[`Seq`](../struct/002_seq.md) and [`SlotIndex`](../struct/003_slot_index.md) have
public fields, so `.0` and `.get()` are interchangeable for them;
`Capacity`'s field is private, making this the only way out of the newtype. Every
one of those 29 sites had no alternative.

**What the callers do with it says what the type is actually for.** Twelve of the
29 pass it to sizing (`Vec::with_capacity`, `ArrayQueue::new`, `resize_with`),
eight to a comparison against a requested count, and the rest to arithmetic on
free/pending space. Not one re-derives a mask from it — that is
[`mask`](003_capacity_mask.md)'s two sites, and they are disjoint.

## Kind

Associated Function/Method (§ Item Kind Taxonomy : Associated Item Kinds #1)

## Definition

`ring_types/src/capacity.rs:60`

```rust
#[ must_use ]
pub const fn get( self ) -> usize
```

Body is `self.0`. `const fn`, `Copy` receiver by value, `#[ must_use ]` — the
same three properties as ten of its eleven inherent siblings.

## File Usage

| File | Line(s) | Context |
|------|---------|---------|
| `ring_types/src/capacity.rs` | 53, 56-57, 59-60, 62 | Doc summary (53); doc example (56-57); `#[ must_use ]` and **the definition (59-60)**; body `self.0` (62) |

Test-only references: `ring_types` — one line, `types_test.rs:93`, inside
`capacity_accepts_powers_of_two`'s loop: `assert_eq!( cap.get(), slots );` for
every power of two up to 2¹⁶. Plus 19 consumer suites reaching it through
`Capacity`.

## Crate Usage

| Crate | Via File | Purpose | Sites |
|-------|----------|---------|-------|
| `ring_types` | `src/capacity.rs` | Defining crate | — |
| `ring_mpsc` | `src/lib.rs` | Stamp-array sizing, contiguity bounds, `Debug` output, drain limits | 6 |
| `ring_debug` | `src/lib.rs` | Lap-violation and occupancy-disagreement checks | 4 |
| `ring_gating` | `src/lib.rs` | Gating distance, `BatchTooLarge` bound | 4 |
| `ring_seqno` | `src/lib.rs` | Lap arithmetic, may-claim test, free-space computation | 3 |
| `ring_spsc` | `src/lib.rs` | `Debug` output, free-space and full tests | 3 |
| `ring_batch` | `src/lib.rs` | `BatchTooLarge` bound | 2 |
| `ring_store` | `src/lib.rs` | `Vec::with_capacity` and `resize_with` | 2 |
| `ring_claim` | `src/lib.rs` | `BatchTooLarge` bound on the consumer count | 2 |
| `ring_bench` | `src/lib.rs` | Reports the configured capacity in a result row | 1 |
| `ring_config` | `src/lib.rs` | Caps a requested batch at the ring's size | 1 |
| `ring_core` | `src/lib.rs` | `ArrayQueue::new( config.capacity().get() )` | 1 |

```sh
cd "$(git rev-parse --show-toplevel)"
command grep -rn 'capacity\.get()\|capacity()\.get()' ring_*/src \
  | command grep -v ring_types/ | command grep -v ':[0-9]*: *//' \
  | sed 's|ring/||;s|/src.*||' | sort | uniq -c
```

Live output:

```
      2 ring_batch
      1 ring_bench
      2 ring_store
      2 ring_claim
      1 ring_config
      1 ring_core
      4 ring_debug
      4 ring_gating
      6 ring_mpsc
      3 ring_seqno
      3 ring_spsc
```

**Identifier collision matters here.** A bare `.get()` grep over the same scope
returns 40 non-doc lines; nine are `UnsafeCell::get` (`ring_cursor` ×4,
`ring_mpsc` ×2, `ring_spsc` ×2, `ring_event` ×1) and two are
[`SlotIndex::get`](010_slot_index_get.md). The receiver-qualified pattern above is
what separates them, and it is a heuristic rather than a parse — a call through a
differently-named binding would be missed.

## Caller Tree

- *No caller within `ring_types`* — nothing in the crate reads a `Capacity`'s slot count; the only internal use of the field is [`mask`](003_capacity_mask.md)'s `self.0 - 1`
- *External: 29 sites across 11 crates* — enumerated in the Crate Usage table above

## Callee Tree

- *(none)* — the body is a field read

**A callee tree with no entries is not an empty section**, it is the finding: this
function cannot fail, cannot allocate, and cannot be affected by a change anywhere
else in the workspace. Which is why 29 call sites is a cheap number rather than a
coupling problem.
