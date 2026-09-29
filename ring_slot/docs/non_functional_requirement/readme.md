# non_functional_requirement

Every operation in the crate is O(1) except two, and both scale for reasons the
design chose deliberately: `BytesSlot::empty` zeroes `N` bytes once per slot at
allocation, and `write` copies the payload rather than the capacity. That budget
holds up on six absences — no `unsafe`, no allocation, no atomics, no loops, no
explicit panic, no path-qualified `std`/`core` — all six mechanically checkable
in six greps over 288 lines, and five of them enforced by nothing.

The second instance is about the obligations the types carry rather than the work
they do. Four traits are attached to each shape where a reader sees them — at the
top for `TypedSlot`, and for `BytesSlot` one at the top and three now written out
below it — and no consumer requires any of them; the one trait three consumers
*do* require appears eighty lines later as a hand-written impl, written that way
to dodge a bound `derive` would have added silently. Between the two: a `Clone`
that copies the capacity, unconditionally on the shape least able to afford it.

### Overview Table

| ID | Name | Covers |
|----|------|--------|
| 001 | [What a Slot Costs](001_what_a_slot_costs.md) | SL33, SL34 — six absences holding up the budget, and a `no_std` profile the crate does not declare while its dependency does |
| 002 | [Four Traits and What They Compare](002_four_traits_and_what_they_compare.md) | SL35, SL36 — a clone proportional to capacity, and the trait that is required but not derived |

### The Budget

| Operation | Cost | Notes |
|-----------|------|-------|
| `TypedSlot::empty` | O(1) | `const`; one discriminant write |
| `TypedSlot::set` | O(1) + move of `T` | `Option::replace` |
| `TypedSlot::get` | O(1) | `const`; a borrow, no copy |
| `TypedSlot::take` | O(1) + move of `T` | `Option::take` |
| `BytesSlot::empty` | **O(N)** | Zeroes the array; `const`, so free in static position |
| `BytesSlot::capacity` | O(1) | `const`; returns `N`, ignores the receiver |
| `BytesSlot::len` | O(1) | `const`; one field read |
| `BytesSlot::is_empty` | O(1) | `const`; one comparison |
| `BytesSlot::write` | **O(payload)** | One comparison, one `copy_from_slice`, one store |
| `BytesSlot::read` | O(1) | A slice range, no copy |
| `Slot::clear` | O(1) | One store, or one drop |

The O(N) construction is paid once per slot at allocation and never again; the
O(payload) write is paid per publish and is bounded by the payload rather than by
`N`.

### Six Zeroes, One Caveat

| Census | Count | Enforced by |
|--------|------:|-------------|
| `unsafe` | 0 | `unsafe-code = "deny"`, workspace-wide |
| `Vec`/`Box`/`String`/`alloc` | 0 | — |
| `Atomic`/`Ordering` | 0 | — |
| `loop`/`while`/`for … in` | 0 | — |
| `panic!`/`unwrap`/`expect` | 0 | — |
| `std::`/`core::` | 0 | — |

The comment filter is what makes the count honest — every one of these words
appears in a doctest, and without stripping comments four of the six read
nonzero. The panic row is about the *source text* and not about reachability:
`copy_from_slice` panics on a length mismatch and the slice index panics
out-of-range, both unreachable because `write`'s bound check precedes them, but
the paths are in the compiled function unless the optimizer proves them away.

Five of the six are conventions rather than guarantees. The census is the test
that does not exist.

### `no_std`-Shaped, Undeclared

Zero allocation, zero atomics, zero `std::` paths, one dependency. A `#![ no_std ]`
probe calling all ten functions plus both trait methods compiles and links — which
proves the *API* is usable from `no_std`, not that the crate is `std`-free; the
stronger claim needs a bare-metal target build and none is installed here. Three
crates in the family declare `#![ no_std ]` — `ring_overflow`, `ring_stats` and
`ring_types` — and most of the rest cannot: `ring_store` allocates a boxed
slice, `ring_mpsc` uses atomics. `ring_slot` is not one of the three, and
`ring_types`, which is, is its only dependency — leaving `ring_slot` the single
undeclared link in its own chain, exactly where the attribute would cost one
line and buy an enforced boundary.

**Correction (2026-09-20):** this section read "None of the 33 crates declares
`#![ no_std ]`" until the census below was retargeted at `ring/` and re-run
against it. Full disposition in [`001`](001_what_a_slot_costs.md) § SL34.

### The Obligation Nobody Asked For

| Bound | Sites in the family |
|-------|--------------------:|
| `Slot + Clone` | 0 |
| `Slot + PartialEq` | 0 |
| `Slot + Eq` | 0 |
| `Slot + Debug` | 0 |
| **`Slot + Default`** | **3** |

`derive` on `BytesSlot< N >` produces an *unconditional* impl — `N` is a const
parameter, so there is no type to bound — while on `TypedSlot< T >` it produces
`impl< T : Clone > Clone`. So `S : Slot + Clone` admits every `BytesSlot< N >` at
a cost proportional to `N` (measured: 4104 bytes moved to duplicate a one-byte
payload in a `BytesSlot< 4096 >`) and only *some* `TypedSlot< T >`. The crate's
own suite calls `.clone()` zero times.

`Default` is the trait that is actually required, and it is hand-written as
`impl< T >` rather than `impl< T : Default >` — the distinction is the whole
reason it is not derived, since `#[ derive( Default ) ]` would emit a `T : Default`
bound that `Option< T >` does not need. Replacing it with the derive is a tidying
edit that compiles today, because every payload in the tree happens to have a
`Default`, and regresses at the first consumer whose payload does not.

### Regenerate

```sh
cd "$(git rev-parse --show-toplevel)"

# the two operations that scale
sed -n '/^    Self { bytes : \[ 0; N ], len : 0 }$/p;/^    self\.bytes\[ \.\.payload\.len() ]\.copy_from_slice( payload );$/p' ring_slot/src/lib.rs

# the six-way census — the comment filter is load-bearing
body=$( grep -vE '^[[:space:]]*//' ring_slot/src/lib.rs )
for pat in 'unsafe' 'Vec|Box|String|alloc' 'Atomic|Ordering' 'loop|while|for [a-z_]+ in' 'panic!|unwrap|expect' 'std::|core::'; do
  printf '  %-26s %d\n' "$pat" "$( printf '%s\n' "$body" | grep -cE "$pat" )"
done

# which crates in the family declare no_std
grep -rl 'no_std' ring_*/src/*.rs 2>/dev/null \
  || echo '  no crate in the family declares no_std'

# where the four traits on each shape come from, and the bound actually required
command grep -E '^#\[ derive|^impl< const N : usize > (core::fmt::Debug|PartialEq|Eq) for BytesSlot' ring_slot/src/lib.rs
grep -rhoE 'Slot \+ [A-Za-z]+' ring_*/src/*.rs | sort | uniq -c

# the hand-written Default, and the bound it deliberately omits
sed -n '/^impl< T > Default for TypedSlot< T >$/,/^}$/p;/^impl< const N : usize > Default for BytesSlot< N >$/,/^}$/p' ring_slot/src/lib.rs

# no clone is exercised anywhere in the crate's own suite — reported through
# `printf`, because `grep -c` exits 1 on a zero count and zero is the claim
printf '  .clone() calls in slot_test.rs: %s\n' \
  "$( grep -c '\.clone()' ring_slot/tests/slot_test.rs )"
```

Live output:

```
    Self { bytes : [ 0; N ], len : 0 }
    self.bytes[ ..payload.len() ].copy_from_slice( payload );
  unsafe                     0
  Vec|Box|String|alloc       0
  Atomic|Ordering            0
  loop|while|for [a-z_]+ in  0
  panic!|unwrap|expect       0
  std::|core::               2
ring_overflow/src/lib.rs
ring_stats/src/lib.rs
ring_types/src/lib.rs
#[ derive( Debug, Clone, PartialEq, Eq ) ]
#[ derive( Clone ) ]
impl< const N : usize > core::fmt::Debug for BytesSlot< N >
impl< const N : usize > PartialEq for BytesSlot< N >
impl< const N : usize > Eq for BytesSlot< N > {}
      1 Slot + Clone
      4 Slot + Default
impl< T > Default for TypedSlot< T >
{
  fn default() -> Self
  {
    Self::empty()
  }
}
impl< const N : usize > Default for BytesSlot< N >
{
  fn default() -> Self
  {
    Self::empty()
  }
}
  .clone() calls in slot_test.rs: 0
```

Clone cost, derive conditionality, and the derived-`Default` refusal come from a
release probe; the figures are quoted in the instances.

### Findings Recorded Here

| ID | Subject | Tier | Finding |
|----|---------|------|---------|
| SL33 | `ring_slot` | n/a — unenforced | Six mechanically-checkable absences hold the cost budget up; only `unsafe` is enforced by a lint, and no test asserts any of the other five |
| SL34 | `ring_slot` | n/a — unenforced | The crate has the source profile of a `no_std` crate and does not declare it, while its one dependency `ring_types` does — so a future `std` import would compile and remove the property with no signal |
| SL35 | `ring_slot` | **measured cost** | `Clone` on a `BytesSlot` copies `N`, not `len` — 4104 bytes for a one-byte payload — and is unconditional on that shape while conditional on `TypedSlot`'s `T` |
| SL36 | `ring_slot` | **latent hazard** | The three real consumer bounds are all `Slot + Default`, satisfied by a hand-written `impl< T >`; replacing it with `#[ derive( Default ) ]` compiles today and narrows the type silently |
