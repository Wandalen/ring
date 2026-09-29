# algorithm

There are two algorithms in this crate and neither is more than four lines. A
sequence becomes a slot by a bitmask that lives in another crate; a buffer comes
into existence by reserving, filling and freezing a `Vec`. Everything else the
type does is a slice index.

The first instance follows the fold and finds the delegation both correct and
unusually well argued — `at` exists so that `seq % capacity` is never written
twice in the family, and the doc says so — then finds `at_mut` spelling the same
composition in two lines rather than one, for a reason that looks like the borrow
checker and, compiled, is not. It also counts what the crate it delegates to
actually sells: `ring_index` publishes three functions, the family imports one,
and both crates that import it import the same one. The second measures
construction with a counting allocator: exactly one allocation at every capacity,
`Default::default` called once per slot, and zero allocations across ten thousand
subsequent operations.

### Overview Table

| ID | Name | Covers |
|----|------|--------|
| 001 | [One Mask, No Modulo](001_one_mask_no_modulo.md) | BF10, BF11 — one composition written two ways, and a Tier 1 crate selling three functions to a family that buys one |
| 002 | [One Allocation, N Defaults](002_one_allocation_n_defaults.md) | BF12, BF13 — the allocation claim measured exact, and the per-slot cost the crate does not bound |

### The Fold

```rust
SlotIndex( ( seq.0 as usize ) & capacity.mask() )
```

One cast, one mask, no branch and no error path. It is total because `Capacity`
cannot exist unless it is a power of two, so the validation happens once at
capacity construction rather than on every operation. `ring_index`'s own module
comment prices the alternative: an integer `%` at 20–40 cycles, on the claim path
and the read path of every operation the family performs.

### What `ring_index` Sells and What the Family Buys

| Function | Returns | Callers outside `ring_index` |
|----------|---------|:----------------------------:|
| `of` | `SlotIndex` | `ring_batch`, `ring_store` |
| `aliases` | `bool` | none |
| `run` | `Vec< SlotIndex >` | none |

`aliases` is `of( a ) == of( b )`, which a caller holding both sequences writes
inline. `run` allocates a `Vec` per call, which is the one thing this path exists
to avoid, and there is no batch accessor on `Buffer` to pair it with.

### What Construction Costs

| Capacity | Allocations | Bytes | `Default::default` calls |
|---------:|:-----------:|------:|-------------------------:|
| 1 | 1 | 8 | 1 |
| 16 | 1 | 128 | 16 |
| 1024 | 1 | 8192 | 1024 |
| 256 × `BytesSlot< 4096 >` | 1 | 1050624 | 256 |

One allocation at every scale, because each of the three steps is chosen not to
reallocate: `with_capacity` takes the whole block, `resize_with` fills exactly
that reservation, and `into_boxed_slice` on a full `Vec` is a no-op rather than a
shrinking copy. Ten thousand `at_mut` calls plus `at`, `all_empty`, `iter` and
`clear` allocate nothing at all.

The initialisation is not once — it is `N` times, and the constant is whatever
the slot shape's `Default` costs. `ring_store` neither knows nor bounds it, and
`ring_slot`, which defines the trait, never mentions `Default`.

### Regenerate

```sh
cd "$(git rev-parse --show-toplevel)"

# the whole fold, and why it needs no error path
command grep -m1 -B1 -A3 -F 'pub fn of( seq : Seq, capacity : Capacity ) -> SlotIndex' ring_index/src/lib.rs
command grep -m1 -A8 -F '//! the [`ring_types::SlotIndex`] it addresses. The feature'"'"'s whole reason for' ring_index/src/lib.rs

# one composition, two bodies
awk '/^  \/\/\/ Borrow the slot a sequence addresses, folding through `ring_index`\.$/{ n1 = NR } n1 && NR >= n1 && NR <= n1 + 4 { print } /^  \/\/\/ assert_eq!\( buffer\.get\( SlotIndex\( 2 \) \)\.get\(\), Some\( &1 \) \);$/{ n2 = NR } n2 && NR >= n2 + 2 && NR <= n2 + 13 { print }' ring_store/src/lib.rs

# three published functions
grep -n '^pub fn ' ring_index/src/lib.rs

# and every use of the crate in family source, doc lines excluded
# sorted: grep is shimmed to a parallel ugrep here, so hits arrive in completion order
grep -rn 'ring_index' ring_*/src/*.rs | grep -v '^ring_index/' \
  | grep -vE ':[[:space:]]*(///|//!|//)' | sed 's|^/home/user1/pro/lib/yrd_gamedev/codename_space_sandbox/module/||' | sort

# the three construction steps
command grep -m1 -A28 -F '  /// Allocate exactly `capacity` empty slots, once.' ring_store/src/lib.rs
```

Allocation counts, `Default::default` counts and the one-line-`at_mut` compile
check come from release probes; the figures are quoted in the instances.

### Findings Recorded Here

| ID | Subject | Tier | Finding |
|----|---------|------|---------|
| BF10 | `ring_store` | n/a — observation | `at` and `at_mut` spell one composition two ways; the two-line form looks borrow-checker-imposed and is not — two-phase borrows accept the one-liner |
| BF11 | `ring_index` | n/a — coverage | A Tier 1 crate publishes three functions and the family imports one; `aliases` and `run` have no caller, and `run` would allocate per batch on a path premised on not allocating |
| BF12 | `ring_store` | n/a — observation | Construction costs exactly one allocation at every capacity and nothing afterwards allocates — an exact match to the reached-test's claim, asserted by no test |
| BF13 | `ring_store` | n/a — doc gap | `Default::default` runs once per slot, so a shape's `default()` sits on the construction path `capacity` times; the doc's "once" attaches to the allocation, and nothing tells a shape author which |
