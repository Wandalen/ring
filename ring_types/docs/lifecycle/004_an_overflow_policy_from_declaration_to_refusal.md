# Lifecycle: An Overflow Policy From Declaration to Refusal

### Scope

- **Purpose**: Model an `OverflowPolicy` value across the six states it occupies between being declared here and being counted in `ring_stats`, and locate the two places where the machine as built differs from the machine as designed.
- **Responsibility**: State the states, transitions, and behavioral invariants.
- **In Scope**: The three variants' divergent paths; the backend that refuses one of them; the handler function nothing calls; the counter route that bypasses it.
- **Out of Scope**: The ownership split that puts the handlers elsewhere (→ [`pattern/001`](../pattern/001_discriminants_here_handlers_elsewhere.md)); the closure of the variant set (→ [`non_functional_requirement/002`](../non_functional_requirement/002_the_enum_sets_are_closed_and_asserted.md)).

### States

| # | State | Owner | Representation |
|---|-------|-------|----------------|
| P0 | **Declared** | **This crate** | One of three variants of `OverflowPolicy` |
| P1 | **Configured** | `ring_config` | A field in a `RingConfig`, set by `with_overflow` |
| P2 | **Admitted** | `ring_core` | A built `Ring` carries it in `self.overflow` |
| P3 | **Refused** | `ring_core` | `Err( RingError::PolicyUnsupported )` — no ring exists |
| P4 | **Applied** | `ring_core` / `ring_overflow` | A `Resolution` produced for a full ring |
| P5 | **Counted** | `ring_stats` | One of three atomic counters incremented |

**Only P0 is this crate's.** The remaining five are the arc a discriminant
travels once other crates take responsibility for acting on it, and modelling
them from here is the only way to see that one of the three variants does not
complete the arc.

**Per-variant reachability is the machine's central fact:**

| Variant | P1 | P2 (in-house) | P2 (crossbeam) | P3 | P4 |
|---------|:--:|:-------------:|:--------------:|:--:|:--:|
| `DropNewest` | ✅ | ✅ | ✅ | — | `DroppedIncoming` |
| `Fail` | ✅ | ✅ | ✅ | — | `Refused` |
| `DropOldest` | ✅ | ❌ | ✅ | ✅ | `EvictedOldest` |

**`DropOldest` is configurable, refused, and supported — in that order, by three
different crates.** `ring_config::with_overflow` stores it without comment;
`ring_core::Ring::new` rejects it; `ring_core::Ring::new_crossbeam` accepts it,
behind a cargo feature.

### Transitions

| # | From → To | Trigger | Guard |
|---|-----------|---------|-------|
| W1 | P0 → P1 | `RingConfig::with_overflow( p )` | none — **every policy is accepted** |
| W2 | P1 → P3 | `Ring::new( &config )` | `config.overflow() == DropOldest` |
| W3 | P1 → P2 | `Ring::new( &config )` | otherwise |
| W4 | P1 → P2 | `Ring::new_crossbeam( &config )` | `#[ cfg( feature = "crossbeam" ) ]`, no policy guard |
| W5 | P2 → P4 | A `push` onto a full ring | — |
| W6 | P4 → P5 | `RingStats::record_drop( p, n )` | — |

**W1 has no guard, and that is the design decision the machine turns on:**

```rust
pub const fn with_overflow( mut self, overflow : OverflowPolicy ) -> Self
{
  self.overflow = overflow;
  self
}
```

A `RingConfig` accepts all three policies unconditionally. The refusal is
deferred to W2, where the backend is known — which is correct, because
whether `DropOldest` is supportable is a property of the backend, not of the
configuration. A config is not yet a ring.

**W2 is the refusal, and its rationale is written at the refusal site:**

> [`RingError::PolicyUnsupported`] for `OverflowPolicy::DropOldest`, which
> neither in-house backend can honour — evicting an unread record contradicts
> the exactly-once delivery both of them guarantee. Rejecting it here, at
> construction, is deliberate: the alternative is a `try_push` that silently
> behaves as `DropNewest` and a caller who never learns the policy was not
> applied.

**So P3 is not a failure state — it is the machine's most deliberate
transition.** The alternative design has no P3 at all and reaches P4 with the
wrong `Resolution`, which is exactly the silent-drift failure the whole family's
documentation is written against.

**W5 is where the two divergences live.** `ring_core::Producer::push` consults
the policy through `ring_overflow::would_resolve`:

```rust
// ring_core/src/lib.rs:411-415
Err( record ) => match would_resolve( self.overflow )
{
  Resolution::DroppedIncoming => Ok( () ),
  Resolution::EvictedOldest | Resolution::Refused => Err( record ),
},
```

**Correction (2026-09-28):** this section previously read "A three-way mapping
collapsed into two by a wildcard," quoting a `_ => Err( record )` arm and
warning that "a fourth variant would join the `_` arm silently." The wildcard
was removed under `ring_core`'s CO1 (see
[`ring_trace/pattern/002`](../../../ring_trace/docs/pattern/002_exhaustive_match_as_a_tripwire.md)
§ TR40) — the match above now names `EvictedOldest` and `Refused` explicitly
rather than folding them into a wildcard. **A three-way mapping named in full,
with `EvictedOldest` and `Refused` sharing one arm by explicit pattern.** Both
still hand the record back, and the reachability argument is unchanged —
`EvictedOldest` cannot arrive here because the crossbeam arm handled
`DropOldest` with `force_push` before reaching this match, and the in-house
arms cannot hold that policy at all — but the concern this finding raised is
now moot: a fourth `Resolution` variant would fail to compile here instead of
joining the arm silently, since the match is exhaustive over named variants
rather than defended by a wildcard.

### Behavioral Invariants

| # | Invariant | Holds because |
|---|-----------|---------------|
| C1 | Every variant reaches P1 | W1 has no guard |
| C2 | `DropOldest` reaches P2 only via W4 | W2's guard is exact and W4 has no policy guard |
| C3 | A ring in P2 never changes policy | `self.overflow` is set once at construction and never written |
| C4 | `would_resolve` is total over all three variants | Wildcard-free `match`, `const fn`, no `Result` |
| C5 | A refusal at W2 costs nothing | `PolicyUnsupported` is a `Copy` discriminant; no ring was allocated |
| C6 | P5 is reachable without passing through the designed W6 | See below |

**C4 is what makes `would_resolve` describe an unreachable state, and this is
not a defect.** It maps `DropOldest → EvictedOldest` for every caller, including
one holding an in-house ring that can never be in that configuration. The
function is documented as the pure half — *"for callers deciding what a policy
*would* do — a factory validating a configuration, a test tabulating the
mapping"* — so describing the full mapping rather than the reachable subset is
its job. A caller wanting the reachable subset must also know which backend it
has, and nothing in the type system carries that.

**C6 is the divergence worth recording.** The designed route to P5 is
`ring_overflow::resolve`, which records the drop and returns the resolution in
one call:

```rust
pub fn resolve( policy : OverflowPolicy, stats : &RingStats ) -> Result< Resolution, RingError >
{
  stats.record_drop( policy, 1 );
  match policy { /* three arms */ }
}
```

Nothing calls it:

```sh
cd "$(git rev-parse --show-toplevel)"
for f in resolve would_resolve; do
  printf '%-14s external src: %s   own crate: %s\n' "$f" \
    "$( command grep -rnE "\b$f\(" ring_*/src | command grep -vE '^ring_overflow/src' | wc -l )" \
    "$( command grep -rnE "\b$f\(" ring_overflow/ --include=*.rs | wc -l )"
done
```

Live output:

```
resolve        external src: 0   own crate: 15
would_resolve  external src: 1   own crate: 14
```

**The pure half has one caller and the recording half has none.** `ring_core`
took `would_resolve` and left `resolve` behind, so no drop is counted at the
moment it happens. P5 is nonetheless reached — by `ring_bench`, after the fact,
from the difference between what was offered and what arrived:

```rust
// ring_bench/src/lib.rs:961
stats.record_drop( workload.config().overflow(), ( offered - received ) as u64 );
```

**So the counters are populated by the measuring layer rather than the dropping
layer.** For this crate's measurement purpose the totals come out the same, and the
aggregate route is arguably better — it counts drops the ring never saw as
drops. But it means `ring_stats`'s per-policy counters are only ever written
with the *configured* policy of a whole run, never with the policy of an
individual full-ring event, and `ring_overflow::resolve` exists as an
unexercised second implementation of the same accounting.

**C3 is worth stating because it is what makes the machine per-ring rather than
per-push.** `self.overflow` is copied out of the config at construction and
never written again, so every push in a ring's lifetime consults the same value.
The policy is a property of the ring, not of the operation — which is why W5 can
be modelled as one transition rather than a decision repeated per call.

### APIs

| File | Relationship |
|------|--------------|
| [../api/002_the_five_classifier_predicates.md](../api/002_the_five_classifier_predicates.md) | Q5 — `drops_silently` describing a policy one backend refuses |

### Integrations

| File | Relationship |
|------|--------------|
| [../integration/001_the_crate_thirty_one_of_thirty_three_depend_on.md](../integration/001_the_crate_thirty_one_of_thirty_three_depend_on.md) | The ten crates that name `OverflowPolicy` |

### Items

| File | Relationship |
|------|--------------|
| [../item/enum/003_overflow_policy.md](../item/enum/003_overflow_policy.md) | P0 — the declaration, and the `Default` that makes `DropNewest` the unconfigured answer |
| [../item/associated_constant/003_overflow_policy_all.md](../item/associated_constant/003_overflow_policy_all.md) | The array `ring_stats` sums over, and what C6's route depends on |
| [../item/associated_function/012_overflow_policy_reports_failure.md](../item/associated_function/012_overflow_policy_reports_failure.md) | A predicate over P0 that no state in this machine consults |

### Non-Functional Requirements

| File | Relationship |
|------|--------------|
| [../non_functional_requirement/002_the_enum_sets_are_closed_and_asserted.md](../non_functional_requirement/002_the_enum_sets_are_closed_and_asserted.md) | T6 — the missing `contains` check on the array C6's route sums over |

### Patterns

| File | Relationship |
|------|--------------|
| [../pattern/001_discriminants_here_handlers_elsewhere.md](../pattern/001_discriminants_here_handlers_elsewhere.md) | Why P1–P5 belong to other crates, and the third dispatcher the ruling does not name |

### State Machines

| File | Relationship |
|------|--------------|
| [003_a_capacity_request_through_validation.md](003_a_capacity_request_through_validation.md) | The crate's other machine — one whose every state this crate owns |

### Types

| File | Relationship |
|------|--------------|
| [../type/002_ring_error.md](../type/002_ring_error.md) | `PolicyUnsupported`, P3's payload |

### Sources

| File | Relationship |
|------|--------------|
| [`src/policy.rs`](../../src/policy.rs) | Lines 95–114 — P0, the three variants and their `Default` |
| [`ring_config/src/lib.rs`](../../../ring_config/src/lib.rs) | Line 104 — W1, the unguarded setter |
| [`ring_core/src/lib.rs`](../../../ring_core/src/lib.rs) | Line 33's backend table; 152–154 W2; 182–183 W4's cfg gate; 379 and 391 W5 |
| [`ring_overflow/src/lib.rs`](../../../ring_overflow/src/lib.rs) | Lines 106–115 the uncalled `resolve`; 131–139 the `would_resolve` that was taken instead |
| [`ring_bench/src/lib.rs`](../../../ring_bench/src/lib.rs) | Line 648 — C6's actual route to P5 |

### Tests

| File | Relationship |
|------|--------------|
| [`tests/types_test.rs`](../../tests/types_test.rs) | ✅ Covers P0 completely — `overflow_policy_has_no_overwrite_variant`, `overflow_policies_partition_by_reporting`, `overflow_policy_defaults_to_drop_newest`. ❌ **Nothing here reaches P1 or beyond**, correctly: this crate must not depend on the crates that own those states |
| [`ring_core/tests/core_test.rs`](../../../ring_core/tests/core_test.rs) | Where W2's refusal is asserted — the transition this crate declares the error for but cannot exercise |
| [`ring_overflow/tests/overflow_test.rs`](../../../ring_overflow/tests/overflow_test.rs) | ⚠️ Thirteen references to `resolve`/`would_resolve` in a crate whose recording half has **zero production callers** — the suite covers a function the family does not use, and passing tells nobody that |

### TY42 — `PolicyUnsupported` Has One Construction Site in the Family

```sh
cd "$(git rev-parse --show-toplevel)"
grep -r 'RingError::PolicyUnsupported' ring_*/src/*.rs | grep -v '^ring_types/' \
  | grep -vE ': *(//|///|//!)'
```

Live output:

```
ring_core/src/lib.rs:      return Err( RingError::PolicyUnsupported );
```

One line. The six-state arc this instance describes narrows, at the refusal step,
to a single `if` in a single backend selector — so every claim the corpus makes
about policy refusal is a claim about that one line.

### TY43 — Both Policy Enums Are Matched Exhaustively by Their Handlers

```sh
cd "$(git rev-parse --show-toplevel)"
printf 'WaitKind variants at an arm in ring_wait:            '
grep -oE '^ +WaitKind::[A-Za-z]+' ring_wait/src/lib.rs | sed 's/.*:://' | sort -u | tr '\n' ' '; echo
printf 'OverflowPolicy variants at an arm in ring_overflow:  '
grep -oE '^ +OverflowPolicy::[A-Za-z]+' ring_overflow/src/lib.rs | sed 's/.*:://' | sort -u | tr '\n' ' '; echo
printf 'wildcard arms in either:                             '
grep -hcE '^ +_ =>' ring_wait/src/lib.rs ring_overflow/src/lib.rs | paste -sd+ | bc
```

Live output:

```
WaitKind variants at an arm in ring_wait:            None Park Spin Yield 
OverflowPolicy variants at an arm in ring_overflow:  DropNewest DropOldest Fail 
wildcard arms in either:                             0
```

This is the good half of the `#[ non_exhaustive ]` asymmetry
(→ [`../decisions/001`](../decisions/001_one_error_type_is_a_rule_the_family_does_not_keep.md), TY5).
Because neither policy enum is marked, the handler crates are forced to be
exhaustive, and the split-discriminants-from-handlers pattern gets a compiler
backstop the error type does not have.

Neither handler names the other's enum — the two are genuinely independent axes,
not one policy read twice.
