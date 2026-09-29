# workaround

A crate this small has room for exactly one workaround, and it turns out to be
the whole crate. `Buffer::new` is three lines that look like they could be one,
and the storage they build is a heap allocation in a data structure normally
chosen for not making any. Both are routes around constraints that cannot be
removed, and neither says so where it is.

The two instances take the constraints in the order a reader meets them: first
the pair that shapes the three lines — no `unsafe`, and no `Clone` bound to
spend — then the one that shapes the type itself, which is that a ring's
capacity has to survive coming from a config file. What comes out is a crate
whose every apparent redundancy is load-bearing and whose every load-bearing
line is unexplained.

### Overview Table

| ID | Name | Covers |
|----|------|--------|
| 001 | [Three Steps to Avoid Two Bounds](001_three_steps_to_avoid_two_bounds.md) | BF50, BF51 — what each construction step buys, and what a drifting argument would cost |
| 002 | [Capacity Is a Value, Width Is a Type](002_capacity_is_a_value_width_is_a_type.md) | BF52, BF53 — the heap as the price of a configurable capacity, and the opposite choice next to it |

### The Constraints and What Routes Around Them

| Constraint | Where it is set | The route | What the route costs |
|------------|-----------------|-----------|---------------------|
| No `unsafe` | `Cargo.toml:227` | Build a `Vec`, freeze it | An intermediate `Vec` and a length that must match |
| No `Clone` on `Slot` | `ring_slot/src/lib.rs:41` — no supertraits | `resize_with( n, S::default )` | Three lines instead of `vec![ .. ; n ]` |
| Capacity must be runtime | `ring_config/src/lib.rs:65` — `new( slots : usize )` | `Box< [ S ] >` rather than `[ S; CAP ]` | One heap allocation per ring |

Each row's route is correct, each is the best available, and none of the three
is named at the code that takes it. That is the pattern this definition records:
the crate is not doing anything clever, it is doing the only remaining thing, and
the difference is invisible from the source.

### The One That Could Silently Break

Only `workaround/001`'s second finding describes a live edit hazard. The two
arguments to `Vec::with_capacity` and `Vec::resize_with` must be equal, they are
the same expression written twice, and if they ever stop agreeing the
`into_boxed_slice` shrink adds a reallocation — measured, silent, and caught by
no test. The other constraints fail loudly: removing `resize_with` in favour of
`vec!` does not compile, and reaching for an array capacity would break
`ring_config`'s signature immediately.

### Regenerate

```sh
cd "$(git rev-parse --show-toplevel)"

# the three steps, whole
command grep -m1 -B1 -A5 -F '  pub fn new( capacity : Capacity ) -> Self' ring_store/src/lib.rs

# the constraints they route around
grep -n 'unsafe-code' Cargo.toml
grep -n 'pub trait Slot' ring_slot/src/lib.rs
command grep -m1 -A7 -F '  pub fn new( slots : usize ) -> Result< Self, RingError >' ring_config/src/lib.rs

# both size parameters on one line
command grep -m1 -F '  /// let buffer : Buffer< BytesSlot< 8 > > = Buffer::new( Capacity::new( 16 ).unwrap() );' ring_store/src/lib.rs

# every library construction of a Buffer
grep -rn 'Buffer::new' ring_*/src/*.rs | grep -vE ':[[:space:]]*(///|//!|//)' | sort
```

Allocation and reallocation counts for the four candidate constructions come
from a release probe under a counting global allocator; the `Clone` rejection
comes from a `rustc` compile probe. Both are quoted in `workaround/001`.

### Findings Recorded Here

| ID | Subject | Tier | Finding |
|----|---------|------|---------|
| BF50 | `ring_store` | n/a — doc gap | `new`'s three steps are the intersection of the workspace `unsafe` denial and a `Clone` bound the `Slot` trait does not carry — `vec![ S::default(); n ]` fails to compile under `new`'s `S : Default` and would propagate `Clone` to the payload if the bound were added — and neither constraint is named at the call site |
| BF51 | `ring_store` | n/a — coverage | The two `capacity.get()` arguments must stay equal or `into_boxed_slice` shrinks and reallocates, measured at 1 alloc + 1 realloc, breaking the allocate-once property silently with no test asserting it; `with_capacity` is measurably redundant today but is the only form the API guarantees |
| BF52 | `ring_store` | n/a — doc gap | The heap allocation is the price of keeping capacity a runtime value, and `ring_config::new( slots : usize ) -> Result` is the concrete signature that makes a const-generic capacity impossible — a whole design was traded here and nothing in either crate records it |
| BF53 | `ring_store` | n/a — observation | Slot width is a const generic and slot count is a runtime value, meeting on one line of this crate's own doc example, under a coherent rule — a size that changes the type is a type parameter, a size that changes only the count is a value — that appears in neither crate |
