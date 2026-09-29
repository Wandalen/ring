# Type: Build Error

### Scope

- **Purpose**: Define what a build may refuse, and record that the list is two entries long because most other refusals were made in another crate or never made at all — and that the second entry arrived from a dependency rather than from this crate's own design.
- **Responsibility**: State the definition and the validation rules.
- **In Scope**: The refusals that reach this crate; where the missing ones went; which paths return `Result` and why.
- **Out of Scope**: `RingError`'s own variants, which belong to `ring_types`; the registry's internal errors (→ [`ring_registry`](../../../ring_registry/readme.md)).

### Definition

**Two variants, and only one of them is about a name.**

```rust
pub enum BuildError
{
  /// A ring is already registered under this name.
  NameTaken,
  /// The backend cannot honour the configured overflow policy.
  Unsupported( RingError ),
}
```

**The second variant is new, and it arrived from outside this crate.** The
definition here was a single `NameTaken` until `ring_core` was implemented; the
paragraph below the table records what changed and why the earlier shape was a
reasonable reading of the evidence available at the time.

**The list is short because most refusals happened elsewhere.** Trace each
thing a factory could plausibly reject:

| Candidate refusal | Where it actually happens |
|-------------------|---------------------------|
| Capacity is zero or not a power of two | `RingConfig::new` returns `Err` — verify with `RingConfig::new( 63 ).is_err()` |
| Batch is zero, or larger than capacity | Neither. `with_batch` **clamps** (→ [`pitfall/001`](../pitfall/001_the_criterion_grades_the_clamped_value.md)) |
| Producers is zero | Neither. `with_producers` clamps to 1 |
| Wait strategy is unsupported | Nothing checks. All four variants are accepted and none is honoured (→ [`pitfall/002`](../pitfall/002_a_wait_strategy_it_can_read_and_cannot_honour.md)) |
| Overflow policy is unsupported | **`ring_core::Ring::new`, one crate down.** `DropOldest` is refused; see below |
| Allocation failed | Rust aborts; not an `Err` |
| The name is already taken | **Here.** The only refusal this crate owns outright |

#### The overflow row changed under this instance

When this table was first written it read "All three are supportable; nothing to
refuse", which was true of every line of code then in the repository.
`ring_core` was a skeleton with zero items. It is now 617 lines and refuses one
of the three:

```sh
cd "$(git rev-parse --show-toplevel)"
command grep 'PolicyUnsupported' ring_core/src/lib.rs ring_types/src/error.rs
```

Live output:

```
ring_core/src/lib.rs:  /// [`RingError::PolicyUnsupported`] for `OverflowPolicy::DropOldest`, which neither
ring_core/src/lib.rs:  /// assert_eq!( Ring::< u8 >::new( &evicting ).unwrap_err(), RingError::PolicyUnsupported );
ring_core/src/lib.rs:      return Err( RingError::PolicyUnsupported );
ring_types/src/error.rs:  PolicyUnsupported,
ring_types/src/error.rs:      | Self::PolicyUnsupported => true,
ring_types/src/error.rs:      | Self::PolicyUnsupported => false,
ring_types/src/error.rs:      Self::PolicyUnsupported => write!( f, "this backend cannot honour the configured overflow policy" ),
```

The variant is `ring_types`', not `ring_core`'s — which is what makes it legal
for `BuildError` to carry (→ V3). Line order follows the installed `grep`'s own
output, which groups by file in its own order rather than the argument order.

`ring_core`'s own doc comment gives the reason: `DropOldest` asks the ring to
evict an unread record, "which neither in-house backend can honour — evicting an
unread record contradicts the exactly-once delivery both of them guarantee."
That is a good reason and the refusal is correct.

**What it costs this crate is that the unnamed path can now fail.**
`OverflowPolicy::DropOldest` is a legal variant of a Contract-legal type,
`ring_config` accepts it without complaint, and `RingConfig` offers no way to
ask whether the ring it describes can actually be built:

```sh
cd "$(git rev-parse --show-toplevel)"
echo '  -- ring_config: --'
command grep -c 'DropOldest' ring_config/src/lib.rs
echo '  -- ring_core, for contrast: --'
command grep -c 'DropOldest' ring_core/src/lib.rs
```

Live output:

```
  -- ring_config: --
0
  -- ring_core, for contrast: --
7
```

So a caller can construct a config that passes every validation `ring_config`
performs and hand it to a `build` that has no way to say no. **This is the
crate's one place where a shipped, running refusal has no channel to travel
on**, and it is why the definition above carries a second variant.

**Three shapes were available and this is the third.** `build` could refuse
`DropOldest` itself before calling `ring_core` — but that duplicates a policy
decision belonging to the backend, and a fourth backend would silently make the
duplicate wrong. `RingConfig` could stop expressing `DropOldest` — but the
policy is meaningful for the crossbeam backend, which accepts it. Wrapping
`RingError` keeps the ruling where it is made and lets it travel.
✅ Ruled: [`decisions/002`](../decisions/002_two_doors_not_one_that_routes.md).
`the_policy_refusal_matches_the_backend_that_makes_it` asserts the relay by
comparing the two side by side rather than hard-coding the expected variant, so
a fourth backend changing its mind fails the test rather than silently
diverging.

**`build_named( cfg, name )` can fail for two reasons; `build( cfg )` for one.**
Both therefore return `Result`, which is a change from the shape originally
implied. That original shape spelled it `Factory::build(cfg)` returning "a handle pair",
with no error in the phrasing — written before there was a backend to refuse
anything, and now describing a signature that cannot be written.

**The refusal this crate is positioned to make and does not:**

`RingConfig::is_tick_safe()` exists, is `const`, and every call to it is a test
of itself:

```sh
cd "$(git rev-parse --show-toplevel)"
command grep -r 'is_tick_safe' --include=*.rs
```

Live output:

```
ring_config/tests/config_test.rs:      cfg.with_wait( kind ).is_tick_safe(),
ring_config/tests/config_test.rs:  assert!( cfg.with_wait( WaitKind::None ).is_tick_safe() );
ring_config/tests/config_test.rs:  assert!( !cfg.with_wait( WaitKind::Spin ).is_tick_safe() );
ring_config/src/lib.rs:  /// assert!( !cfg.is_tick_safe() );
ring_config/src/lib.rs:  /// assert!( cfg.with_wait( WaitKind::None ).is_tick_safe() );
ring_config/src/lib.rs:  pub const fn is_tick_safe( &self ) -> bool
```

Six hits, all inside `ring_config`: the definition, its two doctest lines, and
three assertions in its own suite. **No code anywhere decides anything based on
the answer.** It reports `self.wait.is_non_blocking()`, and exactly one of the four wait
kinds qualifies:

```rust
assert_eq!( WaitKind::ALL.iter().filter( | w | w.is_non_blocking() ).count(), 1 );
```

`WaitKind::None` is that one. `WaitKind::Spin` is the `#[ default ]`, so **the
default configuration is not tick-safe**, and `build` — the one point where a
config becomes a ring — accepts it without comment. A `TickUnsafe` variant here
would enforce tick-safety at its natural chokepoint.

**It is deliberately not in the definition above**, because refusing it
unconditionally would break every legitimate off-tick ring, and refusing it
conditionally requires `build` to know whether its result will be used on a tick
— which it cannot. The realistic shape is a separate opt-in constructor, and
that is a ruling, not a detail. Recorded in [`decisions/`](../decisions/readme.md).

### Validation

| Rule | Statement |
|------|-----------|
| V1 | `NameTaken` is returned only by the naming path. `Unsupported` is returned by both, because it originates one crate down and neither path can avoid it |
| V2 | A `NameTaken` build **must not have constructed a ring**, or must have dropped it before returning (→ [`lifecycle/004`](../lifecycle/004_name_state_through_a_registration.md)) |
| V3 | `BuildError` names no type from outside the Contract's five. `NameTaken` carries no payload; `Unsupported` carries `RingError`, which is `ring_types`' and therefore legal. ⚠️ **The caveat came true one crate down** — see below |
| V4 | It wraps `RingError` and does not flatten it. `Unsupported( RingError )` keeps the backend's own vocabulary intact rather than re-spelling `PolicyUnsupported` as a `BuildError` variant of the same name |

**V1 is now asymmetric, and the asymmetry is the design.** A name collision is
this crate's own failure; a policy refusal is a failure it relays. The
unnamed path exists to be the simple one, and it acquires a `Result` not because
this crate found something to check but because the crate it delegates to did.
Relaying is the honest shape: hiding the refusal behind a panic or a silent
policy substitution would make `build` lie about what ring it returned.

**V2 is the one with teeth.** A registry rejection arriving after a successful
construction means either a leaked ring or a silently-discarded one, and the
second is fine while the first is not — the difference is whether the handle
pair was ever exposed. The state machine instance pins the ordering that makes
V2 free rather than checked. **`Unsupported` does not need V2's protection**,
because it is raised before a ring exists — which is worth stating, since the
two variants otherwise look like they need the same guarantee.

**V3 is the Contract rule at type grain.** Every public signature in an exported
crate may only name the five. An error payload is a public signature, and it is
the easiest place to widen the surface accidentally, because the payload is
usually added for diagnostics rather than as an API decision. `Unsupported`'s
payload survives that rule only because `RingError` lives in `ring_types`; had
`ring_core` defined its own error type, this variant could not carry it.

**The caveat V3 ended on — "the rule stops holding the moment somebody adds
`NameTaken { existing : … }`" — happened, in `ring_registry` rather than here:**

```sh
cd "$(git rev-parse --show-toplevel)"
command grep 'NameTaken' ring_registry/src/lib.rs
```

Live output:

```
  NameTaken
      Self::NameTaken { name } => write!( f, "a ring is already registered as {name:?}" ),
  /// [`RegistryError::NameTaken`] if the name is already live, paired with the
        Err( ( RegistryError::NameTaken { name }, ring ) )
```

`RegistryError::NameTaken` is a *struct variant carrying the name*. This crate
receives it and **discards the payload**, returning the unit `BuildError::NameTaken`:

```rust
Err( ( RegistryError::NameTaken { .. }, _refused ) ) => Err( BuildError::NameTaken ),
```

Two reasons, and the Contract one is the weaker of them. `String` is not from
outside the five — it is `core`/`alloc` — so V3 as literally written does not
forbid it. The reason that actually decides it is that `BuildError` is `Copy`,
and a `String` payload would end that. The caller also passed the name in and
still holds it, so relaying it back adds nothing.

**The finding is that V3 anticipated the right event and mispredicted which rule
would stop it.** It expected a Contract violation and got a `Copy` bound. Both
forbid the payload, so the outcome is the same and the reasoning is not: a rule
whose stated justification is not the one doing the work will be relaxed the
moment somebody checks the stated one and finds it satisfied.

**V4 replaces the rule that stood here before, which said the opposite.** The
earlier V4 read: "It does not wrap `RingError`. Nothing that reaches this crate
can produce one, and a `From` impl would suggest otherwise." That was true of
the code as it stood and is now false — `ring_core::Ring::new` produces exactly
one, from a config this crate will hand it. **It is retained in this form rather
than deleted because the reasoning that produced it was sound and the conclusion
still failed**: it inferred an impossibility from an absence of callers in a
family that was thirty-one skeletons deep. An error type nothing produces yet is
not an error type nothing will produce.

### Algorithms

| File | Relationship |
|------|--------------|
| [../algorithm/002_assembling_a_ring_from_a_validated_record.md](../algorithm/002_assembling_a_ring_from_a_validated_record.md) | F1 and F2 — the two candidate failures, of which one is this type |

### APIs

| File | Relationship |
|------|--------------|
| [../api/001_the_build_surface.md](../api/001_the_build_surface.md) | V1's other half — the path with no `Result` |
| [../api/002_the_named_build_surface.md](../api/002_the_named_build_surface.md) | The path that returns this |

### Invariants

| File | Relationship |
|------|--------------|
| [../invariant/002_construction_is_the_only_path.md](../invariant/002_construction_is_the_only_path.md) | V3's Contract rule, and the four leaks that already widen the surface |

### Pitfalls

| File | Relationship |
|------|--------------|
| [../pitfall/001_the_criterion_grades_the_clamped_value.md](../pitfall/001_the_criterion_grades_the_clamped_value.md) | Why two candidate refusals became clamps instead |
| [../pitfall/002_a_wait_strategy_it_can_read_and_cannot_honour.md](../pitfall/002_a_wait_strategy_it_can_read_and_cannot_honour.md) | The `TickUnsafe` variant's subject, and why it is unresolved |

### State Machines

| File | Relationship |
|------|--------------|
| [../lifecycle/004_name_state_through_a_registration.md](../lifecycle/004_name_state_through_a_registration.md) | V2's ordering |

### Types

| File | Relationship |
|------|--------------|
| [001_factory.md](001_factory.md) | The type whose operations produce this |

### Sources

| File | Relationship |
|------|--------------|
| [`ring_config/src/lib.rs`](../../../ring_config/src/lib.rs) | `is_tick_safe`, and the clamping setters that removed three candidate refusals |
| [`ring_core/src/lib.rs`](../../../ring_core/src/lib.rs) | `Ring::new`'s `DropOldest` refusal — `Unsupported`'s origin, and the reason the unnamed path returns `Result` |
| [`ring_types/src/error.rs`](../../../ring_types/src/error.rs) | `RingError::PolicyUnsupported`, the payload V3 permits |
| [`ring_types/src/policy.rs`](../../../ring_types/src/policy.rs) | `is_non_blocking` and its doctest — one of four; `OverflowPolicy`'s three variants |

### Tests

| File | Relationship |
|------|--------------|
| [`tests/factory_test.rs`](../../tests/factory_test.rs) | ⚠️ Written as prescribed and **one clause turned out untestable**. `a_refusal_drops_nothing_that_was_already_registered` covers the half of V2 that can fail — the registered ring survives untouched. The other half ("the refused build's own ring is destroyed") is unobservable: that ring is empty by construction, so no record-drop counter sees it die. It holds structurally instead — `_refused` is an ordinary binding with no `mem::forget` on the path. The test was originally named for the clause it cannot check (→ `tests/manual/readme.md` F1). `both_paths_relay_the_same_refusal` covers V1's asymmetry from both paths; `the_unnamed_path_can_never_return_name_taken` covers V1's first clause by exhaustive match rather than by example, which is why `BuildError` is deliberately **not** `#[ non_exhaustive ]` (→ [`api/001`](../api/001_the_build_surface.md)'s guarantee 1) |

### FC47 — `Copy` Is the Family Convention, Not This Type's Choice, and Nothing Records It

`BuildError` cannot carry the colliding name because it is `Copy` and a `String`
is not (→ [`workaround/001`](../workaround/001_returning_ownership_through_the_error.md)
FC50). That reads as a local decision. It is a convention seven types deep:

```sh
cd "$(git rev-parse --show-toplevel)"
for p in ring_bench/src/lib.rs:WorkloadError ring_bench/src/lib.rs:RunError \
         ring_debug/src/lib.rs:Violation ring_factory/src/lib.rs:BuildError \
         ring_flush/src/lib.rs:ConfigError ring_registry/src/lib.rs:RegistryError \
         ring_testkit/src/lib.rs:Anomaly ring_types/src/error.rs:RingError; do
  f=${p%:*}; t=${p#*:}
  printf '  %-15s %s\n' "$t" \
    "$( command grep -B4 "^pub enum $t" "$f" \
          | command grep -E '^ *#\[ *(derive|non_exhaustive)' | tr '\n' ' ' )"
done
```

Live output:

```
  WorkloadError   #[ derive( Debug, Clone, Copy, PartialEq, Eq ) ] 
  RunError        #[ derive( Debug, Clone, Copy, PartialEq, Eq ) ] 
  Violation       #[ derive( Debug, Clone, Copy, PartialEq, Eq ) ] 
  BuildError      #[ derive( Debug, Clone, Copy, PartialEq, Eq ) ] 
  ConfigError     #[ derive( Debug, Clone, Copy, PartialEq, Eq ) ] 
  RegistryError   #[ derive( Debug, Clone, PartialEq, Eq ) ] 
  Anomaly         #[ non_exhaustive ] #[ derive( Debug, Clone, Copy, PartialEq, Eq ) ] 
  RingError       #[ derive( Debug, Clone, Copy, PartialEq, Eq, Hash ) ] #[ non_exhaustive ] 
```

Seven of the eight derive `Copy`. The one that does not is `RegistryError`, and
it is the one holding a `String` — so the family's rule is not "errors are
`Copy`" but "errors are `Copy` unless they carry an owned payload", and
`RegistryError` is the single exception that proves it by being the single type
with something to carry.

That reframes the `NameTaken` decision. It is not this crate weighing a `String`
against a derive; it is this crate staying on the side of a seven-to-one
convention while the crate below it took the other side for the same field. The
disagreement four source lines apart is real (FC50), and neither end knows it is
a disagreement, because the convention lives in eight derive lines and no
document.

### FC48 — The Enum and Its Payload Make Opposite Exhaustiveness Choices

`BuildError` is deliberately not `#[ non_exhaustive ]` — an addition should break
every caller's match, on purpose. `RingError`, which it wraps, is:

```sh
cd "$(git rev-parse --show-toplevel)"
echo '  -- the wrapper --'
command grep -B3 '^pub enum BuildError' ring_factory/src/lib.rs | command grep -E 'derive|non_exhaustive|pub enum'
echo '  -- the payload --'
command grep -B4 '^pub enum RingError' ring_types/src/error.rs | command grep -E 'derive|non_exhaustive|pub enum'
echo '  -- every non_exhaustive attribute in the family --'
command grep -r '^#\[ non_exhaustive \]' --include=*.rs ring_*/src/ | sed 's|ring/||'
```

Live output:

```
  -- the wrapper --
#[ derive( Debug, Clone, Copy, PartialEq, Eq ) ]
pub enum BuildError
  -- the payload --
/// Pitfall: a derive on a wrapper is a constraint on the wrapped type's future,
#[ derive( Debug, Clone, Copy, PartialEq, Eq, Hash ) ]
#[ non_exhaustive ]
pub enum RingError
  -- every non_exhaustive attribute in the family --
ring_testkit/src/lib.rs:#[ non_exhaustive ]
ring_types/src/error.rs:#[ non_exhaustive ]
```

Two `#[ non_exhaustive ]` attributes in thirty-three crates, and one of them is
on the type this crate's own error carries. The other is
`ring_testkit::Anomaly`, in a crate `ring_factory` does not depend on and whose
enum nothing here wraps — so the asymmetry below is this pair's alone.

So a consumer matching a `BuildError` exhaustively writes one arm per variant and
gets a compile error when a variant is added — the behaviour this crate wants.
Matching the `RingError` inside it, they are forced to write a wildcard and get
silence when a variant is added — the behaviour `ring_types` wants. Both are
defensible; they are adjacent in a single `match`, and neither type says why it
differs from the one beside it.

The asymmetry has a direction worth naming. `BuildError`'s variants are this
crate's to add, so breaking callers is a cost this crate pays deliberately.
`RingError`'s are added by whichever backend needs a new refusal, and a
`ring_spsc` change should not break a consumer's match arms — so the wildcard is
protecting a boundary two crates away. Read that way the two choices are the same
rule applied at different distances, which is the sentence that belongs on both
types and is on neither.

**Correction (2026-09-20):** the sentence under the measurement read "One
`#[ non_exhaustive ]` in thirty-three crates, and it is on the type this crate's
own error carries". It was false, and the recipe directly above it had already
said so twice over: its own label reads `-- every non_exhaustive attribute in
the family --` and the Live output beneath that label lists **two** files,
`ring_testkit/src/lib.rs` and `ring_types/src/error.rs`. Re-running the block
today reproduces it byte-identical, so the measurement was correct and current
throughout — `ring_testkit::Anomaly` acquired the attribute after the prose was
written, and nothing ever read the sentence against the output one line above
it. What survives is all of FC48: `BuildError` is still deliberately exhaustive,
`RingError` inside it is still not, the two are still adjacent in a single
`match`, and neither type still says why it differs from the one beside it.
Exclusivity was never the inconsistency — the wrapper/payload disagreement is,
and `Anomaly` is not on either side of it.
