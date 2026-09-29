# Pitfall: A Wait Strategy It Can Read and Cannot Honour

### Scope

- **Purpose**: Record that `cfg.wait()` compiles here only because the enum lives in `ring_types`, while `ring_wait` — the crate that implements the four strategies — is not in this crate's dependency closure at all.
- **Responsibility**: Name the trap, the failures, and the mitigations.
- **In Scope**: `WaitKind`'s four variants; the gap between reading a discriminant and constructing a waiter.
- **Out of Scope**: What each strategy does (→ [`ring_wait`](../../../ring_wait/readme.md)); the tick-safety question the same field raises (→ [`invariant/002`](../invariant/002_construction_is_the_only_path.md)).

### Trap

**`WaitKind` is in `ring_types` and `ring_wait` is not in this crate's
closure.** Both halves are verifiable:

```sh
cd "$(git rev-parse --show-toplevel)"
command grep 'pub enum WaitKind' ring_types/src/policy.rs
cargo tree -p ring_factory --prefix none --no-dedupe \
  | command grep -c '^ring_wait'
```

Live output:

```
pub enum WaitKind
0
```

So this compiles —

```rust
match cfg.wait()
{
  WaitKind::Spin => …,
  WaitKind::Yield => …,
  WaitKind::Park => …,
  WaitKind::None => …,
}
```

— and there is nothing to put on the right-hand side. The factory can read the
choice, exhaustively match on it, and log it. It cannot build the thing the
choice names.

**This family's own Contract ruling is what makes this possible and it is not a mistake.**
Putting the discriminants in `ring_types` is the same arrangement that lets
`OverflowPolicy` be named in a public signature without exporting
`ring_overflow` — and `OverflowPolicy` works, because `ring_overflow` *is*
reachable here through `ring_core`. The two policy enums are placed
identically and land differently, purely because of what else happens to be in
the graph.

**The trap is that an exhaustive match reads as complete handling.** A `match`
with four arms and no `_` is the strongest completeness signal Rust offers, and
here it can be written over a field the crate has no capacity to act on. The
compiler's approval covers the shape and says nothing about the contents.

**This is the same shape as [`ring_flush`'s `OnBarrier`](../../../ring_flush/docs/pitfall/001_on_barrier_cannot_see_the_barrier.md)**,
which is worth stating because two of the five exported crates hit it
independently: a configuration name whose implementation is one crate further
away than the dependency list reaches. The family's habit of putting
vocabulary in `ring_types` and mechanism in a sibling makes the name travel
further than the capability, and nothing flags where they part company.

### Failure

| # | Failure | How it presents |
|---|---------|-----------------|
| W1 | `build` ignores `cfg.wait()` entirely | Every ring waits the same way. The sweep's wait axis produces identical numbers across four labels and reads as "wait strategy doesn't matter" |
| W2 | `build` treats all four as `Spin` because that is `WaitKind::default()` | Same as W1, and more convincing: the default-value case genuinely is correct, so a spot check passes |
| W3 | `ring_wait` is added as a dependency and the strategy is constructed here | Works, and puts strategy selection in the factory rather than in the backend that waits. The waiter must then be passed down through `ring_core` into `ring_spsc`/`ring_mpsc`, neither of which currently takes one |
| W4 | The backend reads `cfg.wait()` itself | Correct, and means this crate's role in the wait field is to pass the record along untouched — which nothing states, so it looks like an omission |
| W5 | `WaitKind::Park` is built for a ring used inside a tick | A tick-safety violation, at the one point in the family where it could have been refused (→ [`invariant/002`](../invariant/002_construction_is_the_only_path.md)) |

**W1 and W2 are the measurement failures** and W2 is the dangerous one: with
`Spin` as the default, a factory that ignores the field is indistinguishable
from a correct one for every config that did not set it. The first sweep point
that sets `Park` is where it shows, and by then the harness reports a
difference of zero rather than an error.

**W3 versus W4 was the actual open question and W3 lost by having nothing to
build.** Wait strategies are `ring_wait`'s to assign; this crate's own design says
`RingConfig` is "the only constructor input"; nothing says whether the factory
resolves `WaitKind` into a waiter or hands the record down. `ring_wait` shipped
as free functions taking a `WaitKind` per call — **there is no waiter object**,
so "the factory constructs the strategy" would amount to validating a
discriminant and discarding it. The live remainder is narrower than W3-vs-W4:
who carries the kind to the call site. Pending 5 in
[`decisions/`](../decisions/readme.md).

**The current dependency graph still answers by omission and that is still not a
decision** — but the reason is now positive rather than incidental. `ring_wait`
is absent from this crate's manifest because there is nothing to depend on it
*for*, not because nobody added the edge.

**The question got more urgent while this instance was being written, because
`ring_wait` was implemented.** W3 was drafted as a shape someone would have to
build; the strategies now exist and take `WaitKind` directly:

```sh
cd "$(git rev-parse --show-toplevel)"
command grep -E '^\s*pub (fn|const) ' ring_wait/src/lib.rs
for m in ring_*/Cargo.toml; do
  command grep -q '^ring_wait = ' "$m" && basename "$( dirname "$m" )"
done
```

Live output:

```
pub const DEFAULT_SPINS : usize = 1024;
pub const fn escalation_hint( kind : WaitKind ) -> Option< WaitKind >
pub fn pause( kind : WaitKind, attempt : usize ) -> bool
pub fn wait_until< F >( kind : WaitKind, spins : usize, mut ready : F ) -> Result< usize, RingError >
pub fn wait< F >( kind : WaitKind, ready : F ) -> Result< usize, RingError >
pub fn for_space( pair : &CursorPair, kind : WaitKind, spins : usize ) -> Result< usize, RingError >
pub fn for_data( pair : &CursorPair, count : u64, kind : WaitKind, spins : usize )
ring_barrier
ring_shutdown
```

**So the honouring code exists, and the two crates that reached for it are
neither of the ones that would close this gap.** `ring_barrier` and
`ring_shutdown` both consume `WaitKind`; `ring_core`, `ring_spsc` and
`ring_mpsc` — the crates a configured `wait` would have to reach — still do not
declare `ring_wait` at all. **The trap is unchanged and its excuse is gone.**
While `ring_wait` was empty, W1 was indistinguishable from "not yet built"; it
is now distinguishable, and what distinguishes it is a manifest line nobody has
had a reason to add.

### Mitigation

**What does not work:**

| Attempt | Why it fails |
|---------|--------------|
| Depend on `ring_wait` | Available, and it decides W3-versus-W4 by accident rather than by ruling. The edge is the *consequence* of the decision, not a way to make it |
| Match exhaustively and document the arms | The match is the trap. Writing it more carefully does not make the arms do anything |
| Assert in this crate's test that the wait field round-trips | `cfg.wait()` in, `cfg.wait()` out — passes trivially, and is exactly what W1 does |
| Let `ring_core` decide | It has the same problem one level down: `ring_core` does not declare `ring_wait` either |

**What works:**

1. **Rule W3 versus W4 explicitly**, and record it. Either answer is
   defensible; the current state — neither, by omission — is the one that
   produces W1.
2. **The wait axis of the sweep must be asserted behaviourally, not by
   round-trip.** A `Park` ring and a `None` ring differ in whether a consumer
   call returns immediately when the ring is empty. That is observable and it
   is what "observable behaviour matches every field" means for this field
   specifically (→ [`non_functional_requirement/001`](../non_functional_requirement/001_five_fields_asserted_one_at_a_time.md)).
3. **`is_tick_safe()` already exists on `RingConfig`** and nothing calls it.
   Whichever crate resolves the strategy is the natural place for W5's check.

**Mitigation 2 is the one that closes W1 and W2 together**, and it is the only
one of the five fields where a round-trip assertion is clearly insufficient.
Capacity, producers and batch all have direct observable consequences that a
round-trip happens to track; wait does not.

### Algorithms

| File | Relationship |
|------|--------------|
| [../algorithm/002_assembling_a_ring_from_a_validated_record.md](../algorithm/002_assembling_a_ring_from_a_validated_record.md) | The step at which `wait` is consumed, or is not |

### Integrations

| File | Relationship |
|------|--------------|
| [../integration/001_declared_edges_and_the_reached_closure.md](../integration/001_declared_edges_and_the_reached_closure.md) | `ring_wait`'s absence, in the seam table |

### Invariants

| File | Relationship |
|------|--------------|
| [../invariant/002_construction_is_the_only_path.md](../invariant/002_construction_is_the_only_path.md) | W5 — the tick-safety check this crate is positioned to make and does not |

### Non-Functional Requirements

| File | Relationship |
|------|--------------|
| [../non_functional_requirement/001_five_fields_asserted_one_at_a_time.md](../non_functional_requirement/001_five_fields_asserted_one_at_a_time.md) | Mitigation 2 — why this field's assertion cannot be a round-trip |

### Pitfalls

| File | Relationship |
|------|--------------|
| [001_the_criterion_grades_the_clamped_value.md](001_the_criterion_grades_the_clamped_value.md) | The other trap — a field altered before arrival, rather than one that arrives intact and cannot be used |

### Types

| File | Relationship |
|------|--------------|
| [../type/002_build_error.md](../type/002_build_error.md) | W5's refusal, if it is ever made — the error that does not currently exist |

### Sources

| File | Relationship |
|------|--------------|
| [`ring_types/src/policy.rs`](../../../ring_types/src/policy.rs) | `WaitKind`'s four variants and `is_non_blocking` |

### Tests

| File | Relationship |
|------|--------------|
| [`tests/factory_test.rs`](../../tests/factory_test.rs) | ❌ **Mitigation 2's test cannot be written at this crate's grain.** It asked for an assertion that a `None` ring returns immediately where a `Park` ring would not — but nothing in the closure reads `cfg.wait()`, so the two rings are byte-identical and the assertion would compare a value against itself. The suite records the absence instead: `wait` is one of the three fields `only_two_of_five_config_fields_are_observable_through_the_factory` names as unobservable. **A test that cannot fail is worse than a documented gap**, because it reports the field as covered |

### FC43 — The Implementing Crate Is Not in the Closure, Measured Rather Than Asserted

`WaitKind` compiles here because it lives in `ring_types`. The four strategies
live somewhere this crate cannot reach:

```sh
cd "$(git rev-parse --show-toplevel)"
echo '  -- the closure --'
cargo tree -p ring_factory --prefix none 2>/dev/null | sed 's/ v0.*//' | sort -u | tr '\n' ' '
echo
printf '  ring_wait in it: %s\n' \
  "$( cargo tree -p ring_factory --prefix none 2>/dev/null | sed 's/ v0.*//' | sort -u | command grep -c '^ring_wait$' )"
echo '  -- where the enum this crate CAN name lives --'
command grep -r 'pub enum WaitKind' --include=*.rs ring_types/src/ | sed 's|ring/||'
```

Live output:

```
  -- the closure --
ring_align ring_atomic ring_store ring_claim ring_config ring_core ring_cursor ring_factory ring_gating ring_handle ring_index ring_mpsc ring_overflow ring_registry ring_seqno ring_slot ring_spsc ring_stats ring_types 
  ring_wait in it: 0
  -- where the enum this crate CAN name lives --
ring_types/src/policy.rs:pub enum WaitKind
```

Nineteen crates and `ring_wait` is not one of them. So `cfg.wait()` type-checks
and there is no function in scope that could act on its result — the gap is not
that the honouring code is unwritten, it is that the crate holding it is not a
dependency.

That is a sharper statement than "not honoured yet" and it changes what closing
the gap costs: not a function, a manifest edit plus whatever dependency-direction
argument that edit needs. `ring_wait` sits below the ring crates in the family's
layering, so the edge is legal — nothing structural blocks it — and nobody has
had to argue for it because the field's absence produces no error.

### FC44 — The Unhonourable Field Is on the Export Surface and the Crate That Would Honour It Is Not

`WaitKind` is `ring_types`'. `ring_types` is on the five-crate export surface;
`ring_wait` is not:

```sh
cd "$(git rev-parse --show-toplevel)"
echo '  -- the surface --'
command grep -vE '^\s*(#|$)' bench_harness/gate/declared/ring/export_surface.txt | tr '\n' ' '
echo
echo '  -- and the four strategies, in a crate that is not on it --'
command grep -rE '^pub fn |^  [A-Z][A-Za-z]*,' --include=*.rs ring_wait/src/ | sed 's|ring/||' | head -8
```

Live output:

```
  -- the surface --
ring_factory ring_handle ring_tls ring_flush ring_types 
  -- and the four strategies, in a crate that is not on it --
ring_wait/src/lib.rs:pub fn pause( kind : WaitKind, attempt : usize ) -> bool
ring_wait/src/lib.rs:pub fn wait_until< F >( kind : WaitKind, spins : usize, mut ready : F ) -> Result< usize, RingError >
ring_wait/src/lib.rs:pub fn wait< F >( kind : WaitKind, ready : F ) -> Result< usize, RingError >
ring_wait/src/lib.rs:pub fn for_space( pair : &CursorPair, kind : WaitKind, spins : usize ) -> Result< usize, RingError >
ring_wait/src/lib.rs:pub fn for_data( pair : &CursorPair, count : u64, kind : WaitKind, spins : usize )
```

So a consumer outside the family can name a wait strategy, put it in a
`RingConfig`, hand it to `Factory::build`, and receive `Ok` — using only Contract
crates at every step, and reaching nothing that implements the strategy at any
of them.

This is the most consumer-visible form of the gap and the one no test in this
crate can reach, because the consumer whose experience it describes does not
exist yet (→ [`api/001`](../api/001_the_build_surface.md) FC5). The type is
exported, the behaviour is not, and the surface file cannot express the
difference — it lists crates, and the problem is a field of a type inside a
listed one (→ [`invariant/002`](../invariant/002_construction_is_the_only_path.md)
FC24, which is the same coarseness costing something in the other direction).

The cheapest honest mitigation is documentation on `WaitKind` itself, in
`ring_types`, saying which crates honour it. Nothing there says anything of the
kind, and this file — one crate away, in a directory a consumer has no reason to
read — is the only place the gap is written down.

**Disposition:** declined — the finding names its own cheapest mitigation
explicitly: "documentation on `WaitKind` itself, in `ring_types`." That type
and its doc comment live in `ring_types/src/policy.rs`, outside this
crate's assigned scope (`ring_factory`). Nothing in `ring_factory`'s own
source can carry the fix — the gap is that `WaitKind` is silent about which
crates honour it, and `ring_factory` neither defines nor documents
`WaitKind`, it only reads it. Restating the gap again here, in a fourth
place, would not close it; the finding already correctly identifies that
this file is the wrong side of the crate boundary to hold the fix.
