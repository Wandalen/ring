# API: The Surface No Crate Has Taken

### Scope

- **Purpose**: Account for the surface from the outside — what a consumer must import to call it, what it re-exports to spare them, and which crates have actually taken the edge.
- **Responsibility**: The composed import set, the re-export decision, the adoption count, and the one measurement another crate routed here and never received.
- **In Scope**: `ring_testkit`'s public surface as a dependency; the manifests and test files that name it.
- **Out of Scope**: What the signatures do (→ [`001`](001_the_script_surface.md)); which edges *this* crate declares (→ [`integration/001`](../integration/001_the_three_edges_and_the_one_that_is_missing.md)).

### The composed call

`Script::run` takes a `&mut Ring< u32 >` the caller has already built. That is a
deliberate choice — [`decisions/readme.md`](../decisions/readme.md) records it as
the reason `ring_config` is a dev-dependency rather than a dependency — and its
consequence is that the smallest useful call names four crates, three of them
besides this one:

| # | Crate | Why the consumer needs it | Re-exported here |
|---|---|---|---|
| 1 | `ring_testkit` | `Script`, `Step`, `Outcome` | — |
| 2 | `ring_core` | `Ring`, to have something to run against | no |
| 3 | `ring_config` | `RingConfig`, the only route to a `Ring` | no |
| 4 | `ring_types` | `OverflowPolicy`, if the run varies the policy | no |

Row 4 is conditional and rows 2–3 are not. The crate declares **zero**
`pub use` re-exports, so every one of those names is imported by the consumer
from the crate it is defined in.

**That is the right call and it is worth saying why.** A testkit re-exporting
`Ring` would give a consumer two paths to one type and a version skew waiting to
happen; a testkit re-exporting `RingConfig` would make itself the documentation
surface for a builder it does not own. The cost is that the crate's own manifest
has to explain, in two comments, why the two crates its tests cannot compile
without are not dependencies — which it does.

### Operations

| # | Signature | Guarantees | Reachable without a `Ring` |
|---|---|---|---|
| A1 | `Script::new( usize ) -> Self` | An empty script with a staging bound | yes |
| A2 | `Script::then( self, Step ) -> Self` | Appends one step; consuming builder | yes |
| A3 | `Script::run( &self, &mut Ring< u32 > ) -> Outcome` | Drives the ring; leaves it used | no |
| A4 | `Outcome::vanished( &self ) -> usize` | Accepted minus delivered minus held | yes |
| A5 | `Outcome::audit( &self ) -> Result< (), Anomaly >` | Placement, then over-delivery, then provenance, then order | yes |
| A6 | `audit_received( &[ u32 ], u32 ) -> Result< (), Anomaly >` | Provenance and order, without an `Outcome` | yes |
| A7 | `leak< T >( Ring< T > ) -> &'static mut Ring< T >` | A `'static` ring; never freed | takes one |
| A8 | `leak_ends< T >( Ring< T > ) -> ( Producer, Consumer )` | A `'static` pair; never freed | takes one |
| A9 | `audit_received_unordered( &[ u32 ], u32 ) -> Result< (), Anomaly >` | Provenance and no duplicate, order left free | yes |

Six of the nine are callable with nothing but this crate in scope. `A6` is the
one that matters for adoption: it is the assertion a concurrent test wants, it
needs no ring, no config and no script, and it is a free function taking a slice.

### Adoption

| Measure | Count |
|---|---|
| Manifests in `ring/` declaring `ring_testkit` as a dependency | 0 |
| Other crates' `.rs` files naming `ring_testkit` | 20 |
| `use ring_testkit` statements outside this crate | 0 |
| `#[ test ]` functions exercising the surface, all inside this crate | 33 |

### Regenerate

```sh
cd "$(git rev-parse --show-toplevel)"
printf 'manifests declaring the edge:  %s\n' "$( command grep -rl 'ring_testkit' --include='Cargo.toml' . | command grep -v 'ring_testkit/Cargo.toml' | command grep -cv '^\./Cargo\.toml$' || true )"
printf 'other crates naming it in rs:  %s\n' "$( command grep -rl 'ring_testkit' --include='*.rs' . | command grep -cv '^\./ring_testkit/' || true )"
printf 'use statements outside it:     %s\n' "$( command grep -rn 'use ring_testkit' --include='*.rs' . | command grep -cv '^\./ring_testkit/' || true )"
printf 'pub use re-exports declared:   %s\n' "$( command grep -c '^pub use' ring_testkit/src/lib.rs || true )"
printf 'global allocators, family-wide: %s\n' "$( command grep -rl 'global_allocator' ring_*/src ring_*/tests 2>/dev/null | wc -l )"
```

Live output:

```
manifests declaring the edge:  0
other crates naming it in rs:  20
use statements outside it:     0
pub use re-exports declared:   0
global allocators, family-wide: 4
```

### Integrations

| File | Relationship |
|------|--------------|
| [../integration/001_the_three_edges_and_the_one_that_is_missing.md](../integration/001_the_three_edges_and_the_one_that_is_missing.md) | The edges this crate declares, from the other side |

### APIs

| File | Relationship |
|------|--------------|
| [001_the_script_surface.md](001_the_script_surface.md) | What each signature above does |

### Non-Functional Requirements

| File | Relationship |
|------|--------------|
| [../non_functional_requirement/002_what_a_fixture_owes_its_consumers.md](../non_functional_requirement/002_what_a_fixture_owes_its_consumers.md) | The stability the absent consumers are not yet holding it to |

### Sources

| File | Relationship |
|------|--------------|
| [`Cargo.toml`](../../Cargo.toml) | The two comments explaining the dev-dependencies |
| [`src/lib.rs`](../../src/lib.rs) | The surface, and the absent `pub use` |

### Tests

| File | Relationship |
|------|--------------|
| `tests/testkit_test.rs` | The 33 tests that are the surface's only callers |

### TK7 — a fixture crate with no consumer

Not one manifest in `ring/` declares `ring_testkit`. Eighteen other crates name
it across twenty files — twenty-three mentions — and every one is prose: there
are zero `use ring_testkit` statements outside this crate, so nothing in the
family can call `Script`, `audit_received` or `leak`.

The mentions are not mistakes. Nineteen of the twenty-three are one recurring comment
listing the four crates that hold loom models, and this crate does hold one, so
the comment is accurate. What they establish is that the family knows the crate
exists and has, so far, had no reason to take the edge.

That leaves the surface measured only by its own 33 tests. Every guarantee in the
Operations table above is a guarantee to a caller that does not exist yet, and
the cheapest one to adopt — `audit_received`, a free function over a slice that
needs no ring, no config and no script — is also the one no `cfg( loom )` model
in `ring_spsc`, `ring_mpsc` or `ring_publish` currently calls.

### TK8 — the allocation measurement routed here and never built

`ring_flush/tests/append_cost_test.rs` declines to discharge its own C2
allocation constraint and routes it: *"The real measurement belongs in
`ring_testkit` (feature 188), which is the one place a counting allocator could
be justified once for every crate that needs one."*

There is no counting allocator here. There is no `#[ global_allocator ]` in this
crate — the recording above counts four in the family, and not one of them is
here or shared.

So a constraint recorded as *deferred to a named home* has two separate reasons
it will not arrive there: the home never built the mechanism, and this crate has
no consumer whose allocations it could count on their behalf. The routing reads
as a plan and is currently a redirection to an empty room.

The routing's own premise has since been overtaken. It argued for one counting
allocator "justified once for every crate that needs one"; four crates have each
built their own instead, in four separate `tests/allocation_test.rs` files. The
duplication the consolidation was meant to prevent has already happened, and it
happened without this crate being asked.
