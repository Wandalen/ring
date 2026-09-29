# Pattern: A Uniform Surface Over Unequal Backends

### Scope

- **Purpose**: Name the pattern this crate is an instance of, state the two rules that make it work, and record the failure mode it invites — so that the next crate applying it does not rediscover the same seam.
- **Responsibility**: The pattern's structure, its applicability conditions, the two rules, its known failure mode, and the evidence for each.
- **In Scope**: The composition shape, stated generally enough to reuse.
- **Out of Scope**: This crate's specific dispatch code (→ [`algorithm/001`](../algorithm/001_backend_dispatch_and_the_refusal_seam.md)).

### Problem

Several implementations must be offered under one caller-visible surface, and
they do not agree on how to do the job — not merely in performance, but in method
set. One publishes by reserving a slot and writing into it; another takes the
value by move and cannot give it back. A trait needs a shared method set to
abstract over, and there is none.

The naive responses both fail. Exposing each implementation's own surface makes
the choice of backend a rewrite rather than a build flag. Defining the trait
against the *union* of their methods forces every implementation to stub out
operations it cannot support, which pushes the incompatibility into runtime
panics instead of removing it.

### Solution

A public handle type wrapping a private enum, one variant per implementation,
with everything uniform held *beside* the enum rather than inside a variant.

```
pub struct Handle { inner : Inner, <uniform state> }
enum Inner { A( … ), B( … ), #[cfg(…)] C( … ) }
```

Every public method matches on `inner`, does the per-implementation thing, and
then applies the uniform state once — after the match, not inside each arm.

### Applicability

Four conditions, all of which hold here. The pattern degrades badly when any is
missing, which is why they are worth stating:

| Condition | Here |
|---|---|
| The implementations do **not** share a method set | `ring_spsc`/`ring_mpsc` publish through a slot reservation; `ArrayQueue` has only `push( value )` |
| The choice is made once and never changes | fixed at construction (→ [`lifecycle/001`](../lifecycle/001_construction_and_backend_selection.md)) |
| The count is small and known | three, or two |
| A vtable on the path would be visible | this is a lock-free ring; the dispatch is in the hot loop |

**If the implementations did share a method set, a trait is better** — it costs
nothing at compile time and adding an implementation is a new impl rather than
an exhaustive-match change across every method. That is the pattern's real
price, and at ten implementations it is the wrong trade.

### Rule 1: Uniform State Lives Beside the Enum

`OverflowPolicy` is a field on `Ring` and on `Producer`; it is not a payload of
any variant. So a backend arm never has to know which policy is in force, and
the policy is applied in exactly one place.

The counter-shape — a policy copy in each variant — has two ways to go wrong
that this one does not: the copies can disagree, and a new variant can forget
to apply it. Neither is detectable by the compiler.

**The exception proves the rule and is worth naming.** `DropOldest` at
crossbeam *is* handled inside the arm, because eviction has to happen *instead
of* the push rather than after it. When uniform handling is impossible, the arm
takes it — but that is one deliberate exception, not the default.

### Rule 2: The Uniform Contract Is the Narrow Intersection

The surface promises **exactly-once delivery of the accepted set** and nothing
more. Drain order, `free_capacity` exactness, and `DropOldest` support are all
explicitly excluded, each with an argument
(→ [`invariant/002`](../invariant/002_uniform_delivery_across_backends.md)).

**A uniform surface that promises the union of what its backends do is a lie;
one that promises the intersection is useful but must say so.** The excluded
list is the pattern's most important artifact — it is what lets a reader tell
whether their program is portable across backends before writing it.

### Consequences

**The pattern's characteristic bug is at the arm whose native signature cannot
express the uniform contract.** Every other arm compiles and behaves; that one
compiles and misbehaves.

Here: `ring_mpsc::Producer::push` takes the record **by value**, so on refusal
there is nothing to hand back — while `try_push` promises `Result< (), T >`. The
call site compiles either way. The first implementation panicked, and it shipped.

Two things about how it was found generalize:

- **The doc examples could not have caught it.** All seven build an SPSC ring —
  the natural shape for documentation, and the wrong shape for verification.
  Measured under the restored bug: 4 integration failures, **0 of 7 doc-test
  failures**.
- **The parameterized test is what caught it**, and only because a second test
  guards the parameterization itself from going vacuous
  (`every_backend_the_build_offers_is_actually_exercised`). A parameterized test
  that silently runs one case reports green while proving nothing — which is the
  same failure one level up.

**The generalizable check:** for each arm, ask whether its native signature can
express the uniform contract *without a wrapper that can panic*. Where it
cannot, that is the seam, and it needs a test that reaches that arm
specifically.

### Algorithms

| File | Relationship |
|------|--------------|
| [../algorithm/001_backend_dispatch_and_the_refusal_seam.md](../algorithm/001_backend_dispatch_and_the_refusal_seam.md) | The instance — dispatch, and the seam as it actually occurred |
| [../algorithm/002_uniform_drain_over_three_shapes.md](../algorithm/002_uniform_drain_over_three_shapes.md) | The drain side, where the seam costs a copy instead of correctness |

### Data Structures

| File | Relationship |
|------|--------------|
| [../data_structure/001_three_way_storage_enum.md](../data_structure/001_three_way_storage_enum.md) | Rule 1 as it is actually laid out |

### Invariants

| File | Relationship |
|------|--------------|
| [../invariant/002_uniform_delivery_across_backends.md](../invariant/002_uniform_delivery_across_backends.md) | Rule 2's exclusion list |

### Tests

| File | Relationship |
|------|--------------|
| `tests/core_test.rs` | `the_same_program_behaves_identically_on_every_backend` — the pattern's own reached-test |
| `tests/manual/readme.md` | C1, C2 — the 4-versus-0 measurement, and which arms the doc examples reach |

### CO43 — The Pattern Is Applied Four Times and Named Once

Four independent applications, each with its own mechanism: `new` normalizes
three constructors, `ends`/`split` imposes one backend's two-step shape on all
three (→ CO35), `try_push` normalizes three refusal conventions
(→ [`../algorithm/001`](../algorithm/001_backend_dispatch_and_the_refusal_seam.md), CO2),
`try_recv_batch` normalizes a loop and a bulk pop.

Only the third has a named pattern instance
(→ [`../pattern/002`](../pattern/002_claim_before_move_so_a_refusal_can_hand_the_record_back.md)),
because only it needed a non-obvious technique. The other three are worth
recording as instances of the same general pattern so that a fifth application
is recognized as one rather than invented again.

### CO44 — Uniformity Is Achieved for Operations and Abandoned for Guarantees

`free_capacity` exists on all three backends and means two different things
(→ [`../pitfall/001`](../pitfall/001_free_capacity_carries_two_contracts.md)).
`len` and `is_empty` do the same (→ CO7). The pattern makes the *call* uniform
and leaves the *contract* asymmetric, with nothing in the type system marking
where.

**This is the pattern's known limit and it belongs in the pattern's own
statement.** A reader applying "uniform surface over unequal backends" elsewhere
should know in advance that it converts a signature difference into a
documentation obligation, and that the obligation is easy to under-fill — as
`api/002` shows, where the mirror-image hazard went undocumented for the
consumer side.
