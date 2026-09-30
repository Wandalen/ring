# Non-Functional Requirement: A Backend Swap Is a Build Flag, Not a Rewrite

### Scope

- **Purpose**: Turn this crate's backend-swap condition into something with a number attached, so "swappable" is a measurement rather than a claim.
- **Responsibility**: The requirement, its threshold, the measurement method, the reading taken, and the one part of the requirement that is not yet measured.
- **In Scope**: The cost, to a caller, of moving a program from one backend to another.
- **Out of Scope**: Whether the backends perform equally (they do not — that is `ring_bench`'s concern); why crossbeam was chosen (→ [`workaround/001`](../workaround/001_crossbeam_queue_as_interim_backend.md)).

### The Requirement

This crate's backend-swap condition is that the swap between backends must be
*"a build flag rather than a rewrite"*. Stated that way it is unfalsifiable, so
it is given a threshold:

> **A program using this crate's surface must move between any two backends the
> build offers by changing only its construction site — no change to any push,
> drain, or occupancy call, and no `cfg` in the caller.**

**One exception, stated here rather than left implicit:** the guarantee does
not cover `OverflowPolicy::DropOldest`. A `RingConfig` carrying it builds
under `--features crossbeam` and returns `Err( RingError::PolicyUnsupported )`
without it — a construction-site outcome change, not merely a differing call
(→ CO39 below, [`../lifecycle/001`](../lifecycle/001_construction_and_backend_selection.md)
CO33). Portability under this requirement is conditional on the policy.

### Measurement

The reached-test is the measurement, and its shape is the evidence rather than
its passing:

```
tests/core_test.rs::the_same_program_behaves_identically_on_every_backend
```

One function body, parameterized over `Backend`. Everything after construction
is backend-agnostic: push, drain-on-refusal, occupancy, and the final assertion
are one code path for all three.

| Reading | Value |
|---|---|
| Lines that differ between backends | **1** — the construction call |
| `cfg` attributes in the test body | **0** |
| Backends exercised, `--features crossbeam` | 3 |
| Backends exercised, default build | 2 |
| Records per backend | 200 through 8 slots ≈ 25 laps |

**Threshold met.** The one differing line is the construction site, which the
requirement explicitly permits.

### Why the Guard Test Is Part of the Measurement

`every_backend_the_build_offers_is_actually_exercised` pins the backend count
per configuration. Without it, this requirement could read as met while the
reached-test silently ran one backend — a parameterized test that covers one
case reports green and measures nothing.

So the requirement's evidence is **two** tests: one that runs the same program
everywhere, and one that asserts "everywhere" is not empty.

### What Is Not Measured

**The construction site's own cost.** `new( &config )` and
`new_crossbeam( &config )` have identical signatures, so the swap is a one-word
edit — but *that* is asserted by inspection here, not by a test. Nothing fails
if a future constructor takes different arguments.

This is a real gap and the cheapest fix is a test that constructs every backend
from one `&RingConfig` value and asserts each reports the backend asked for,
which the existing `the_producer_count_selects_between_spsc_and_mpsc` and
`the_crossbeam_backend_ignores_the_producer_count` pair *nearly* does — they
share the config shape but are separate functions, so a signature divergence
would be edited into both rather than caught. Recorded rather than fixed,
because the fix belongs with the `ring_handle` surface reconciliation
(→ [`integration/002`](../integration/002_handle_surface_divergence.md)) where
the signatures are being settled anyway.

### Lifecycles

| File | Relationship |
|------|--------------|
| [../lifecycle/001_construction_and_backend_selection.md](../lifecycle/001_construction_and_backend_selection.md) | The construction site this requirement permits to differ |

### Invariants

| File | Relationship |
|------|--------------|
| [../invariant/002_uniform_delivery_across_backends.md](../invariant/002_uniform_delivery_across_backends.md) | What "behaves identically" means precisely, including the three exclusions |

### Patterns

| File | Relationship |
|------|--------------|
| [../pattern/001_uniform_surface_over_unequal_backends.md](../pattern/001_uniform_surface_over_unequal_backends.md) | The shape that makes the threshold achievable |

### Sources

| File | Relationship |
|------|--------------|
| [`../pattern/001_uniform_surface_over_unequal_backends.md`](../pattern/001_uniform_surface_over_unequal_backends.md) | The composition shape whose "build flag, not a rewrite" promise this instance gives a numeric threshold |

### Tests

| File | Relationship |
|------|--------------|
| `tests/core_test.rs` | `the_same_program_behaves_identically_on_every_backend` — the measurement |
| `tests/core_test.rs` | `every_backend_the_build_offers_is_actually_exercised` — the guard that keeps the measurement non-vacuous |

### CO39 — The Build-Flag Claim Has One Exception, and It Is a Runtime Error

The requirement as stated is that a consumer moves between backends by changing
a feature flag and a producer count, never by editing call sites. That holds for
the surface — every method exists on every backend and means the same thing.

It does not hold for construction. A `RingConfig` carrying
`OverflowPolicy::DropOldest` builds a ring under `--features crossbeam` and
returns `Err( RingError::PolicyUnsupported )` without it
(→ [`../lifecycle/001`](../lifecycle/001_construction_and_backend_selection.md), CO33).

**A build flag that turns a success into a runtime error is a code change in
everything but name** — the caller's error path now runs. The requirement should
either exclude `DropOldest` explicitly or state that portability is conditional
on the policy.

```sh
cd "$(git rev-parse --show-toplevel)"
command grep -m1 -A2 -F 'One exception, stated here rather than left implicit' ring_core/docs/non_functional_requirement/001_backend_swap_is_a_build_flag.md
```

Live output:

```
**One exception, stated here rather than left implicit:** the guarantee does
not cover `OverflowPolicy::DropOldest`. A `RingConfig` carrying it builds
under `--features crossbeam` and returns `Err( RingError::PolicyUnsupported )`
```

**Disposition:** applied — "The Requirement" blockquote is now immediately
followed by an explicit `DropOldest` exception naming the construction-site
outcome change, so the requirement excludes the policy explicitly rather
than leaving the gap implicit. Now prints: `One exception, stated here
rather than left implicit:`

### CO40 — No `#[ inline ]` Anywhere in a Pure Dispatch Layer

```sh
cd "$(git rev-parse --show-toplevel)"/ring_core
printf 'inline attrs: '; grep -c 'inline' src/lib.rs
printf 'pub fns:      '; grep -cE '^    pub (const )?fn ' src/lib.rs
```

Live output:

```
inline attrs: 0
pub fns:      16
```

Zero of sixteen. Whether that costs anything depends on a fact worth stating:
`Ring< T >` is generic, so its methods are instantiated in the calling crate and
are inlining candidates regardless of the attribute. A non-generic dispatch layer
with this shape would need `#[ inline ]` to avoid a real cross-crate call per
operation; this one does not.

Recorded because the reasoning is invisible in the source. A reader adding
`#[ inline ]` "to be safe" would be adding it for a problem genericity already
solves.
