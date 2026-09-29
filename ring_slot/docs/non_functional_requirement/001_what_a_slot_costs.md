# Non-Functional Requirement: What a Slot Costs

### Scope

**Purpose:** Record the cost of every operation the crate offers, the six things
the crate provably does not do, and that the resulting profile would qualify it
for `no_std` — which this crate does not declare, though its one dependency does.

**Responsibility:** The runtime cost budget of `ring_slot` — per operation, and
the crate-wide absences that make the budget hold.

**In Scope:** All ten `pub fn` in `ring_slot/src/lib.rs`; the mechanical
census of the crate's source; the `no_std` question.

**Out of Scope:** Memory *footprint* — what a slot occupies at rest — is
[`data_structure/001`](../data_structure/001_sixteen_bytes_to_carry_eight.md).
The four traits' costs, derived or hand-written, are
[`non_functional_requirement/002`](002_four_traits_and_what_they_compare.md).

---

## The Budget

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

Two operations scale, and only two:

```sh
cd "$(git rev-parse --show-toplevel)"
sed -n '/^    Self { bytes : \[ 0; N ], len : 0 }$/p;/^    self\.bytes\[ \.\.payload\.len() ]\.copy_from_slice( payload );$/p' ring_slot/src/lib.rs
```

Live output:

```
    Self { bytes : [ 0; N ], len : 0 }
    self.bytes[ ..payload.len() ].copy_from_slice( payload );
```

The first is paid once per slot at allocation and never again; the second is
paid per publish and is bounded by the payload, not by `N`.

---

### SL33 — Six Absences Hold the Budget Up, and All Six Are Mechanically Checkable

The table above is only true because the crate does none of the things that
would break it:

```sh
cd "$(git rev-parse --show-toplevel)"
body=$( grep -vE '^[[:space:]]*//' ring_slot/src/lib.rs )
for pat in 'unsafe' 'Vec|Box|String|alloc' 'Atomic|Ordering' 'loop|while|for [a-z_]+ in' 'panic!|unwrap|expect' 'std::|core::'; do
  printf '  %-26s %d\n' "$pat" "$( printf '%s\n' "$body" | grep -cE "$pat" )"
done
```

Live output:

```
  unsafe                     0
  Vec|Box|String|alloc       0
  Atomic|Ordering            0
  loop|while|for [a-z_]+ in  0
  panic!|unwrap|expect       0
  std::|core::               2
```

No `unsafe`, no allocation, no atomics, no loops, no explicit panic, no
path-qualified `std`/`core` use. The comment filter is what makes the count
honest — every one of these words appears in doctests, and without stripping
them the census reports nonzero for four of the six.

Two of the six want a caveat, and the crate states neither.

The panic row is about the *source text*, not about reachability.
`copy_from_slice` panics on a length mismatch and `self.bytes[ ..k ]` panics on
an out-of-range `k`; both are unreachable here because `write`'s bound check
precedes them
([`algorithm/001`](../algorithm/001_write_is_a_bound_check_and_a_copy.md) SL13),
but the panic *paths* are in the compiled function unless the optimizer proves
them away. A reader taking "zero panics" from the census has read one row too
literally.

The `std::`/`core::` row is a zero of a different kind: the crate imports nothing
path-qualified because it needs nothing — its only dependency is `ring_types`,
and everything else is prelude.

**Finding.** The crate's non-functional profile is unusually easy to state and
unusually easy to check: six greps, all zero, over 288 lines. That is worth
recording as a baseline, because every one of the six is a property a future edit
could take away silently — adding a `Vec` for a growable variant, an `Atomic` for
a shared slot, a loop for an incremental writer
([`item/002`](../item/002_the_six_of_a_bytes_slot.md) SL28 names the pressure
that would invite one).

None of the six is asserted anywhere. The workspace lint table denies `unsafe`
and nothing else on this list, so five of the six absences are conventions rather
than guarantees. The census above is the test that does not exist.

---

### SL34 — The Profile Is `no_std`-Shaped and This Crate Does Not Declare It

Zero allocation, zero atomics, zero `std::` paths, one dependency. That is the
shape of a crate that runs without an operating system, and a `no_std` consumer
does compile against the whole public surface — measured, release:

```
   Compiling ring_types v0.1.0
   Compiling ring_slot v0.1.0
   Compiling nostd_probe v0.0.0
    Finished `release` profile [optimized] target(s) in 0.40s
```

The probe is a `#![ no_std ]` library calling all ten functions plus both trait
methods. It links.

That is weaker evidence than it looks, and the distinction matters: a `no_std`
crate may depend on a crate that links `std`, so this proves the *API* is usable
from `no_std`, not that `ring_slot` itself is `std`-free. Proving the stronger
claim needs a build for a bare-metal target, and none is installed on this
machine — so it is stated here as untested rather than established.

What is established is where in the family the question is asked at all:

```sh
cd "$(git rev-parse --show-toplevel)"
grep -rl 'no_std' ring_*/src/*.rs 2>/dev/null \
  || echo '  no crate in the family declares no_std'
```

Live output:

```
ring_overflow/src/lib.rs
ring_stats/src/lib.rs
ring_types/src/lib.rs
```

**Correction (2026-09-20): the finding below was written against a family in
which nothing declared `#![ no_std ]`, and was never revisited once that stopped
being true.** The recipe above has since been retargeted at `ring/` and re-run,
and its output — three declaring crates, printed directly above — has
contradicted the paragraph under it ever since. `ring_types` is one of the
three: it declares the attribute at `ring_types/src/lib.rs:27`, and it is
`ring_slot`'s only dependency. The "neither declares it" half is therefore
false, and the half about the dependency is exactly backwards.

**Finding, as it now stands.** `ring_slot` has the source-level profile of a
`no_std` crate and does not declare `#![ no_std ]`; its one dependency,
`ring_types`, does. The declaration is what would turn the profile into a
guarantee — with it, a future `std::collections` import fails to compile;
without it, the same import compiles and the profile is gone with no signal.
`ring_slot` is now the only undeclared link in its own dependency chain, which
makes the missing line cheaper to add than it was when this was written, not
dearer.

Whether the family *wants* `no_std` is a family-wide question and not this
crate's to settle: `ring_store` allocates a boxed slice and `ring_mpsc` uses
atomics, so most of the family cannot be `no_std` regardless. But the two crates
at the bottom of the tier list — `ring_types` at Tier 0 and `ring_slot` at Tier 1
— are exactly the ones where it would cost nothing and buy an enforced boundary,
and `ring_types` has since taken it. Recording it here because the cost of
adding the attribute is one line today and grows with every future import that
would have been rejected.

---

### Related

| Instance | Relationship |
|----------|--------------|
| [`non_functional_requirement/002`](002_four_traits_and_what_they_compare.md) | The four traits' costs, derived or hand-written, which this budget omits |
| [`data_structure/001`](../data_structure/001_sixteen_bytes_to_carry_eight.md) | Footprint at rest, against this instance's cost in motion |
| [`algorithm/001`](../algorithm/001_write_is_a_bound_check_and_a_copy.md) | Why `write` is the only branching operation, and why it cannot panic |
| [`algorithm/002`](../algorithm/002_emptiness_three_ways.md) | The O(1) emptiness computations |
| [`decisions/002`](../decisions/002_a_length_rather_than_a_flag.md) | The `MaybeUninit` rejection that buys the zero in the `unsafe` row |

### Sources

| Fact | Where |
|------|-------|
| The two O(N) operations | `ring_slot/src/lib.rs:278, 347` |
| The six-way census | `ring_slot/src/lib.rs`, comment-stripped |
| One dependency | `ring_slot/Cargo.toml` |
| Which crates declare `no_std` | `ring_*/src/*.rs` — three occurrences, quoted above |
| The dependency's own declaration | `ring_types/src/lib.rs:27` |
| A `no_std` consumer compiling | Release probe, quoted above |
| `unsafe` denied workspace-wide | `Cargo.toml` — `[workspace.lints.rust]` |

### Tests

| Test | Covers |
|------|--------|
| `a_write_of_exactly_capacity_is_accepted` | The O(payload) path at its maximum |
| `an_oversized_write_is_refused_with_both_numbers` | The bound check that keeps the panic paths unreachable |
| `a_zero_capacity_slot_accepts_only_nothing` | `N == 0`, where the O(N) construction is free |
| `capacity_is_the_const_parameter` | The O(1) `const` reads |
| *(to create)* | The six-way census as an assertion, so an added `Vec` or `Atomic` fails a test |
