# Capacity

## Representation

A validated power-of-two slot count, held as a **private** `usize`. The privacy is
the whole mechanism: with no public field and one fallible constructor, every
`Capacity` value in the workspace has passed both checks, so
[`mask`](../associated_function/003_capacity_mask.md) is total and `ring_index`
needs no runtime test of its own. It is the only one of the crate's three structs
whose field is private, the only one without `Default`, and the only one whose
construction can fail — all three for the same reason
(→ [`../../pattern/002`](../../pattern/002_a_newtype_that_makes_a_check_unnecessary.md)).

Fifteen crates take it in `src/`, and not one of them re-checks the invariant:

```sh
cd "$(git rev-parse --show-toplevel)"
command grep -r 'is_power_of_two' ring_*/src | command grep -v ring_types/
```

Live output:

```
ring_align/src/lib.rs:    CACHE_LINE.is_power_of_two(),
```

returns one line, and it is not one of the fifteen. `ring_align` does not consume
`Capacity` at all — it is absent from the Crate Usage table below — and this call
checks `CACHE_LINE`, an unrelated constant, against the same standard-library
method name. So the claim holds exactly as stated for the fifteen consumers; the
one hit the raw grep now finds is a same-named method call on a different type.

## Kind

Struct (§ Item Kind Taxonomy : Stable Item Kinds #6)

## Definition

`ring_types/src/capacity.rs:23`

```rust
#[ derive( Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash ) ]
pub struct Capacity( usize );
```

Eight derives, and the absent ninth is the interesting one: `Seq` and
`SlotIndex` both derive `Default` and this does not, because `Capacity::default()`
would have to be `Capacity( 0 )` — exactly the value
[`new`](../associated_function/001_capacity_new.md) rejects.

## File Usage

| File | Line(s) | Context |
|------|---------|---------|
| `ring_types/src/capacity.rs` | 6, 13, 16-17, 20, 23, 25, 35-38, 56-57, 70-72 | Module doc (6); type doc and its example (13, 16-17, 20); **definition (23)**; inherent `impl` header (25); doc examples on `new` (35-38), `get` (56-57), `mask` (70-72) |
| `ring_types/src/lib.rs` | 19, 41 | Module responsibility table (19, doc); **re-export (41)** |

**Two of the eighteen lines are code.** The rest are documentation, of which the
ten example lines are compiled and run as doc tests — so they are executable, but
they are not call sites in the sense the Crate Usage table below means. The same
ratio holds workspace-wide and is why a bare identifier count over this crate
overstates production usage by roughly fifty to one
(→ [`../readme.md`](../readme.md) § Where the counts come from).

Test-only references: `ring_types` (10 in `tests/types_test.rs`), plus 20
consumer crates' test suites — the 15 below and `ring_event`, `ring_flush`,
`ring_publish`, `ring_slot`, `ring_tls`, which name the type in `tests/` without
naming it in `src/`.

## Crate Usage

| Crate | Via File | Purpose |
|-------|----------|---------|
| `ring_types` | `src/capacity.rs`, `src/lib.rs` | Defining crate |
| `ring_barrier` | `src/lib.rs` | Bounds a barrier's slot range |
| `ring_batch` | `src/lib.rs` | Rejects a batch wider than the ring (`BatchTooLarge`) |
| `ring_store` | `src/lib.rs` | Sizes the backing allocation |
| `ring_claim` | `src/lib.rs` | Bounds a claim against available space |
| `ring_config` | `src/lib.rs` | Carries the validated capacity a `RingConfig` was built with |
| `ring_core` | `src/lib.rs` | Carries the validated `Capacity` the crossbeam variant was constructed from, rather than rebuilding one (`src/lib.rs:233`) |
| `ring_cursor` | `src/lib.rs` | Bounds cursor arithmetic |
| `ring_debug` | `src/lib.rs` | Reports the ring's size in a dump |
| `ring_gating` | `src/lib.rs` | Computes the gating distance a producer may run ahead |
| `ring_index` | `src/lib.rs` | **The reason the type exists** — folds a `Seq` with `mask()` (`src/lib.rs:51`) |
| `ring_mpsc` | `src/lib.rs` | Folds inline rather than via `ring_index` (`src/lib.rs:543`) |
| `ring_seqno` | `src/lib.rs` | Bounds sequence-to-slot conversion |
| `ring_shutdown` | `src/lib.rs` | Bounds the drain loop at close |
| `ring_spsc` | `src/lib.rs` | Sizes the single-producer ring |
| `ring_wait` | `src/lib.rs` | Bounds the space check a waiter loops on |

Every consumer is a single-file crate, so `Via File` is `src/lib.rs` in all
fifteen rows. That is a property of the family's shape rather than of this type:

```sh
cd "$(git rev-parse --show-toplevel)"
ls ring_*/src/*.rs | sed 's|.*/||' | sort | uniq -c
```

Live output:

```
      1 capacity.rs
      1 error.rs
      1 id.rs
     33 lib.rs
      1 policy.rs
```

returns `33 lib.rs` plus `capacity.rs`, `error.rs`, `id.rs` and `policy.rs` at one
each — **`ring_types` is the only crate in the family with more than one source
file.** Every other `Via File` column in this subtree is therefore uniform by
construction, and a future crate that grows a second module is the event that
makes the column start carrying information.
