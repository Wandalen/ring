# API: The Policy Surface

### Scope

- **Purpose**: Fix what a consumer configures — the small surface by which a policy is chosen and bound to a buffer — and account for what being on the export Contract costs a surface this narrow.
- **Responsibility**: State the operations, the error behaviour, and the compatibility guarantees.
- **In Scope**: Policy construction, binding, and inspection.
- **Out of Scope**: The call that runs a flush (→ [The Driver Surface](002_the_driver_surface.md)); the policy's semantics (→ [Flush Policy](../type/001_flush_policy.md)).

### Abstract

**This surface is deliberately smaller than it could be**, because the crate's
value is that a policy is chosen once and then not touched. A rich policy API
would invite exactly the per-call-site decision-making
[`pattern/001`](../pattern/001_policy_as_a_value.md) exists to remove.

`ring_flush` is one of the family's five exported crates, so everything here is
a public commitment reaching every consumer
(→ [A Decision on the Export Surface](../integration/002_a_decision_on_the_export_surface.md)).

### Operations

| # | Operation | Signature shape | Guarantee |
|---|-----------|-----------------|-----------|
| A1 | Construct a policy | `FlushPolicy::OnFull` / `OnBarrier` / `OnBatch( n )` | Always succeeds; validation is at binding, not construction ([`type/001`](../type/001_flush_policy.md)'s N2) |
| A2 | Bind a policy to a buffer | `Flusher::new( buffer, producer, policy ) -> Result< Flusher, ConfigError >` | **This is where validation happens.** N1 and N2 are checked against the buffer's real capacity |
| A3 | Inspect the bound policy | `fn policy( &self ) -> FlushPolicy` | `Copy` return; no interior state exposed |
| A4 | Inspect accumulated count | `fn staged( &self ) -> usize` | Advisory — see below |

**A2 takes a third argument this instance did not anticipate.** The destination
producer is bound at the same moment as the policy, not supplied per flush. It
has to be: a `Flusher` that borrowed a producer per call would let two flushers
publish the same buffer to different rings, which is
[the publication-point invariant](../invariant/002_publication_point_is_designed_not_inherited.md)'s
C3 with an extra step. Binding all three together makes the destination as
fixed as the policy.

**A2 is the whole surface, really.** A1 is enum construction; A3 and A4 are
inspection. The binding is the only operation that can fail and the only one
that makes a decision.

**One operation is missing from this table entirely, and without it the
surface does not work: `append`.** A buffer moved into `Flusher::new` is owned
by the flusher, so the caller has no way to reach it — staging a record has to
go through the driver. `fn append( &mut self, record : T ) -> Result< (), RingError >`
is the crate's hot path and the one operation on it that
[`nfr/002`](../non_functional_requirement/002_the_decision_costs_nothing_on_the_append_path.md)
constrains. It is absent here because this instance was written before ownership
was settled, when the buffer was assumed to stay with the caller. Its surface
belongs to [`api/002`](002_the_driver_surface.md), which is where the driver's
operations are specified.

**A4 is advisory and must be documented as such.** Between the read and any
action taken on it, an append on the owning thread can change the count. It is
useful for diagnostics and for a consumer deciding whether a drive call is
worthwhile; it is not a basis for a correctness decision. This is the same
one-sided-staleness shape [`ring_spsc`'s free-capacity type](../../../ring_spsc/docs/type/002_free_capacity.md)
documents, with the opposite direction of safety: there, staleness is
conservative and therefore safe; here it is neither, because a count can move
in only one direction between flushes but the flush itself resets it.

### Absences

| Absent operation | Why |
|------------------|-----|
| `set_policy( policy )` | A policy that changes mid-life makes the publication point a function of *when* it changed — [the publication-point invariant](../invariant/002_publication_point_is_designed_not_inherited.md)'s C3 in slow motion |
| `flush_now()` | An unconditional flush is a call-site decision with no policy involved; it is the exact capability [`pattern/001`](../pattern/001_policy_as_a_value.md) removes on purpose |
| `FlushPolicy::default()` | No defensible default (→ [`type/001`](../type/001_flush_policy.md)'s trait table) |
| A registry of buffers | `ring_registry`'s, and the ownership question is open (→ [`ring_tls`'s registration state](../../../ring_tls/docs/lifecycle/004_registration_state.md)) |
| Any per-append hook | Would put a call on the path `ring_tls` constrains to zero atomics and zero allocations |

**`flush_now()` is the absence that will be requested,** and the request will
be legitimate: a consumer shutting down needs the buffer emptied regardless of
policy. That case is real and is handled as a distinct, named lifecycle
operation rather than as a general escape hatch
(→ [From Configuration to the Final Drain](../lifecycle/002_from_configuration_to_the_final_drain.md)) —
the difference being that a final drain is a phase, whereas `flush_now()` is a
capability any code path could reach for.

### Error Handling

| Condition | Result | Rationale |
|-----------|--------|-----------|
| `OnBatch( 0 )` at A2 | `Err( ConfigError::ZeroBatch )` | Would fire every append — a fourth policy |
| `OnBatch( n )` with `n` exceeding buffer capacity | `Err( ConfigError::BatchExceedsCapacity )` | Could never fire; would degrade to `OnFull` |
| ~~Binding a second policy to a bound buffer~~ | ~~`Err( ConfigError::AlreadyBound )`~~ | **Variant not built** — see below |
| `OnBarrier` bound to a driver that never announces | **No error possible** | Not statically checkable; the failure is observable only through [`FlushOutcome::NotTriggered`](../type/002_flush_outcome.md) |

**`ConfigError::AlreadyBound` does not exist, and the check it would have
performed is unreachable.** `Flusher::new` takes the buffer **by value**. A
second binding would need to name the first buffer, and the first buffer was
moved — so the program does not compile, and there is no runtime state in which
`new` could observe a double binding to report it.

This is [`type/001`](../type/001_flush_policy.md)'s N3 enforced by ownership
rather than by validation, which is strictly better: a compile error instead of
an `Err` the caller might ignore, at no runtime cost. **The lesson is not that
the instance was wrong to specify the variant** — it was specified before the
ownership question was settled, and specifying a runtime check for a rule you
have not yet found a static enforcement for is the right default. It is worth
recording only because the enum is now two variants and every doc that said
three has to say two.

**Every remaining error is a configuration error surfaced at binding time**,
which is the design intent: an invalid configuration should fail before any
record is appended, not degrade into a different working policy after a million
of them.

**The last row is the honest gap.** The most likely misconfiguration in this
crate — an `OnBarrier` policy nobody announces to — is the one error this
surface cannot return. Its detection is a runtime observation, not a
construction-time check (→ [the pitfall](../pitfall/001_on_barrier_cannot_see_the_barrier.md)'s F1).

### Compatibility Guarantees

1. **The three variants are stable.** Adding a fourth is a breaking change for
   every consumer matching exhaustively — and exhaustive matching is a property
   [`pattern/001`](../pattern/001_policy_as_a_value.md) deliberately buys, so
   `#[non_exhaustive]` would trade away the thing the enum was chosen for.
2. **`FlushPolicy` stays `Copy` and stays 16 bytes** (→ [The Policy Enum](../data_structure/001_the_policy_enum.md)). A variant carrying a non-`Copy` payload breaks the append path silently.
3. ~~No public type here names a crate outside the Contract's five.~~ **False as
   stated — see finding `FL5` below.** `ConfigError` is this crate's own, not
   `ring_tls`'s, but A2's third parameter is `ring_core::Producer`, and
   `ring_core` is not one of the Contract's five — the leak
   [`ring_handle` documents](../../../ring_handle/docs/integration/002_on_the_export_surface.md) as invisible to gate G5.
4. **A2's fallibility is permanent.** Making binding infallible later would mean either dropping validation or panicking, both worse.

**Guarantee 1 is the expensive one and it is worth stating plainly.** A fourth
policy — `OnInterval`, `OnIdle`, `OnPressure` — is a breaking change to five
crates' worth of consumers. That is the price of exhaustiveness, and it was
paid deliberately.

### Data Structures

| File | Relationship |
|------|--------------|
| [../data_structure/001_the_policy_enum.md](../data_structure/001_the_policy_enum.md) | Guarantee 2's structural basis |

### APIs

| File | Relationship |
|------|--------------|
| [002_the_driver_surface.md](002_the_driver_surface.md) | The other half — this configures, that runs |

### Integrations

| File | Relationship |
|------|--------------|
| [../integration/002_a_decision_on_the_export_surface.md](../integration/002_a_decision_on_the_export_surface.md) | Why guarantee 1 costs what it does |

### Lifecycles

| File | Relationship |
|------|--------------|
| [../lifecycle/002_from_configuration_to_the_final_drain.md](../lifecycle/002_from_configuration_to_the_final_drain.md) | Where `flush_now()`'s legitimate use case is served instead |

### Patterns

| File | Relationship |
|------|--------------|
| [../pattern/001_policy_as_a_value.md](../pattern/001_policy_as_a_value.md) | Why the absences are absences |

### Types

| File | Relationship |
|------|--------------|
| [../type/001_flush_policy.md](../type/001_flush_policy.md) | The configured value; its N1–N3 are A2's error cases |

### Sources

| File | Relationship |
|------|--------------|
| [`../type/001_flush_policy.md`](../type/001_flush_policy.md) | The three variants guarantee 1 fixes |

### Tests

| File | Relationship |
|------|--------------|
| `tests/flush_test.rs` | A2's **two** error cases, not three — `an_unusable_batch_size_is_refused_at_binding` covers `ZeroBatch` and `BatchExceedsCapacity`. The third, `AlreadyBound`, was not built and has no test because it has no reachable state: `Flusher::new` takes the buffer **by value**, so a second binding cannot name the first buffer. Ownership does what the check would have done, at compile time |

### FL5 — Compatibility Guarantee 3 Is False, and the Crate That Hit It Filed the Breach in Its Own Manifest

Guarantee 3 checked the error type and not the constructor:

```sh
cd "$(git rev-parse --show-toplevel)"
echo '  -- the declared export Contract --'
command grep -v '^#' bench_harness/gate/declared/ring/export_surface.txt | command grep -v '^$' | sed 's/^/    /'
echo '  -- what this crate imports --'
command grep -E '^use ring_' ring_flush/src/lib.rs
echo '  -- and the public parameter that names one of them --'
command grep -E 'producer : Producer' ring_flush/src/lib.rs
echo '  -- what the one outside consumer had to write because of it --'
command grep 'ring_core  — ' ring_bench/Cargo.toml
```

Live output:

```
  -- the declared export Contract --
    ring_factory
    ring_handle
    ring_tls
    ring_flush
    ring_types
  -- what this crate imports --
use ring_core::Producer;
use ring_tls::TlsBuffer;
use ring_types::RingError;
  -- and the public parameter that names one of them --
  producer : Producer< 'a, T >,
    producer : Producer< 'a, T >,
  -- what the one outside consumer had to write because of it --
#   ring_core  — `ring_flush::Flusher::new` takes a `ring_core::Producer` and
```

`Flusher::new`'s third parameter is `ring_core::Producer< 'a, T >`. `ring_core`
is not on the Contract, and the declared file's own header says the five names
are "the only ring_* crates a consumer outside `ring_*` may name as a
dependency."

**So the guarantee is false in exactly the way it cites `ring_handle` as
documenting.** It reasoned about `ConfigError` — this crate's own type, not
`ring_tls`'s — concluded correctly about it, and did not look at the argument
list. `TlsBuffer` and `RingError` are both Contract types; the producer is the
one parameter that is not, and it is the one no consumer can avoid, because
every `Flusher` needs one.

**There is no Contract-only route to the value either.** `ring_handle::Producer`
is a distinct wrapper struct holding a `ring_core::Producer` in a private field,
so `ring_handle::split()` yields a type `Flusher::new` will not accept.

The confirmation is not this instance's own reading. `ring_bench` — the first
crate to consume this one from outside — carries the breach in a manifest
comment, in the crate's own words: `ring_flush::Flusher::new` takes a
`ring_core::Producer` and "re-exports neither it nor a way to build one, so the
staging candidate cannot be constructed without this line," concluding **"it is
a Contract defect, not a preference."** Two crates found the same thing
independently; this surface's own guarantee still says it cannot happen.

```sh
# -mN and no -n: this file is its own subject, so an unbounded
# match also finds this command line and every copy of its own
# output below, and -n re-prefixes a fresh line number onto each
# earlier pass's output -- the stacked addresses this block carried
cd "$(git rev-parse --show-toplevel)"
command grep -m1 'False as' ring_flush/docs/api/001_the_policy_surface.md
```

Live output:

```
3. ~~No public type here names a crate outside the Contract's five.~~ **False as
```

**Disposition:** applied — Guarantee 3's own bullet now strikes the false claim
and states the actual defect (A2's producer parameter is `ring_core::Producer`,
not a Contract type) inline, citing this finding instead of contradicting it.
The API leak itself is not fixed here — `ring_handle::Producer` cannot
substitute since it wraps `ring_core::Producer` in a private field, so closing
the leak needs either a `ring_handle` accessor or a `Flusher::new` signature
change, both breaking-change decisions for the whole Contract, out of a
docs-corpus pass's authority. Now prints: `False as`

### FL6 — The Same Question Is Ruled Closed Here and Recorded Open Two Directories Away

Guarantee 1 settles `#[non_exhaustive]`; the decisions register does not:

```sh
# -mN and no -n: this file is its own subject, so an unbounded
# match also finds this command line and every copy of its own
# output below, and -n re-prefixes a fresh line number onto each
# earlier pass's output -- the stacked addresses this block carried
cd "$(git rev-parse --show-toplevel)"
echo '  -- the compatibility guarantee, as stated --'
command grep -m1 '^1\. \*\*The three variants are stable' ring_flush/docs/api/001_the_policy_surface.md
echo '  -- and the pending decision covering the same question --'
command grep -E '^\| P3 \|' ring_flush/docs/decisions/readme.md
echo '  -- and what the crate source says about the same attribute --'
command grep 'non_exhaustive' ring_flush/src/lib.rs | sed -E 's/^(.{0,120}).*/\1/'
printf '    mentions in src/lib.rs: %s\n' \
  "$( command grep -c 'non_exhaustive' ring_flush/src/lib.rs )"
```

Live output:

```
  -- the compatibility guarantee, as stated --
1. **The three variants are stable.** Adding a fourth is a breaking change for
  -- and the pending decision covering the same question --
| P3 | `#[non_exhaustive]` on `FlushPolicy` | **Open**, unchanged. Still a family-grain question |
  -- and what the crate source says about the same attribute --
    mentions in src/lib.rs: 0
```

Guarantee 1 does not merely lean against the attribute; it gives a reason that
would close the question — exhaustive matching is what the enum was chosen for,
so `#[non_exhaustive]` trades away the point. That is a ruling, written in the
document a consumer reads to learn what is stable. Two directories away, P3
records the same question as Open and unchanged, "still a family-grain
question."

**Both are defensible and they cannot both be current.** If Guarantee 1 holds,
P3 is answered and should say so. If P3 is genuinely open, Guarantee 1 is
promising a consumer a stability property the crate has not decided to keep.

The asymmetry in consequence is what makes this worth recording rather than
tidying: a reader who finds Guarantee 1 stops looking. A compatibility-guarantee
list is a terminal document — nobody reads it and then checks whether a pending
decision contradicts it — so an open question restated as a guarantee is
functionally closed, by whoever wrote the guarantee, without the family-grain
ruling P3 exists to require.

`decisions/001` is the resolution: it argues the same conclusion Guarantee 1
asserts, with a measurement, and still declines to rule because the sibling enum
carries five times the cost. **That is the reasoning Guarantee 1 skipped**, and
the guarantee should cite it rather than pre-empt it.
