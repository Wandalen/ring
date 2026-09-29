# Invariant: Construction Is the Only Path

### Scope

- **Purpose**: State the restriction this crate exists to impose — that a ring comes into existence only through this crate — and record that it does not currently hold, with four named leaks and one of them already public API.
- **Responsibility**: State the invariant, its enforcement, and what breaks when it does not hold.
- **In Scope**: Every reachable way to obtain a ring; what the export surface confines and what it does not.
- **Out of Scope**: What `build` does once called (→ [`algorithm/002`](../algorithm/002_assembling_a_ring_from_a_validated_record.md)); whether the config it receives is the one the caller wrote (→ [`pitfall/001`](../pitfall/001_the_criterion_grades_the_clamped_value.md)).

### Invariant Statement

**Every ring in the system was produced by `ring_factory`.**

This crate states the intent as a property of `RingConfig` — it "is the only
constructor input" — and the reasoning behind it is:

> Constructors sprout everywhere, one per combination someone needed. The set
> of legal configurations is whatever happens to have been written.

**The value of the invariant is enumerability.** If one function makes rings,
the set of rings that can exist is the set of `RingConfig` values that pass
validation, which is finite, describable, and sweepable. If several functions
make rings, that set is the union of whatever each of them permits, and the
benchmark sweep covers a subset of the real configuration space without any
way to know which subset.

### Enforcement Mechanism

**Three mechanisms are named for this, and all three currently fail. The
invariant does not hold today.**

| # | Mechanism | Status |
|---|-----------|--------|
| M1 | The export surface — only five `ring_*` crates may be named by a consumer outside the family | **Vacuous.** Confines nothing yet |
| M2 | Gate G5, which enforces M1 mechanically | Runs, passes, and has nothing to check |
| M3 | Backend crates not exposing their own constructors | **False.** Four public constructors exist |

**M3 is not a shortfall in a plan; it is a contradiction with shipped code.**
Both backends expose two constructors each:

```sh
cd "$(git rev-parse --show-toplevel)"
command grep 'pub fn new\|pub fn with_config' ring_spsc/src/lib.rs ring_mpsc/src/lib.rs
```

Live output:

```
ring_spsc/src/lib.rs:  pub fn new( capacity : Capacity ) -> Self
ring_spsc/src/lib.rs:  pub fn with_config( config : &RingConfig ) -> Self
ring_mpsc/src/lib.rs:  pub fn new( capacity : Capacity ) -> Self
ring_mpsc/src/lib.rs:  pub fn with_config( config : &RingConfig ) -> Self
```

**`with_config` is the one that matters, because it reads one field of five:**

```sh
cd "$(git rev-parse --show-toplevel)"
for c in ring_spsc ring_mpsc; do
  awk '/pub fn with_config/,/^  }/' $c/src/lib.rs | command grep -o 'config\.[a-z_]*()' | sort -u
done
```

Live output:

```
config.capacity()
config.capacity()
```

Both bodies are `Self::new( config.capacity() )`. A caller who holds a
`RingConfig` and calls `with_config` has passed the whole record and had
`wait`, `overflow`, `producers` and `batch` discarded without a diagnostic —
and `producers` in particular selects the *backend*, which a method on a
specific backend cannot honour even in principle.

**`ring_mpsc`'s own doctest demonstrates the gap and passes:**

```rust
let config = RingConfig::new( 32 ).unwrap().with_producers( 4 );
let ring : Ring< TypedSlot< u8 > > = Ring::with_config( &config );
assert_eq!( ring.capacity(), config.capacity() );
```

`with_producers( 4 )` is set, never read, and never asserted on. The test is
correct about what it claims; the claim is one field wide.

**M1 and M2 are vacuous for a stated reason, not an oversight.** The gate skips
family-internal manifests by design —

```sh
cd "$(git rev-parse --show-toplevel)"
command grep -F 'grep -qx -- "$owner" && continue' bench_harness/gate/g5_export_surface.sh
```

Live output:

```
  printf '%s\n' "${members[@]}" | grep -qx -- "$owner" && continue
```

— and [`bench_harness`'s own invariant](../../../bench_harness/docs/invariant/001_gate_non_vacuity.md)
already records that no crate outside the family depends on any `ring_*` crate,
so the confinement has nothing to confine. That is honest and temporary. What
it means for *this* invariant is that the only enforcement standing between a
consumer and `ring_spsc::Ring::new` is a text file no code reads yet.

### Violation Consequences

| # | Leak | Consequence |
|---|------|-------------|
| L1 | `Ring::new( capacity )` on either backend | A ring with default wait, default overflow, and a batch of one, none of which the caller chose. The sweep cannot describe it because no `RingConfig` corresponds to it |
| L2 | `Ring::with_config( &cfg )` | **A ring that looks configured and is not.** Four fields silently dropped. This is worse than L1 precisely because the call site reads as compliance |
| L3 | An outside consumer names `ring_spsc` directly | Permitted today by G5's vacuity, forbidden by this family's own Contract ruling. The family's contract is wider in practice than in writing |
| L4 | A consumer takes `ring_tls` alone | Gets staging with no ring behind it. Not a ring, so not strictly a violation — recorded because it is the one export that yields a working object without touching this crate |

**L2 is the finding.** L1 is a low-level constructor doing a low-level thing;
anyone calling `Ring::new( Capacity::new( 64 )? )` knows they have bypassed
configuration. L2 accepts the exact type this crate designates as *the*
constructor input and honours a fifth of it. A reviewer scanning for
"is the config being used" sees `with_config` and stops.

**L2's blast radius was bounded, and both of the ways it could grow have since
grown.** This paragraph previously recorded that nothing outside the two
defining crates called `with_config`, and predicted that "the moment
`ring_core` **or a test** reaches for the obvious convenience, the leak
acquires users." That disjunction was written expecting one branch or the
other to fire. Both did. Re-run it:

```sh
cd "$(git rev-parse --show-toplevel)"
command grep -r 'with_config' --include=*.rs | command grep -v '/ring_spsc/\|/ring_mpsc/'
```

Live output:

```
ring_bench/tests/bench_test.rs://! | `with_config` ignores the policy the factory refuses | `the_direct_doors_ignore_the_policy_the_contract_door_refuses` |
ring_bench/tests/bench_test.rs:/// `ring_spsc::Ring::with_config` and `ring_mpsc::Ring::with_config` read one
ring_bench/src/lib.rs:    ring_spsc::Ring::with_config( &workload.config() );
ring_bench/src/lib.rs:    ring_mpsc::Ring::with_config( &workload.config() );
ring_core/src/lib.rs:      true => Storage::Mpsc( ring_mpsc::Ring::with_config( config ) ),
ring_core/src/lib.rs:      false => Storage::Spsc( ring_spsc::Ring::with_config( config ) ),
```

**`ring_core` reached for it first, on both branches**, and `ring_core` is the
crate `ring_factory` delegates construction to. So the leak's earliest caller
was not a consumer bypassing the factory — it was the factory's own dependency,
sitting on the sanctioned path.

**That was better than it looked and worse than it looked, in different ways.**
It was better because `ring_core` compensates for two of the four dropped
fields itself: it reads `config.overflow()` and stores it, and it reads
`config.is_multi_producer()` to pick the branch, so `with_config`'s omissions
are covered before they matter (→ [`algorithm/001`](../algorithm/001_selecting_a_backend_from_one_boolean.md)).
It was worse because the leak acquired a *precedent* inside the family: the
next crate wanting a ring from a config would have a call site to copy, and the
copy would not carry `ring_core`'s compensations with it.

**That next crate arrived, and the uncompensated half of the prediction is the
half that held.** `ring_bench` calls `with_config` from `src/lib.rs`, once per
backend, and compensates for nothing: its only reads of
`overflow()` are a `record_drop` argument and a `writeln!` argument — both
report what the policy *was*, neither enforces it. Call sites outside the two
defining crates now number four across two crates, and only half of them are
covered.

**The copy is deliberate, which the prediction did not anticipate and which
changes what to do about it.** `ring_bench` did not reach for a convenience and
inherit a defect quietly. It reached for both doors on purpose, names this
crate's Pending 8 in the test's own doc comment, and pins the divergence with a
passing assertion:

```sh
cd "$(git rev-parse --show-toplevel)"
sed -n '/^fn the_direct_doors_ignore_the_policy_the_contract_door_refuses/,/^}/p' \
  ring_bench/tests/bench_test.rs | command grep 'assert\|for candidate'
```

Live output:

```
  assert!( run( Candidate::ContractRing, &evicting ).is_err(), "the factory refuses" );
  for candidate in [ Candidate::DirectSpsc, Candidate::DirectMpsc ]
    assert!( outcome.is_lossless(), "{} built and filled a ring anyway", candidate.name() );
```

**So L2 stopped being a described gap and became a measured one.** The identical
`RingConfig` that `ring_factory::build` rejects outright produces a working,
policy-free ring one level down, and a green test says so on every run. The
prediction had the mechanism right and the register wrong: the copy is not
careless, it is evidentiary.

**Two of the five fields are still unread by anyone on this path** — `wait` and
`batch` — which is the same shortfall this instance opened with, relocated one
crate down and now harder to see, because it is split across two files that each
look complete. The per-field census belongs with the arc it measures and lives
in [`algorithm/002`](../algorithm/002_assembling_a_ring_from_a_validated_record.md)
under its Step 4; it is cited here rather than repeated so the two documents
cannot drift apart.

**The resolution is not obviously "delete `with_config`."** It has a defensible
reading: a per-backend constructor that takes the record and uses the part
applicable to it. Under that reading the missing piece is a name that says so
(`with_capacity_from`) or a doc comment that states the four omissions, rather
than removal. **Adoption raises the cost of every option except the doc comment,
and has now raised it twice**: a rename touches four call sites across two
crates rather than two, and a deletion needs both a replacement and a rewrite of
the test written to assert the behaviour being deleted. The one option adoption
made *cheaper* is the doc comment — now the only response that costs nothing,
and also the only one with a passing test already demonstrating what it would
have to say. Recorded in [`decisions/`](../decisions/readme.md).

### Algorithms

| File | Relationship |
|------|--------------|
| [../algorithm/001_selecting_a_backend_from_one_boolean.md](../algorithm/001_selecting_a_backend_from_one_boolean.md) | Why `producers` cannot be honoured by a method on a backend — the field that chooses between them |

### APIs

| File | Relationship |
|------|--------------|
| [../api/001_the_build_surface.md](../api/001_the_build_surface.md) | The path this invariant says is the only one |

### Integrations

| File | Relationship |
|------|--------------|
| [../integration/002_the_crate_the_export_surface_routes_through.md](../integration/002_the_crate_the_export_surface_routes_through.md) | M1 and M2, and what this family's own Contract ruling obliges once G5 stops being vacuous |

### Invariants

| File | Relationship |
|------|--------------|
| [001_configuration_fully_determines_the_ring.md](001_configuration_fully_determines_the_ring.md) | The companion — that one says the config determines the ring, this one says nothing else builds one |

### Patterns

| File | Relationship |
|------|--------------|
| [../pattern/002_one_way_in.md](../pattern/002_one_way_in.md) | The pattern this invariant is the formal statement of, and its consequences |

### Pitfalls

| File | Relationship |
|------|--------------|
| [../pitfall/002_a_wait_strategy_it_can_read_and_cannot_honour.md](../pitfall/002_a_wait_strategy_it_can_read_and_cannot_honour.md) | `wait` is one of the four fields L2 discards, and the one this crate could not honour either |

### Sources

| File | Relationship |
|------|--------------|
| [`bench_harness/gate/declared/ring/export_surface.txt`](../../../bench_harness/gate/declared/ring/export_surface.txt) | M1's five names and the Contract ruling, quoted in its header |
| [`bench_harness/docs/invariant/001_gate_non_vacuity.md`](../../../bench_harness/docs/invariant/001_gate_non_vacuity.md) | M2's vacuity, already recorded at gate grain |
| [`ring_spsc/src/lib.rs`](../../../ring_spsc/src/lib.rs) | L1 and L2's SPSC half |
| [`ring_mpsc/src/lib.rs`](../../../ring_mpsc/src/lib.rs) | L1 and L2's MPSC half, and the doctest that sets `producers` and never reads it |

### Tests

| File | Relationship |
|------|--------------|
| [`tests/factory_test.rs`](../../tests/factory_test.rs) | Correct as written: no test in this file asserts the invariant, because a test cannot prove no other constructor exists. The available check remains the grep in *Enforcement Mechanism*. **This crate added a second construction path of its own** — `build_crossbeam`, ruled and documented rather than accidental (→ [`decisions/002`](../decisions/002_two_doors_not_one_that_routes.md)) — so the grep's expected count changed and the invariant's subject is now "one door per backend family", not "one door" |

### FC23 — Nine Public Ways to Obtain a Ring, and Three of Them Are This Crate's

The invariant's subject is a count, so the count is the measurement:

```sh
cd "$(git rev-parse --show-toplevel)"
command grep -rE '^\s*pub (const )?fn (new|with_config|new_crossbeam|build|build_named|build_crossbeam)\b' \
  --include=*.rs ring_spsc/src/ ring_mpsc/src/ ring_core/src/ ring_factory/src/ \
  | sed 's|ring/||;s/(.*//'
```

Live output:

```
ring_spsc/src/lib.rs:  pub fn new
ring_spsc/src/lib.rs:  pub fn with_config
ring_mpsc/src/lib.rs:  pub fn new
ring_mpsc/src/lib.rs:  pub fn with_config
ring_core/src/lib.rs:  pub fn new
ring_core/src/lib.rs:  pub fn new_crossbeam
ring_factory/src/lib.rs:  pub fn build< S : Send >
ring_factory/src/lib.rs:  pub fn build_named< S : Send >
ring_factory/src/lib.rs:  pub fn build_crossbeam< S : Send >
```

Nine. Six of them are somebody else's, and they are not equivalent doors — the
four in `ring_spsc` and `ring_mpsc` construct a backend directly and read one
config field, `ring_core`'s two construct a dispatching ring and read three, and
this crate's three add the error translation and the registry.

Ranking them by how much of the record they honour gives the real shape of the
leak: a caller reaching `ring_spsc::Ring::with_config` gets a ring built from
`capacity` alone, silently, with four fields discarded and no refusal. That is a
worse outcome than the invariant's phrasing suggests, which reads as though the
alternative doors produced the same ring by a different route.

None of the six can be closed from here. `ring_spsc` and `ring_mpsc` need public
constructors to be testable in isolation, and `ring_core` needs them to be usable
by `ring_tls`. The invariant is therefore a statement about what the export
surface confines, not about what exists — and the export surface confines crates
(→ FC24).

### FC24 — The Surface Confines Crates and Every Leak Is a Type Inside a Confined Crate

`export_surface.txt` lists five crate names. Each of the six alternative
constructors lives in a crate that is *not* on that list, which is what makes
the confinement argument work — and the mechanism is coarser than the claim:

```sh
cd "$(git rev-parse --show-toplevel)"
echo '  -- the surface --'
command grep -vE '^\s*(#|$)' bench_harness/gate/declared/ring/export_surface.txt | tr '\n' ' '
echo
echo '  -- and the crates the six leaks live in --'
command grep -rlE '^\s*pub (const )?fn (new|with_config|new_crossbeam)\b' \
  --include=*.rs ring_spsc/src/ ring_mpsc/src/ ring_core/src/ \
  | sed 's|ring/||;s|/src/.*||' | sort -u | tr '\n' ' '
echo
```

Live output:

```
  -- the surface --
ring_factory ring_handle ring_tls ring_flush ring_types 
  -- and the crates the six leaks live in --
ring_core ring_mpsc ring_spsc 
```

Three crates, none of them on the surface. So the invariant holds for a consumer
outside the family, today, by crate-level exclusion.

The gap is that crate-level exclusion cannot express a *partial* export. The
moment any of those three needs to be nameable for one reason — `ring_core`
would be the candidate, since `ring_tls` and `ring_bench` both build through it
— its every public constructor arrives with it, and the surface file has no way
to admit a crate while withholding a function. Adding a name is one line and
opens four doors.

The same coarseness is already being paid in the other direction:
[`api/002`](../api/002_the_named_build_surface.md) FC7 shows `ring_registry`'s
eight methods reaching an outside consumer through a re-exported type, without
`ring_registry` being on the surface at all. So the file both over-confines
(a crate is all-or-nothing) and under-confines (a re-export smuggles an API
past it), and the invariant is stated against the half that happens to work.
