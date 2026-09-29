# Non-Functional Requirement: Construction Cost Is Paid Once and Never on the Path

### Scope

- **Purpose**: State a requirement this crate's own acceptance criteria do not — that a factory may spend freely at build time and must leave nothing that a publish or drain pays for — and give it a measurement it can actually fail.
- **Responsibility**: State the quality attribute, the statement, the measurement method, and the acceptance threshold.
- **In Scope**: What construction may cost; what it may not leave behind; how the difference is observed.
- **Out of Scope**: Per-field fidelity (→ [`non_functional_requirement/001`](001_five_fields_asserted_one_at_a_time.md)); the ring's own steady-state performance, which belongs to the backend crates.

### Quality Attribute

**Performance isolation.** Not speed — the requirement is that a cost incurred
in one phase stay in that phase.

This is the attribute the whole family is organised around, and it is stated
per-crate elsewhere: `ring_tls`'s zero-allocation append path, `ring_flush`'s
policy consulted by value, `ring_spsc`'s no-RMW publish path. **This crate is
the one place where spending is *permitted*,** which makes it the one place
where the boundary can be crossed by accident in the expensive direction.

### Statement

> **Construction may allocate, branch, validate, and consult shared state
> freely. It must not install anything that costs a publish or a drain.**

The asymmetry is the point. `build` runs once per ring; a publish runs millions
of times. A microsecond spent in `build` is free; a nanosecond added to publish
is the measurement.

**This crate's own acceptance criteria do not say this.** They are entirely about field
fidelity, and nothing in them would fail if `build` returned handles that
performed an allocation per publish. The requirement is real regardless — the
family exists to measure write paths — so it is stated here rather than
left implied.

**What "installs" means, concretely:**

| # | Construction-time choice | Per-operation cost it installs |
|---|--------------------------|-------------------------------|
| P1 | Boxing the backend behind `dyn` to unify the two `Ring` types | An indirect call per publish (→ [`algorithm/001`](../algorithm/001_selecting_a_backend_from_one_boolean.md)'s A2) |
| P2 | Storing `RingConfig` on a handle and re-reading it per operation | A load and a branch per publish, for a value that never changes |
| P3 | An `Arc` the handles clone per operation rather than once | An atomic RMW per publish — **the specific thing `ring_spsc`'s no-lock invariant forbids** |
| P4 | A capacity that is not a power of two, forcing a modulo | A division per index. Excluded upstream by `Capacity`'s validation |
| P5 | Attaching statistics counters unconditionally | An RMW per publish (`ring_stats`'s own concern) |

**P1 and P3 are the two this crate can actually cause**, and both are the
natural spelling of a real problem. P1 solves "return one of two types" the
obvious way. P3 arises from a misreading of `Arc` — the refcount touch belongs
at clone and drop, not per use — and
[`ring_handle`'s own instance](../../../ring_handle/docs/data_structure/001_two_handles_over_one_backend.md)
already notes the distinction, in its words: "a handle that cloned itself per
operation would perform an RMW per publish while every signature stayed the
same."

**P4 is included because it is the one already prevented**, and it shows what
prevention looks like: a validated newtype at the entry point, refusing loudly.
Nothing analogous guards P1, P2, P3 or P5.

### Measurement Method

**The requirement is not measured by timing `build`.** Build time is irrelevant
and a benchmark of it would pass under every failure above. What must be
measured is the *publish path of a ring the factory produced*, compared against
the same backend constructed directly.

| # | Method | Detects |
|---|--------|---------|
| M1 | Publish-path benchmark: `factory.build( cfg )` versus `Ring::new( cfg.capacity() )`, same operations, same count | P1, P2, P3 — any per-operation cost the factory added |
| M2 | Assert the handle types are not `dyn`-shaped — a compile-time check that the concrete backend type is visible | P1, before it is measurable |
| M3 | An allocation counter across a publish loop, asserting zero after warm-up | P1 and P5 |
| M4 | Read the generated code for one publish and count the atomic operations | P3, and the only method that distinguishes it from ordinary contention |

**M1 is the primary and it has a trap: it needs a baseline the factory did not
build.** That baseline is `Ring::new`, which is exactly the constructor
[`pattern/002`](../pattern/002_one_way_in.md) wants unreachable. So this
measurement depends on a family-internal construction path continuing to exist —
which it should, for `ring_core` and `ring_testkit`, and which is a reason the
"one way in" boundary is at the export surface rather than at the crate edge.

**M4 is the only method that would catch P3 unambiguously**, because an extra
atomic RMW under contention looks like contention. A benchmark alone reports a
slower number and invites the conclusion that the workload is heavier, not that
the path acquired an instruction.

**The measurement belongs in `ring_bench`, not here.** This crate's own test
suite can carry M2 and M3; M1 and M4 need the harness. Recording that split
matters because a requirement whose measurement lives in another crate is one
nobody runs by default.

### Acceptance Threshold

| # | Threshold | Rationale |
|---|-----------|-----------|
| T1 | A factory-built ring's publish path performs the same number of atomic operations as a directly-constructed one | The strongest statement, and the one P3 violates |
| T2 | A factory-built ring's publish path performs zero allocations after warm-up | P1 and P5 |
| T3 | No handle type returned by `build` is a trait object | P1, checkable at compile time |
| T4 | `build`'s own cost is **unbounded and unmeasured** | Explicitly stated so nobody optimises it |
| T5 | Any future per-operation cost is opt-in via a config field, never a default | P5's shape — statistics are wanted sometimes and must not be free-riding |

**T4 is a threshold in the permissive direction and is worth stating.** Without
it, a well-meaning contributor will make `build` `const`, avoid an allocation,
or cache a backend — each of which trades an irrelevant cost for a risk to T1 or
T2. The requirement's whole content is that the two phases are not comparable.

**T5 is the one that will be tested by `ring_stats`.** It is a declared
dependency of this crate that the build arc never touches
(→ [`lifecycle/001`](../lifecycle/001_from_a_record_to_a_handle_pair.md)'s D5).
When it does, the question is whether counters are always installed or
configured — and `RingConfig` has no field for it, so the answer today would
have to be "always", which T5 forbids.

**T1–T3 were blocked by implementation only, not by any ruling** — this was the
crate's one instance waiting on code rather than on a decision. The code exists
now and they remain unasserted, which is the more interesting outcome: nothing
in this crate's suite measures allocation counts or per-operation cost, because
a correctness suite that also asserts timings is a flaky suite. **Waiting on
code turned out not to be the same as being unblocked by it.** T1 and T4's
measurement belongs to `ring_bench`; T5's to `ring_stats`.

### Algorithms

| File | Relationship |
|------|--------------|
| [../algorithm/001_selecting_a_backend_from_one_boolean.md](../algorithm/001_selecting_a_backend_from_one_boolean.md) | A2 — the trait-object option, rejected on exactly this requirement |
| [../algorithm/002_assembling_a_ring_from_a_validated_record.md](../algorithm/002_assembling_a_ring_from_a_validated_record.md) | The steps permitted to allocate |

### Data Structures

| File | Relationship |
|------|--------------|
| [../data_structure/002_the_handle_pair_as_output.md](../data_structure/002_the_handle_pair_as_output.md) | D1/D2/D3 — the shape choice P3 lives inside |

### Lifecycles

| File | Relationship |
|------|--------------|
| [../lifecycle/001_from_a_record_to_a_handle_pair.md](../lifecycle/001_from_a_record_to_a_handle_pair.md) | D5 — `ring_stats`, and T5's coming test |
| [../lifecycle/002_the_factory_outlives_nothing.md](../lifecycle/002_the_factory_outlives_nothing.md) | Why T4's permissiveness costs nothing — construction happens once and the factory keeps nothing |

### Non-Functional Requirements

| File | Relationship |
|------|--------------|
| [001_five_fields_asserted_one_at_a_time.md](001_five_fields_asserted_one_at_a_time.md) | This crate's own stated requirement; this one is the unstated companion |

### Patterns

| File | Relationship |
|------|--------------|
| [../pattern/002_one_way_in.md](../pattern/002_one_way_in.md) | M1's baseline problem — the measurement needs the constructor the pattern discourages |

### Sources

| File | Relationship |
|------|--------------|
| [`ring_handle/docs/data_structure/001_two_handles_over_one_backend.md`](../../../ring_handle/docs/data_structure/001_two_handles_over_one_backend.md) | P3's distinction, in that crate's own words |
| [`ring_spsc/docs/invariant/002_no_lock_in_the_path.md`](../../../ring_spsc/docs/invariant/002_no_lock_in_the_path.md) | The publish-path invariant P3 would violate |

### Tests

| File | Relationship |
|------|--------------|
| [`tests/factory_test.rs`](../../tests/factory_test.rs) | T2 and T3 are assertable here and **are not asserted** — deliberately, since timing assertions in a correctness suite are flaky and this crate's `build` is a validation plus one allocation whose cost is `ring_core`'s. T1 and T4's measurement belongs to `ring_bench`, stated here rather than left as a silent gap, which is what this row asked for |

### FC35 — The Requirement's Own Measurement Appears Nowhere in the Suite

The requirement was written to be failable. Nothing in the crate attempts it:

```sh
cd "$(git rev-parse --show-toplevel)"
echo '  -- timing and allocation machinery in the tests --'
for k in 'Instant' 'Duration' 'elapsed' 'alloc' 'GlobalAlloc'; do
  printf '  %-12s %s\n' "$k" \
    "$( command grep -c "$k" ring_factory/tests/factory_test.rs )"
done
echo '  -- the one hit, in full --'
command grep 'alloc' ring_factory/tests/factory_test.rs
echo '  -- what the tests do count --'
command grep -E 'AtomicUsize|Ordering::' ring_factory/tests/factory_test.rs | head -4
```

Live output:

```
  -- timing and allocation machinery in the tests --
  Instant      0
  Duration     0
  elapsed      0
  alloc        1
  GlobalAlloc  0
  -- the one hit, in full --
/// allocation hook in this suite to watch its buffer. It is guaranteed
  -- what the tests do count --
use core::sync::atomic::{ AtomicUsize, Ordering };
  static DROPPED : AtomicUsize = AtomicUsize::new( 0 );
      DROPPED.fetch_add( 1, Ordering::Relaxed );
  assert_eq!( DROPPED.load( Ordering::Relaxed ), 0, "nothing should have dropped yet" );
```

No clock, no `Duration`, no allocation hook. The one atomic in the file counts
`Drop` calls for the ownership tests, not costs.

**The single `alloc` hit is the sharp part.** It is not code — it is a doc
comment at line 245 reading *"allocation hook in this suite to watch its
buffer"*, describing machinery the suite does not contain. A reader who greps
for the requirement's enforcement finds one match, and that match is a sentence
asserting the enforcement exists. Prose that names a mechanism is
indistinguishable, to a grep, from the mechanism; the only thing separating them
here is that one of them is on a `///` line.

That is the deliberate choice this instance already records — timing assertions
in a correctness suite are flaky — and the consequence is worth naming as a
measured absence rather than a stated intention: the requirement has a
measurement described in prose and no artefact anywhere that would go red if it
were violated. `ring_bench` is where such an artefact would live, and it grades
throughput rather than construction, so a factory that started leaving work for
the tick path would show up as a slower number in a benchmark nobody would think
to attribute here.

The narrow, cheap check the requirement does have is structural and is in
[`data_structure/002`](../data_structure/002_the_handle_pair_as_output.md) FC11:
`Split< u32 >` and `Ring< u32 >` are the same width, so the factory adds no
indirection for a publish to pay for. That is one of the three sub-claims, it is
free to re-run, and it is the only one currently enforced by anything.

### FC36 — Construction Allocates Once and the Crate Cannot See It

"May spend freely at build time" is satisfied trivially here, because this
crate's own contribution to construction is two statements with no allocation in
them:

```sh
cd "$(git rev-parse --show-toplevel)"
echo '  -- this crate build body --'
sed -n '/pub fn build< S : Send >/,/^  }/p' ring_factory/src/lib.rs
echo '  -- and where the allocation is, doc comments excluded --'
for f in ring_core ring_store; do
  command grep -E 'Vec::|vec!|Box::|with_capacity|ArrayQueue::new' "$f/src/lib.rs" \
    | command grep -vE '^ *(///|//!)' | sed "s|^|$f/src/lib.rs:|"
done
```

Live output:

```
  -- this crate build body --
  pub fn build< S : Send >( &self, cfg : RingConfig ) -> Result< Split< S >, BuildError >
  {
    let ring = Ring::new( &cfg ).map_err( BuildError::Unsupported )?;
    Ok( Split::new( ring ) )
  }
  -- and where the allocation is, doc comments excluded --
ring_core/src/lib.rs:          crossbeam_queue::ArrayQueue::new( config.capacity().get() ),
ring_store/src/lib.rs:    let mut slots = Vec::with_capacity( capacity.get() );
```

The buffer is allocated in `ring_store`, reached through `ring_core`, and this
crate never names either. So the requirement's subject — what construction
costs — is entirely a property of crates downstream, and the factory's
contribution to build cost is a move and an error translation.

That reframes the requirement rather than dissolving it. Its enforceable content
is the *negative* half: not "construction may be expensive", which is somebody
else's licence to spend, but "construction leaves nothing behind that a publish
pays for", which is a claim about the boundary this crate does own — the
returned value. And that half is exactly what FC11's width measurement checks.

The positive half is unowned. No document in the family states a budget for what
a build may cost, and the only crate that could measure one builds rings in a
setup phase it deliberately excludes from its numbers.
