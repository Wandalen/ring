# Workaround: crossbeam-queue as the Interim Backend

### Scope

- **Purpose**: Record the external dependency this crate absorbs so that consumers are not blocked on the in-house rings, the cost that absorption imposes, and the condition under which it can be deleted.
- **Responsibility**: What is absorbed, why absorbing it here rather than in a consumer is the cheaper site, what it costs in surface and in build configuration, and the deletion condition.
- **In Scope**: `crossbeam-queue`'s `ArrayQueue`, the `crossbeam` cargo feature, and the constraints its shape places on this crate's surface.
- **Out of Scope**: Whether the in-house rings beat it (that is `ring_bench`'s benchmark question); the in-house rings' own designs (→ [`ring_spsc`](../../../ring_spsc/docs/readme.md), [`ring_mpsc`](../../../ring_mpsc/docs/readme.md)).

### Constraint

The constraint this workaround exists to satisfy is a **scheduling** one, not
a technical one. Both in-house rings work. What they do not have is the
operating history that would let a consumer bet a shipping channel on them,
and acquiring that history is `ring_bench`'s own subject matter — so the
consumer waits on the benchmark, and the benchmark is not the consumer's
problem.

`crossbeam-queue` discharges it because it already has the history. It serves
as an *"optional wrap"* standing in as the *"MVP backend"*, with the swap
between backends being a build flag rather than a rewrite.

### Workaround

A consumer could depend on `crossbeam-queue` directly and switch later. That is
the alternative this crate exists to avoid, and the difference is where the
switch happens:

| | Consumer depends on crossbeam directly | Consumer depends on `ring_core` |
|---|---|---|
| Cost of switching later | A rewrite at every call site | A cargo feature |
| Who discovers the shape mismatch | Each consumer, separately, at switch time | This crate, once, now |
| Where the surface differences are documented | Nowhere | [`pitfall/001`](../pitfall/001_free_capacity_carries_two_contracts.md), and the module docs' difference table |
| What the benchmark compares | Two different call-site shapes | One surface over three backends |

The last row is the one that decides it. `ring_bench`'s whole output is a
**comparison**, and a comparison needs the compared things to be reachable
through one surface — otherwise the measurement includes the call sites' own
differences.

### Cost

Absorbing it is not free, and three of the costs are structural rather than
incidental.

1. **The surface had to become value-shaped.** `ArrayQueue` offers
   `push( value ) -> Result< (), value >` and `pop() -> Option< value >` and
   nothing else — no reservation, no in-place write, no publish guard. Both
   in-house rings publish through a *slot*. So a slot-shaped uniform surface
   cannot exist across all three, and the in-place reservation API stops at
   this crate's boundary
   (→ [`pattern/001`](../pattern/001_uniform_surface_over_unequal_backends.md)).
   A caller who needs to build a record in the ring's own memory reaches past
   `ring_core` and gives up the swap.

2. **This became the family's only crate with a cargo feature**, and therefore
   the only one that is two programs. The gates were single-configuration until
   it arrived, which cost a real coverage misreading before it was found
   (→ [`pitfall/002`](../pitfall/002_feature_gated_code_reads_as_uncovered.md)).

3. **One capability had to be admitted that the in-house rings refuse.**
   `ArrayQueue::force_push` evicts the oldest unread record to make room, which
   is exactly `OverflowPolicy::DropOldest`. Both in-house rings guarantee
   exactly-once delivery, so they cannot honour it, and `Ring::new` rejects it
   with `RingError::PolicyUnsupported` at construction while
   `Ring::new_crossbeam` accepts it. **The overflow policy is thus the one
   place where the backends are not interchangeable at the level of what a
   caller may ask for**, and it is a construction-time error rather than a
   silent degradation precisely because a caller who never learns their policy
   was ignored is worse off than one whose build failed.

Two costs it does *not* impose, both worth stating because they were live
concerns:

- **No transitive `unsafe` obligation.** Gate G6 requires every crate opting
  out of the workspace `unsafe` deny to be declared and justified. This crate
  contains no `unsafe`; `ArrayQueue`'s is its own, upstream, and reviewed by a
  population this project is not part of — which is the entire point of using
  it rather than writing it.
- **No atomic added at this layer.** `crossbeam-queue` is declared
  `default-features = false, features = [ "alloc" ]`, so it brings no runtime
  and no global state (→ [`invariant/001`](../invariant/001_no_atomic_of_its_own.md)).

### Removal

**Delete the `crossbeam` feature and this workaround when the in-house rings
have the operating history the consumers were waiting for.** Concretely, all
three must hold:

1. `ring_bench` has published a verdict on the in-house rings against this
   backend.
2. No consumer's build enables the `crossbeam` feature.
3. `OverflowPolicy::DropOldest` has no user, or has been re-specified as
   something the in-house rings can honour.

**Condition 3 is the one that can outlive the other two.** If a consumer
genuinely needs eviction, deleting this backend deletes the only implementation
of it in the family, and the deletion becomes a feature removal rather than a
cleanup. That is worth knowing before the removal is scheduled rather than
during it.

Note what is *not* a deletion condition: the in-house rings being faster.
This workaround's stated purpose is decoupling schedules, not performance, and
a slower in-house ring that consumers trust discharges it just as well as a
faster one.

### Invariants

| File | Relationship |
|------|--------------|
| [../invariant/001_no_atomic_of_its_own.md](../invariant/001_no_atomic_of_its_own.md) | Why `default-features = false` is load-bearing rather than tidy |
| [../invariant/002_uniform_delivery_across_backends.md](../invariant/002_uniform_delivery_across_backends.md) | What "the same program on every backend" is asserted to mean, and the three behaviours excluded from it |

### Patterns

| File | Relationship |
|------|--------------|
| [../pattern/001_uniform_surface_over_unequal_backends.md](../pattern/001_uniform_surface_over_unequal_backends.md) | The value-shaped surface cost 1 forces |

### Pitfalls

| File | Relationship |
|------|--------------|
| [../pitfall/002_feature_gated_code_reads_as_uncovered.md](../pitfall/002_feature_gated_code_reads_as_uncovered.md) | Cost 2, measured |

### Types

| File | Relationship |
|------|--------------|
| [../type/001_backend.md](../type/001_backend.md) | How a caller recovers which of the three it was given |

### Sources

| File | Relationship |
|------|--------------|
| [`../non_functional_requirement/001_backend_swap_is_a_build_flag.md`](../non_functional_requirement/001_backend_swap_is_a_build_flag.md) | Gives the "build flag rather than a rewrite" condition this workaround exists under a numeric threshold |

### Tests

| File | Relationship |
|------|--------------|
| `tests/core_test.rs` | `crossbeam_honours_drop_oldest_by_evicting` — cost 3's positive half. Asserting the policy is *accepted* would not distinguish it from a constructor that took the policy and behaved as `DropNewest`; the delivered contents are what separate them |
| `tests/core_test.rs` | `drop_oldest_is_rejected_by_the_in_house_backends` — cost 3's negative half, at construction rather than at push |
| `tests/manual/readme.md` | C3 — that both feature configurations are built by something, which no test can check |
