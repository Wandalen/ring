# Associated Function Items

### Scope

- **Purpose**: Catalog the crate's thirteen associated functions — twelve `const fn` classifiers, accessors and constructors, plus one trait method.
- **Responsibility**: Give each one's signature, its callers inside and outside the crate, and its callees.
- **In Scope**: All thirteen. Each carries `## Caller Tree` and `## Callee Tree` per OT006.
- **Out of Scope**: The `impl` blocks holding them (→ [`../implementation/`](../implementation/)); the three associated constants, which are a different kind (→ [`../associated_constant/`](../associated_constant/)).

### Overview Table

| ID | Name | Kind | Defined In | Status |
|----|------|------|-----------|--------|
| 001 | [Capacity::new](001_capacity_new.md) | Associated Function/Method (A#1) | `src/capacity.rs:40` | 🔄 |
| 002 | [Capacity::get](002_capacity_get.md) | Associated Function/Method (A#1) | `src/capacity.rs:60` | 🔄 |
| 003 | [Capacity::mask](003_capacity_mask.md) | Associated Function/Method (A#1) | `src/capacity.rs:75` | 🔄 |
| 004 | [RingError::is_configuration](004_ring_error_is_configuration.md) | Associated Function/Method (A#1) | `src/error.rs:111` | 🔄 |
| 005 | [RingError::is_transient](005_ring_error_is_transient.md) | Associated Function/Method (A#1) | `src/error.rs:146` | 🔄 |
| 006 | [Display::fmt for RingError](006_display_fmt_for_ring_error.md) | Associated Function/Method (A#1) | `src/error.rs:164` | 🔄 |
| 007 | [Seq::next](007_seq_next.md) | Associated Function/Method (A#1) | `src/id.rs:45` | 🔄 |
| 008 | [Seq::advanced_by](008_seq_advanced_by.md) | Associated Function/Method (A#1) | `src/id.rs:65` | 🔄 |
| 009 | [Seq::distance_to](009_seq_distance_to.md) | Associated Function/Method (A#1) | `src/id.rs:82` | 🔄 |
| 010 | [SlotIndex::get](010_slot_index_get.md) | Associated Function/Method (A#1) | `src/id.rs:110` | 🔄 |
| 011 | [WaitKind::is_non_blocking](011_wait_kind_is_non_blocking.md) | Associated Function/Method (A#1) | `src/policy.rs:81` | 🔄 |
| 012 | [OverflowPolicy::reports_failure](012_overflow_policy_reports_failure.md) | Associated Function/Method (A#1) | `src/policy.rs:153` | 🔄 |
| 013 | [OverflowPolicy::drops_silently](013_overflow_policy_drops_silently.md) | Associated Function/Method (A#1) | `src/policy.rs:178` | 🔄 |

### Production call sites, measured

| Function | Production callers | Crates |
|----------|-------------------|--------|
| `Capacity::get` | 29 | 11 |
| `Seq::advanced_by` | 14 | 7 |
| `Seq::distance_to` | 11 | 4 |
| `Capacity::new` | 2 | 2 |
| `Capacity::mask` | 2 | 2 |
| `Seq::next` | **2** | 2 |
| `SlotIndex::get` | 2 | 1 |
| `WaitKind::is_non_blocking` | 1 | 1 |
| `Display::fmt` | — | reached by formatting machinery, not by name |
| `RingError::is_configuration` | **0** | — |
| `RingError::is_transient` | **0** | — |
| `OverflowPolicy::reports_failure` | **0** | — |
| `OverflowPolicy::drops_silently` | **0** | — |

**Four of the thirteen have no production caller anywhere in the family**, and
all four are classifiers. Every crate that acts on an error or a policy matches
the variant directly rather than asking the predicate that names the rule. The
predicates are exercised by doc examples and test suites and by nothing else
(→ [`../implementation/002_impl_ring_error.md`](../implementation/002_impl_ring_error.md),
[`../implementation/008_impl_overflow_policy.md`](../implementation/008_impl_overflow_policy.md)).

**The one that got a defence is the exception worth copying.**
`WaitKind::is_non_blocking` has a single production caller, `ring_config`, and
that crate's manual test plan records a check (M4) that the body keeps delegating
to the predicate rather than re-deriving the rule locally. Nothing equivalent
protects the other four.

### Disambiguating `get` and `next`

`.get()` and `.next()` are the two identifiers in this catalog that collide with
methods on unrelated types, so their counts above are not raw greps. Non-doc
`.get()` call sites in `ring_*/src` outside `ring_types` number 40; nine
are `UnsafeCell::get` (`ring_cursor` ×4, `ring_mpsc` ×2, `ring_spsc` ×2,
`ring_event` ×1), leaving 29 for `Capacity::get` and 2 for `SlotIndex::get`:

```sh
cd "$(git rev-parse --show-toplevel)"
command grep -rn '\.get()' ring_*/src | command grep -v ring_types/ | command grep -v ':[0-9]*: *//' \
  | command grep -v 'capacity\.get()\|capacity()\.get()\|index\.get()' | command sed -E 's/^([^:]+):[0-9]+:/\1:/'
```

Live output:

```
ring_cursor/src/lib.rs:    self.0.get().load( order )
ring_cursor/src/lib.rs:    self.0.get().store( value, order );
ring_cursor/src/lib.rs:    self.0.get().fetch_add( n, order )
ring_cursor/src/lib.rs:    self.0.get().compare_exchange( current, new, success, failure )
ring_event/src/lib.rs:    self.get()
ring_mpsc/src/lib.rs:    unsafe { &*self.slots.at( seq ).get() }
ring_mpsc/src/lib.rs:    unsafe { &mut *self.slots.at( seq ).get() }
ring_spsc/src/lib.rs:    unsafe { &*self.slots.at( seq ).get() }
ring_spsc/src/lib.rs:    unsafe { &mut *self.slots.at( seq ).get() }
```

lists exactly those nine. Likewise `.next()` matches `Iterator::next` at
`ring_tls/src/lib.rs:318`, which is not this crate's function.

### All twelve inherent functions take `self` by value

Only `Display::fmt` takes `&self`, and the trait's signature imposes it. Every
type here is `Copy` and at most sixteen bytes, so a reference would cost more than
the value (→ [`../../non_functional_requirement/001`](../../non_functional_requirement/001_errors_and_positions_do_not_allocate.md)).

**Twelve of the thirteen are `const fn`** — the same twelve. So the crate's whole
inherent surface is usable in a `const` context, a capability nothing in the
workspace currently exploits: `Capacity::new`'s one production call site doesn't
pass a literal.
